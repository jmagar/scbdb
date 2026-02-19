set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

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
    docker compose down -v

clean:
    @if [ -d target ]; then rm -rf target; fi
