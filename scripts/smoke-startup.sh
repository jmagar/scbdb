#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_NAME="$(basename "$0")"
readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DEFAULT_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-90}"
readonly DEFAULT_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"

log() {
  printf '[%s] %s\n' "$SCRIPT_NAME" "$*"
}

err() {
  printf '[%s] error: %s\n' "$SCRIPT_NAME" "$*" >&2
}

server_pid=""
cleanup() {
  if [[ -n "$server_pid" ]]; then
    log "Stopping API server (pid=$server_pid)..."
    kill "$server_pid" >/dev/null 2>&1 || true
    wait "$server_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

wait_for_postgres() {
  local attempts=$((DEFAULT_TIMEOUT_SECONDS / DEFAULT_POLL_INTERVAL_SECONDS))
  local postgres_user="${POSTGRES_USER:-scbdb}"

  log "Waiting for postgres container readiness..."
  for ((i = 1; i <= attempts; i += 1)); do
    if docker exec scbdb-postgres pg_isready -U "$postgres_user" >/dev/null 2>&1; then
      log "Postgres is ready."
      return 0
    fi
    sleep "$DEFAULT_POLL_INTERVAL_SECONDS"
  done

  err "postgres did not become ready within ${DEFAULT_TIMEOUT_SECONDS}s"
  return 1
}

get_base_url() {
  if [[ -n "${SMOKE_BASE_URL:-}" ]]; then
    printf '%s' "$SMOKE_BASE_URL"
    return 0
  fi

  local bind_addr="${SCBDB_BIND_ADDR:-0.0.0.0:3000}"
  local host="${bind_addr%:*}"
  local port="${bind_addr##*:}"

  if [[ "$host" == "0.0.0.0" ]]; then
    host="127.0.0.1"
  fi

  printf 'http://%s:%s' "$host" "$port"
}

resolve_auth_header() {
  if [[ -n "${SMOKE_BEARER_TOKEN:-}" ]]; then
    printf 'Authorization: Bearer %s' "$SMOKE_BEARER_TOKEN"
    return 0
  fi

  if [[ -n "${SCBDB_API_KEYS:-}" ]]; then
    local first_key
    first_key="${SCBDB_API_KEYS%%,*}"
    printf 'Authorization: Bearer %s' "$first_key"
    return 0
  fi

  printf ''
}

probe_json_endpoint() {
  local url="$1"
  local expect_status="$2"
  local auth_header="$3"
  local curl_args=(--silent --show-error --output /tmp/scbdb-smoke-body.txt --write-out '%{http_code}' "$url")

  if [[ -n "$auth_header" ]]; then
    curl_args=(--silent --show-error -H "$auth_header" --output /tmp/scbdb-smoke-body.txt --write-out '%{http_code}' "$url")
  fi

  local status
  status="$(curl "${curl_args[@]}")"

  if [[ "$status" != "$expect_status" ]]; then
    err "probe failed for $url (expected $expect_status, got $status)"
    err "response body: $(tr -d '\n' </tmp/scbdb-smoke-body.txt | cut -c1-400)"
    return 1
  fi

  log "Probe OK: $url (status=$status)"
}

wait_for_health() {
  local base_url="$1"
  local health_url="${base_url}/api/v1/health"
  local attempts=$((DEFAULT_TIMEOUT_SECONDS / DEFAULT_POLL_INTERVAL_SECONDS))

  log "Waiting for API health endpoint..."
  for ((i = 1; i <= attempts; i += 1)); do
    if curl --silent --show-error --output /tmp/scbdb-smoke-health.txt --write-out '%{http_code}' "$health_url" | grep -q '^200$'; then
      log "API health endpoint is responding."
      return 0
    fi
    sleep "$DEFAULT_POLL_INTERVAL_SECONDS"
  done

  err "health endpoint did not return 200 within ${DEFAULT_TIMEOUT_SECONDS}s"
  return 1
}

main() {
  cd "$REPO_ROOT"

  log "Startup smoke begins (non-destructive)."

  log "1/6 Starting postgres service..."
  just db-up

  log "2/6 Validating database readiness..."
  wait_for_postgres

  log "3/6 Running migrations..."
  just migrate

  log "4/6 Verifying DB ping..."
  cargo run --quiet --bin scbdb-cli -- db ping

  log "5/6 Booting API server..."
  cargo run --quiet --bin scbdb-server >/tmp/scbdb-smoke-server.log 2>&1 &
  server_pid="$!"

  local base_url
  base_url="$(get_base_url)"
  wait_for_health "$base_url"
  probe_json_endpoint "${base_url}/api/v1/health" "200" ""

  log "6/6 Running representative read probe..."
  local auth_header
  auth_header="$(resolve_auth_header)"
  probe_json_endpoint "${base_url}/api/v1/products?limit=1" "200" "$auth_header"

  log "Startup smoke passed."
}

main "$@"
