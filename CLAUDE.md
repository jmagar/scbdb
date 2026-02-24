# CLAUDE.md

## Project Identity

SCBDB is a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages.

## Stack Snapshot

- Backend: Rust workspace (`scbdb-cli`, `scbdb-server`, shared crates)
- API: Axum + tower middleware
- DB: PostgreSQL + sqlx migrations
- Frontend: React 19 + TypeScript + Vite
- Styling/UI: Tailwind v4 + shadcn/ui

## Crate Map

| Crate | Type | Responsibility |
|-------|------|----------------|
| `scbdb-cli` | bin | clap CLI — `collect`, `regs`, `db` subcommands |
| `scbdb-server` | bin | Axum HTTP server — REST API + scheduler |
| `scbdb-core` | lib | Shared domain types, models, error types |
| `scbdb-db` | lib | sqlx queries, migrations, DB access layer |
| `scbdb-scraper` | lib | Shopify `products.json` collector + retry/backoff |
| `scbdb-legiscan` | lib | LegiScan API client + bill ingestion |
| `scbdb-sentiment` | lib | Market sentiment pipeline |
| `scbdb-profiler` | lib | Brand intelligence profiler — RSS/YouTube/Twitter signal collection + embeddings |

## Product Scope

- Single brand registry: `config/brands.yaml`
- Brand relationship model: `portfolio` or `competitor`
- Tier model: `1`, `2`, `3`
- Core capabilities: product intelligence, pricing history, regulatory tracking, reporting
- Phase 4+ infrastructure: Qdrant (signal dedup), TEI (embeddings) — operational
- Post-MVP roadmap: Spider

## Canonical Docs

- `docs/INDEX.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/TECHNOLOGY.md`
- `docs/DATABASE_SCHEMA.md`
- `docs/API_DESIGN.md`
- `docs/CONFIG_LOADING.md`
- `docs/MVP.md`
- `docs/DEPLOYMENT.md`
- `docs/DEVELOPMENT.md`
- `docs/TESTING.md`
- `docs/LOGGING.md`
- `docs/EXTRACTION_PROMPT_SCHEMA.md`
- `docs/SENTIMENT_PIPELINE.md`
- `docs/SENTIMENT_DASHBOARD.md`
- `docs/STORE_LOCATOR.md`
- `docs/LOCATIONS_DASHBOARD.md`

## Development Rules

- CLI-first delivery for MVP phases.
- Use typed configuration and fail fast on startup errors.
- Validate external data at boundaries.
- Keep migrations append-only and schema docs current.
- Keep docs aligned to implemented architecture; no historical-stack references in active docs.

## Verification Commands

| Command | Purpose |
|---------|---------|
| `just ci` | Full gate: check + test |
| `just check` | fmt check + clippy + web lint/typecheck |
| `just test` | Rust + web tests |
| `just migrate-status` | Current migration state |
| `cargo clippy --workspace -- -D warnings` | Clippy strict (CI-equivalent) |
| `just build` | Build workspace artifacts |
| `just format` | Apply formatters (cargo fmt + web) |
| `just migrate` | Apply pending migrations |
| `just seed` | Seed brands from `config/brands.yaml` |
| `just db-up` / `just db-down` | Start / stop PostgreSQL container |
| `just db-reset` | Destroy and recreate PostgreSQL data (destructive) |
| `just hooks` | Install lefthook git hooks |
| `just bootstrap` | Full environment setup: db-up → migrate → ping → seed |
| `just serve` | Start API server + web dev server (Ctrl-C stops both) |
| `just collect-products` | Shorthand: collect products for all brands |
| `just collect-brand <slug>` | Shorthand: collect products for one brand |
| `just collect-pricing` | Shorthand: capture pricing snapshots |
| `just collect-dry` | Shorthand: dry-run product collection |
| `just dev` | Start postgres + web dev server |
| `just clean` | Remove `target/` build artifacts |

## Operational Defaults

| Setting | Value |
|---------|-------|
| DB name | `scbdb` |
| DB user | `scbdb` |
| DB host port | `15432` (avoid conflict with system postgres) |
| DB container | `scbdb-postgres` |
| Server port | `3000` (via `SCBDB_BIND_ADDR=0.0.0.0:3000`) |
| API base path | `/api/v1/` |
| TEI URL | `http://localhost:52000` (sentiment embeddings) |
| Qdrant URL | `http://localhost:53333` (sentiment vector store) |
| Brands config | `./config/brands.yaml` (via `SCBDB_BRANDS_PATH`) |
| Required env vars | `POSTGRES_PASSWORD`, `DATABASE_URL` |

## Quick Start

```bash
cp .env.example .env          # set POSTGRES_PASSWORD + LEGISCAN_API_KEY
just bootstrap                 # db-up → wait → migrate → ping → seed
just dev                       # start postgres + web dev server
```

## CLI Subcommand Reference

```bash
# Product collection
scbdb-cli collect products                     # collect all brands
scbdb-cli collect products --brand <slug>      # single brand
scbdb-cli collect products --dry-run           # preview without DB writes
scbdb-cli collect pricing                      # capture price snapshots
scbdb-cli collect pricing --brand <slug>       # single brand
scbdb-cli collect verify-images                # verify stored image URLs return HTTP 200
scbdb-cli collect verify-images --brand <slug> # single brand
scbdb-cli collect verify-images --concurrency 12
scbdb-cli collect locations                    # collect retail store locations for all brands
scbdb-cli collect locations --brand <slug>     # single brand
scbdb-cli collect locations --dry-run          # preview without DB writes

# Sentiment signals
scbdb-cli sentiment collect                    # collect + score signals for all brands
scbdb-cli sentiment collect --brand <slug>     # single brand
scbdb-cli sentiment collect --dry-run          # preview without DB/Qdrant writes
scbdb-cli sentiment status [--brand <slug>]    # show recent sentiment scores
scbdb-cli sentiment report [--brand <slug>]    # generate markdown sentiment report

# Reporting (Phase 5)
scbdb-cli report                               # generate reports/exports

# Regulatory tracking
scbdb-cli regs ingest [--state SC] [--keyword hemp] [--dry-run]
scbdb-cli regs status [--state SC] [--limit 20]
scbdb-cli regs timeline --state SC --bill HB1234
scbdb-cli regs report [--state SC]

# Database management
scbdb-cli db ping         # verify DB connection
scbdb-cli db migrate      # apply pending migrations
scbdb-cli db seed         # seed brands from config/brands.yaml
```

## Codebase Gotchas

- **Tower middleware order** — In `ServiceBuilder`, the LAST `.layer()` is outermost (runs first). Add `request_id` after `TraceLayer`, not before.
- **LegiScan `search` response** — Returns numbered JSON objects `{"0":{...},"1":{...}}`, not a Vec. Deserialize as `HashMap<String, Value>` with `#[serde(flatten)]`, filter numeric keys. Same pattern as `MasterListData`.
- **sqlx + `SELECT 1`** — PostgreSQL `int4` maps to `i32`, not `i64`. Use `query_scalar::<_, i32>`.
- **`dotenvy` policy** — Library crates must NOT call `dotenvy::dotenv()`. Only binary entrypoints (`scbdb-cli`, `scbdb-server`) load `.env`.
- **Git hooks** — pre-commit runs `cargo fmt`; pre-push runs `cargo test` + `cargo clippy -D warnings`. Run `just check` and `just test` before pushing to avoid failed pushes.
- **`SCBDB_API_KEYS` auth mode** — When unset, Bearer auth is disabled (all endpoints open). When set to a comma-separated list of tokens, all non-`/health` endpoints require `Authorization: Bearer <token>`. Safe default for dev; required for production.
- **Web routing** — Hash-based routing (`window.location.hash`), no React Router. Routes: `#/brands` (brand list), `#/brands/<slug>` (brand profile), default (dashboard). Switch lives in `web/src/main.tsx`.
- **API base URL** — Server binds to `SCBDB_BIND_ADDR` (default `0.0.0.0:3000`). Vite dev proxy forwards `/api/*` → `localhost:3000`. Direct API testing: `http://localhost:3000/api/v1/health`.

## Web Frontend

Single-page app at `web/` — hash-based navigation, no router library.

| Hash | Component |
|------|-----------|
| `#/` (default) | `DashboardPage` — 5-tab view (Products, Pricing, Regulatory, Sentiment, Locations) |
| `#/brands` | `BrandsPage` — brand registry list |
| `#/brands/<slug>` | `BrandProfilePage` — profile, signals, edit forms |

Key layout:

| Path | Purpose |
|------|---------|
| `web/src/components/` | All UI panels |
| `web/src/hooks/use-dashboard-data.ts` | TanStack Query hooks for all 6 API queries |
| `web/src/lib/api/` | `client.ts` (apiGet helper) + `dashboard.ts` (typed fetch functions) |
| `web/src/types/api.ts` | TypeScript types mirroring server response shapes |

Dev: `pnpm --dir web dev` (frontend only) or `just serve` (API server + web together).

## Documentation Workflow

When project behavior or architecture changes:

1. Update canonical technical docs first.
2. Update `docs/PRD.md` if product requirements changed.
3. Update phase docs if delivery sequencing changed.
4. Keep this file synchronized with current state.
