# SCBDB

## Document Metadata

- Version: 1.2
- Status: Active
- Last Updated (EST): 09:30:00 | 02/19/2026 EST

SCBDB is a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages.

## What It Does

- Tracks 25 brands from a single `config/brands.yaml` registry (portfolio + competitor, tiered 1–3).
- Collects product catalog and variant data from Shopify storefronts via `scbdb-scraper`.
- Captures time-series pricing snapshots tied to auditable collection runs.
- Tracks legislative activity and bill timelines via `scbdb-legiscan` (Phase 3).
- Runs sentiment analysis over product and brand data via `scbdb-sentiment` (Phase 4).
- Generates decision-ready outputs for portfolio and competitor analysis (Phase 5).

## Current Scope

- Rust Cargo workspace with 7 crates: `scbdb-cli`, `scbdb-server`, `scbdb-core`, `scbdb-db`, `scbdb-scraper`, `scbdb-legiscan`, `scbdb-sentiment`.
- Two binaries: `scbdb-cli` (operator commands) and `scbdb-server` (Axum HTTP API).
- PostgreSQL 16 persistence with sqlx migrations (8 tables: brands, products, product variants, price snapshots, collection runs, bills, bill events, and run-brand outcomes).
- CLI-first MVP execution; the server is a read-only query layer over the database.
- React 19 + TypeScript frontend (`web/`) with Tailwind v4 + shadcn/ui — Phase 5.
- Phased delivery docs under `docs/mvp_phases/`.

Post-MVP roadmap includes Spider, Qdrant, and TEI integration.

## Phase 1 Quickstart

### Prerequisites

- Rust (stable, MSRV 1.93.0): `rustup install stable`
- Docker + Docker Compose plugin
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features postgres,rustls`
- `just`: `cargo install just`
- `lefthook`: `cargo install lefthook` (git hooks — pre-commit format, pre-push clippy + tests)
- `pnpm` (web frontend): `npm install -g pnpm` or `corepack enable pnpm`

### First-time setup

```bash
# 1. Copy env template and set POSTGRES_PASSWORD
cp .env.example .env

# 2. Bootstrap: starts postgres, waits for healthy, runs migrations, seeds brands, verifies DB
just bootstrap

# 3. Install git hooks (one-time per clone)
just hooks
```

### Daily workflow

```bash
just db-up            # start postgres container
just db-down          # stop postgres container
just dev              # start postgres + web dev server together
just migrate          # apply pending migrations
just migrate-status   # show current migration state
just ci               # full gate: check + test (run before pushing)
just check            # fmt check + clippy + web typecheck + lint
just test             # run all tests (Rust + web)
just format           # fix formatting issues (cargo fmt + pnpm format)
just db-reset         # destroy all postgres data (interactive prompt)
just clean            # remove build artifacts (target/)
```

### Verify the server

```bash
cargo run --bin scbdb-server &
curl http://localhost:3000/api/v1/health
# → {
#     "data": {"status": "ok", "database": "ok"},
#     "meta": {"request_id": "550e8400-e29b-41d4-a716-446655440000", "timestamp": "2026-02-19T09:00:00.000Z"}
#   }
```

The server binds to `0.0.0.0:3000` by default. Override with the `SCBDB_BIND_ADDR` environment variable.

### CLI commands

```bash
# Database
cargo run --bin scbdb-cli -- db ping      # test DB connection
cargo run --bin scbdb-cli -- db migrate   # run pending migrations
cargo run --bin scbdb-cli -- db seed      # seed brands from config/brands.yaml

# Data collection (Phase 2 — scaffold wired, DB writes pending)
cargo run --bin scbdb-cli -- collect products                       # collect product catalog for all brands
cargo run --bin scbdb-cli -- collect products --brand <SLUG>        # restrict to one brand by slug
cargo run --bin scbdb-cli -- collect products --dry-run             # preview without writing to DB
cargo run --bin scbdb-cli -- collect pricing                        # capture pricing snapshots
cargo run --bin scbdb-cli -- collect pricing --brand <SLUG>         # restrict to one brand

# Regulatory tracking (Phase 3 — fully implemented)
cargo run --bin scbdb-cli -- regs ingest    # ingest bills from LegiScan into DB
cargo run --bin scbdb-cli -- regs status    # show current bill statuses
cargo run --bin scbdb-cli -- regs timeline  # show bill event timeline
cargo run --bin scbdb-cli -- regs report    # generate regulatory summary report

# Not yet implemented (stub that exits with an error)
# cargo run --bin scbdb-cli -- report  # reports and exports (Phase 5)
```

## Environment Variables

Copy `.env.example` to `.env`. Key variables:

| Variable | Default | Notes |
|---|---|---|
| `POSTGRES_PASSWORD` | (none) | **Required** — no default, must be set explicitly |
| `DATABASE_URL` | `postgres://scbdb:changeme@localhost:15432/scbdb` | Full connection string |
| `POSTGRES_PORT` | `15432` | Host-side port mapping (avoids collision with a system postgres on 5432) |
| `SCBDB_BIND_ADDR` | `0.0.0.0:3000` | Server listen address |
| `SCBDB_LOG_LEVEL` | `info` | Log verbosity |
| `SCBDB_BRANDS_PATH` | `./config/brands.yaml` | Path to brand registry |
| `LEGISCAN_API_KEY` | (empty) | Required for legislative tracking (Phase 3) |

See `.env.example` for the full list, including scraper tuning variables (`timeout`, `user agent`, `concurrency`, `retry backoff`).

## Core Docs

- Documentation index: `docs/INDEX.md`
- Product requirements: `docs/PRD.md`
- Architecture: `docs/ARCHITECTURE.md`
- Technology stack: `docs/TECHNOLOGY.md`
- Database schema: `docs/DATABASE_SCHEMA.md`
- API design: `docs/API_DESIGN.md`
- Config loading strategy: `docs/CONFIG_LOADING.md`
- MVP index and phases: `docs/MVP.md`
- MVP phase breakdown: `docs/mvp_phases/` (phases 1–5)
- Deployment runbook: `docs/DEPLOYMENT.md`
- Development standards: `docs/DEVELOPMENT.md`
- Testing standards: `docs/TESTING.md`
- Logging and errors: `docs/LOGGING.md`
- Extraction prompt schema: `docs/EXTRACTION_PROMPT_SCHEMA.md`
