# SCBDB

SCBDB is a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages.

## Status Snapshot (Verified)

- Tracks 24 brands from `config/brands.yaml` (portfolio + competitor, tiered 1-3); 16 brands have confirmed Twitter/X handles for brand-timeline monitoring.
- Rust Cargo workspace with 7 crates:
  - `scbdb-cli`
  - `scbdb-server`
  - `scbdb-core`
  - `scbdb-db`
  - `scbdb-scraper`
  - `scbdb-legiscan`
  - `scbdb-sentiment`
- Two binaries:
  - `scbdb-cli`: operator workflows (collect, regs, sentiment, db)
  - `scbdb-server`: Axum API for health, products, pricing, regulatory, and sentiment views
- PostgreSQL 16 with sqlx migrations.
- Current schema includes 10 tables:
  - `brands`
  - `products`
  - `product_variants`
  - `collection_runs`
  - `collection_run_brands`
  - `price_snapshots`
  - `bills`
  - `bill_events`
  - `sentiment_snapshots`
  - `store_locations`
- Frontend (`web/`) is a Vite + React 19 + TypeScript dashboard with five data tabs: Products, Pricing, Regulatory, Sentiment, and Locations. Styling uses raw CSS variables (no Tailwind/shadcn).

## Implemented Capabilities

- Product catalog collection from Shopify storefronts.
- Pricing snapshot collection tied to auditable collection runs.
- Legislative ingestion and reporting via LegiScan.
- Sentiment collection and scoring pipeline (Google News RSS + Reddit sources), with snapshot persistence.
- Store locator crawler — detects Locally.com, Storemapper, JSON-LD, and embedded JSON formats; tracks `first_seen_at` per location for territory monitoring.
- Health-check API at `GET /api/v1/health` plus dashboard endpoints for products, pricing snapshots/summary, bills, sentiment summary/snapshots, and location summary/by-state.

## Known Limitations

- API auth is enabled when `SCBDB_API_KEYS` is set (comma-separated bearer tokens); in development, auth is disabled when keys are omitted.
- `scbdb-cli report` exists as a stub and exits with an error.
- `SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS` is parsed from env but currently not active (collection still runs one brand at a time).
- Store locator URLs in `brands.yaml` are best-guess defaults for most brands; run `collect locations --dry-run` to validate and update via auto-discovery.

## Quickstart

### Prerequisites

- Rust toolchain with MSRV 1.93 (`rustup install stable`)
- Docker + Compose plugin (`docker compose`)
- `sqlx-cli`:
  - `cargo install sqlx-cli --no-default-features --features postgres,rustls`
- `just`:
  - `cargo install just`
- `lefthook`:
  - `cargo install lefthook`
- Node.js (web project requires `>=20.19.0`)
- `pnpm`:
  - `npm install -g pnpm` (or `corepack enable pnpm`)

### First-time setup

```bash
# 1) Copy env template and set secrets/credentials
cp .env.example .env

# 2) Bootstrap database (start postgres, migrate, verify, seed brands)
just bootstrap

# 3) Install git hooks
just hooks

# 4) Install frontend dependencies
pnpm --dir web install
```

### Daily workflow

```bash
just db-up            # start postgres container
just db-down          # stop compose services
just dev              # start postgres and run web dev server (if pnpm is available)
just migrate          # apply pending migrations
just migrate-status   # show migration status
just check            # rust fmt/clippy + web typecheck/lint/format check
just test             # rust + web tests
just ci               # check + test
just format           # apply rust + web formatting
just db-reset         # destroy postgres data (interactive)
just clean            # remove build artifacts (target/)
```

## Server

```bash
cargo run --bin scbdb-server
curl http://localhost:3000/api/v1/health
```

Default bind address is `0.0.0.0:3000` (`SCBDB_BIND_ADDR` overrides).

## CLI Commands

### Database

```bash
cargo run --bin scbdb-cli -- db ping
cargo run --bin scbdb-cli -- db migrate
cargo run --bin scbdb-cli -- db seed
```

### Product & Pricing Collection

```bash
cargo run --bin scbdb-cli -- collect products
cargo run --bin scbdb-cli -- collect products --brand <slug>
cargo run --bin scbdb-cli -- collect products --dry-run

cargo run --bin scbdb-cli -- collect pricing
cargo run --bin scbdb-cli -- collect pricing --brand <slug>
```

### Regulatory Tracking

```bash
cargo run --bin scbdb-cli -- regs ingest
cargo run --bin scbdb-cli -- regs ingest --state SC --keyword hemp --dry-run
cargo run --bin scbdb-cli -- regs status
cargo run --bin scbdb-cli -- regs status --state SC --limit 50
cargo run --bin scbdb-cli -- regs timeline --state SC --bill HB1234
cargo run --bin scbdb-cli -- regs report
cargo run --bin scbdb-cli -- regs report --state SC
```

### Sentiment

```bash
cargo run --bin scbdb-cli -- sentiment collect
cargo run --bin scbdb-cli -- sentiment collect --brand cann --dry-run
cargo run --bin scbdb-cli -- sentiment status
cargo run --bin scbdb-cli -- sentiment status --brand cann
cargo run --bin scbdb-cli -- sentiment report
cargo run --bin scbdb-cli -- sentiment report --brand cann
```

### Store Locator

```bash
cargo run --bin scbdb-cli -- collect locations
cargo run --bin scbdb-cli -- collect locations --brand cann
cargo run --bin scbdb-cli -- collect locations --dry-run
```

### Not Yet Implemented

```bash
cargo run --bin scbdb-cli -- report
```

## Environment Variables

Copy `.env.example` to `.env`.

| Variable | Required | Default | Notes |
|---|---|---|---|
| `DATABASE_URL` | Yes | `postgres://scbdb:changeme@localhost:15432/scbdb` | Primary DB connection string |
| `POSTGRES_PASSWORD` | Yes | `changeme` in template | Must be set for local docker postgres |
| `POSTGRES_PORT` | No | `15432` | Host port mapping |
| `POSTGRES_DB` | No | `scbdb` | Docker postgres DB name |
| `POSTGRES_USER` | No | `scbdb` | Docker postgres user |
| `SCBDB_ENV` | No | `development` | `development`, `test`, or `production` |
| `SCBDB_BIND_ADDR` | No | `0.0.0.0:3000` | API listen address |
| `SCBDB_LOG_LEVEL` | No | `info` | Used when `RUST_LOG` is unset |
| `SCBDB_BRANDS_PATH` | No | `./config/brands.yaml` | Brand registry path |
| `LEGISCAN_API_KEY` | Optional* | empty | Required for meaningful `regs ingest` runs |
| `SCBDB_DB_MAX_CONNECTIONS` | No | `10` | DB pool max |
| `SCBDB_DB_MIN_CONNECTIONS` | No | `1` | DB pool min (must be <= max) |
| `SCBDB_DB_ACQUIRE_TIMEOUT_SECS` | No | `10` | DB acquire timeout |
| `SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS` | No | `30` | Scraper request timeout |
| `SCBDB_LEGISCAN_REQUEST_TIMEOUT_SECS` | No | `30` | LegiScan request timeout |
| `SCBDB_SCRAPER_USER_AGENT` | No | `scbdb/0.1 (product-intelligence)` | Scraper user agent |
| `SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS` | No | `1` | Parsed, not currently active |
| `SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS` | No | `250` | Inter-request delay |
| `SCBDB_SCRAPER_MAX_RETRIES` | No | `3` | Retry attempts |
| `SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS` | No | `5` | Backoff base |
| `SENTIMENT_TEI_URL` | No** | `http://localhost:52000` | Parsed by sentiment pipeline |
| `SENTIMENT_QDRANT_URL` | No** | `http://localhost:53333` | Parsed by sentiment pipeline |
| `SENTIMENT_QDRANT_COLLECTION` | No** | `scbdb_sentiment` | Parsed by sentiment pipeline |
| `REDDIT_CLIENT_ID` | No** | empty | Parsed by sentiment pipeline |
| `REDDIT_CLIENT_SECRET` | No** | empty | Parsed by sentiment pipeline |
| `REDDIT_USER_AGENT` | No** | `scbdb/0.1.0` | Parsed by sentiment pipeline |

- `*` `LEGISCAN_API_KEY` is optional for booting, but required to ingest real regulatory data.
- `**` Sentiment vars are not required for server startup, but they are required for `sentiment collect` (the command fails if any key is missing from env).

## Core Docs

- `docs/INDEX.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/TECHNOLOGY.md`
- `docs/DATABASE_SCHEMA.md`
- `docs/API_DESIGN.md`
- `docs/CONFIG_LOADING.md`
- `docs/MVP.md`
- `docs/mvp_phases/`
- `docs/DEPLOYMENT.md`
- `docs/DEVELOPMENT.md`
- `docs/TESTING.md`
- `docs/LOGGING.md`
- `docs/EXTRACTION_PROMPT_SCHEMA.md`
- `docs/SENTIMENT_PIPELINE.md`
- `docs/STORE_LOCATOR.md`
- `docs/LOCATIONS_DASHBOARD.md`
