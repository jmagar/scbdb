# Review Scope

## Target

Full codebase review of SCBDB — a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages. Strict mode enabled.

## Technology Stack

- **Backend**: Rust workspace (8 crates) — Axum + tower middleware, sqlx + PostgreSQL
- **Frontend**: React 19 + TypeScript + Vite, Tailwind v4 + shadcn/ui
- **Infrastructure**: Docker Compose, Qdrant (vector search), TEI (embeddings)

## Crate Map

| Crate | Type | Files |
|-------|------|-------|
| `scbdb-cli` | bin | CLI entry — collect, regs, sentiment, db subcommands |
| `scbdb-server` | bin | Axum HTTP server — REST API + scheduler |
| `scbdb-core` | lib | Shared domain types, models, error types, config |
| `scbdb-db` | lib | sqlx queries, migrations, DB access layer |
| `scbdb-scraper` | lib | Shopify products.json collector, store locator parsers |
| `scbdb-legiscan` | lib | LegiScan API client + bill ingestion |
| `scbdb-sentiment` | lib | Sentiment pipeline — RSS/Reddit/YouTube/Twitter signals |
| `scbdb-profiler` | lib | Brand intelligence — RSS/YouTube/Twitter signal collection + embeddings |

## Files Included

### Rust Source (~100 files)
- `crates/scbdb-cli/src/` — CLI commands (collect, regs, sentiment)
- `crates/scbdb-core/src/` — Domain models, config, brands
- `crates/scbdb-db/src/` — Database queries, seed, migrations logic
- `crates/scbdb-scraper/src/` — Shopify client, store locator formats (13+ parsers)
- `crates/scbdb-legiscan/src/` — LegiScan client, types, retry logic
- `crates/scbdb-sentiment/src/` — Sentiment pipeline, sources, embeddings, scoring
- `crates/scbdb-profiler/src/` — Brand profiler, RSS/YouTube/Twitter intake
- `crates/scbdb-server/src/` — Axum API routes, middleware, scheduler

### TypeScript/React Source (~40 files)
- `web/src/components/` — Dashboard, brand pages, panels, forms
- `web/src/hooks/` — TanStack Query data hooks
- `web/src/lib/` — API client, utilities, color system
- `web/src/types/` — API response type definitions

### Configuration & Infrastructure
- `Cargo.toml`, `docker-compose.yml`, `lefthook.yml`
- `config/brands.yaml`
- `migrations/` — 17 SQL migration pairs
- `.github/workflows/ci.yml`

### Tests
- Rust: `*_test.rs` files + `tests/` directories in crates
- TypeScript: `*.test.ts` and `*.test.tsx` files in `web/src/`

## Flags

- Security Focus: no
- Performance Critical: no
- Strict Mode: **yes** (--strict flag detected)
- Framework: auto-detected (Rust/Axum backend, React/Vite frontend)

## Review Phases

1. Code Quality & Architecture
2. Security & Performance
3. Testing & Documentation
4. Best Practices & Standards
5. Consolidated Report
