set shell := ["bash", "-eu", "-o", "pipefail", "-c"]
set dotenv-load := true

default:
    @just --list

ci: check test

# Bootstrap local environment: start db, migrate, verify health
bootstrap:
    @echo "Starting database..."
    just db-up
    @echo "Waiting for postgres to be ready..."
    @for i in {1..60}; do \
      if docker exec scbdb-postgres pg_isready -U scbdb >/dev/null 2>&1; then break; fi; \
      sleep 1; \
    done; \
    if ! docker exec scbdb-postgres pg_isready -U scbdb >/dev/null 2>&1; then \
      echo "error: postgres did not become healthy within 60s"; \
      exit 1; \
    fi
    @echo "Running migrations..."
    just migrate
    @echo "Verifying database health..."
    cargo run --bin scbdb-cli -- db ping
    @echo "Seeding brand registry..."
    just seed
    @echo "Bootstrap complete."

# Seed brands from config/brands.yaml
seed:
    cargo run --bin scbdb-cli -- db seed

# Collect full product catalog from all brands
collect-products:
    cargo run --bin scbdb-cli -- collect products

# Collect product catalog for a single brand (usage: just collect-brand <slug>)
collect-brand brand:
    cargo run --bin scbdb-cli -- collect products --brand '{{brand}}'

# Capture price snapshots for all brands
collect-pricing:
    cargo run --bin scbdb-cli -- collect pricing

# Dry-run product collection (preview only, no DB writes)
collect-dry:
    cargo run --bin scbdb-cli -- collect products --dry-run

dev:
    @echo "Starting local dependencies..."
    docker compose up -d postgres
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then \
      echo "Starting web dev server..."; \
      pnpm --dir web dev; \
    fi

# Start API server + web dev server together; Ctrl-C stops both
serve:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Starting postgres..."
    docker compose up -d postgres
    echo "Starting API server..."
    cargo run --bin scbdb-server &
    SERVER_PID=$!
    cleanup() {
        echo ""
        echo "Shutting down..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    }
    trap cleanup EXIT INT TERM
    echo "Starting web dev server..."
    pnpm --dir web dev

build:
    @if [ -f Cargo.toml ]; then cargo build --workspace; else echo "No Cargo workspace yet"; fi

test:
    @if [ -f Cargo.toml ]; then cargo test --workspace; else echo "No Cargo workspace yet"; fi
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then pnpm --dir web test; fi

check:
    @if [ -f Cargo.toml ]; then cargo fmt --all -- --check; else echo "No Cargo workspace yet"; fi
    @if [ -f Cargo.toml ]; then cargo clippy --workspace -- -D warnings; else true; fi
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then pnpm --dir web typecheck; fi
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then pnpm --dir web lint; fi
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then pnpm --dir web format:check; fi

format:
    @if [ -f Cargo.toml ]; then cargo fmt --all; else echo "No Cargo workspace yet"; fi
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then pnpm --dir web format; fi

migrate:
    sqlx migrate run

migrate-status:
    sqlx migrate info

db-up:
    docker compose up -d postgres

db-down:
    docker compose down

db-reset:
    @printf "This will destroy all postgres data. Continue? [y/N] " && read r && case "$$r" in [yY]|[yY][eE][sS]) docker compose down -v ;; *) echo "Aborted." ;; esac

hooks:
    lefthook install

clean:
    @if [ -d target ]; then rm -rf target; fi
