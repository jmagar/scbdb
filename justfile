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
    @status=""; \
    for i in $$(seq 1 60); do \
      status="$$(docker inspect --format='{{{{.State.Health.Status}}}}' scbdb-postgres 2>/dev/null || true)"; \
      if [ "$$status" = "healthy" ]; then break; fi; \
      sleep 1; \
    done; \
    if [ "$$status" != "healthy" ]; then \
      echo "error: postgres did not become healthy within 60s (last status: $$status)"; \
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

dev:
    @echo "Starting local dependencies..."
    docker compose up -d postgres
    @if [ -d web ] && command -v pnpm >/dev/null 2>&1; then \
      echo "Starting web dev server..."; \
      pnpm --dir web dev; \
    fi

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
    @printf "This will destroy all postgres data. Continue? [y/N] " && read r && [ "$$r" = "y" ] && docker compose down -v

hooks:
    lefthook install

clean:
    @if [ -d target ]; then rm -rf target; fi
