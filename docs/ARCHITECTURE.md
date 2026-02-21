# Architecture

## Document Metadata

- Version: 1.2
- Status: Active
- Last Updated (EST): 00:00:00 | 02/21/2026 EST

## Target Repository Structure

Cargo workspace monorepo with the Vite frontend co-located at the top level.

```
scbdb/
+-Cargo.toml              # workspace root
+-Cargo.lock
+-.cargo/
|  +-config.toml         # workspace-wide cargo settings
+-crates/
|  +-scbdb-cli/          # binary -- clap CLI entrypoint
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-main.rs
|  +-scbdb-server/       # binary -- axum HTTP server
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-main.rs
|  +-scbdb-core/         # lib -- shared domain types, models, error types
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-lib.rs
|  +-scbdb-db/           # lib -- database access, migrations, queries
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-lib.rs
|  +-scbdb-scraper/      # lib -- Shopify products.json collector
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-lib.rs
|  +-scbdb-legiscan/     # lib -- LegiScan API client and ingestion
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-lib.rs
|  +-scbdb-sentiment/    # lib -- market sentiment pipeline
|  |  +-Cargo.toml
|  |  +-src/
|  |     +-lib.rs
|  +-scbdb-profiler/     # lib -- brand signal ingestion (YouTube, RSS, newsroom, social)
|     +-Cargo.toml
|     +-src/
|        +-lib.rs
+-web/                    # Vite + React 19 frontend
|  +-package.json
|  +-vite.config.ts
|  +-tsconfig.json
|  +-index.html
|  +-src/
|     +-main.tsx
|     +-components/
|     +-lib/
+-config/
|  +-brands.yaml         # seed data -- brand registry (portfolio + competitor, URLs, tiers, notes)
+-migrations/             # sqlx SQL migration files (append-only)
+-justfile                # task runner (build, dev, migrate, etc.)
+-lefthook.yml            # git hooks config (pre-commit checks)
+-docker-compose.yml      # local dev services (PostgreSQL)
+-Dockerfile              # multi-stage build for Rust backend
+-.github/
   +-workflows/
      +-ci.yml          # GitHub Actions CI pipeline
```

## Workspace Layout

### Binaries

| Crate | Type | Purpose |
|---|---|---|
| `scbdb-cli` | bin | clap CLI -- subcommands for scraping, brand management, legislative queries, and report export |
| `scbdb-server` | bin | Axum HTTP server -- REST API consumed by the frontend |

### Libraries

| Crate | Type | Purpose |
|---|---|---|
| `scbdb-core` | lib | Shared domain models, error types, configuration, and common traits |
| `scbdb-db` | lib | PostgreSQL layer -- sqlx connection pooling, migrations, typed queries |
| `scbdb-scraper` | lib | Shopify `products.json` collector -- pagination, normalization, rate limiting |
| `scbdb-legiscan` | lib | LegiScan API client -- bill/amendment/vote ingestion |
| `scbdb-sentiment` | lib | Market sentiment aggregation and scoring |
| `scbdb-profiler` | lib | Brand intelligence profiler -- collects signals via RSS/YouTube/Twitter, embeds via TEI, upserts to DB |

### Dependency Graph

```
scbdb-cli ------+---> scbdb-scraper  --> scbdb-core
                +---> scbdb-legiscan --> scbdb-core
                +---> scbdb-sentiment-> scbdb-core
                +---> scbdb-db -------> scbdb-core

scbdb-server ---+---> scbdb-db -------> scbdb-core
                +---> scbdb-scraper  --> scbdb-core
                +---> scbdb-legiscan --> scbdb-core
                +---> scbdb-sentiment-> scbdb-core
                +---> scbdb-profiler --> scbdb-core
                                    --> scbdb-db
```

Both binaries depend on the same set of library crates. No library crate depends on a binary
crate. `scbdb-core` is the leaf dependency with zero internal deps.

## Database

**PostgreSQL** is the sole persistent data store. All access goes through the `scbdb-db` crate
using **sqlx** -- compile-time checked SQL queries with async support and zero ORM overhead.

### Key choices

- **Raw SQL over ORM** -- sqlx validates queries against the real database schema at compile
  time. No runtime query building, no magic.
- **Connection pooling** -- sqlx's built-in `PgPool` manages connections. The pool is created
  once at startup and shared via Axum state or passed into CLI command handlers.
- **Migrations** -- SQL files in `migrations/` managed by `sqlx migrate`. Applied via
  `just migrate` or automatically on server startup in development.

### Schema conventions

- All tables use `snake_case` names.
- Primary keys are `id BIGINT GENERATED ALWAYS AS IDENTITY`.
- Timestamps use `TIMESTAMPTZ` (never `TIMESTAMP`), defaulting to `NOW()`.
- Soft deletes via `deleted_at TIMESTAMPTZ` where needed -- never hard-delete user-facing data.
- Foreign keys are always indexed.

## Backend (Rust)

### CLI (`scbdb-cli`)

Rust binary built with **clap** for command parsing. Single entry point with subcommands for
collection, reporting, legislative queries, and competitor management.

### API Server (`scbdb-server`)

**Axum**-based HTTP server exposing a REST API consumed by the frontend. Serves product data,
competitor listings, legislative tracking, market sentiment, brand profiles, and scrape status.
Protected by API key authentication, CORS, and rate limiting middleware via tower layers.

### Data Collection

#### Shopify Scraper (`scbdb-scraper`)

Custom scraper that fetches `{domain}/products.json` from Shopify-powered competitor
storefronts. Handles pagination, normalizes the Shopify product/variant schema into the internal
product model, and persists structured data to the database.

This is the primary ingestion path -- most tracked brands run Shopify storefronts (see
`config/brands.yaml` for shop URLs).

#### LegiScan Extraction (`scbdb-legiscan`)

Integration with the LegiScan API for tracking cannabis-related legislation. Ingests bills,
amendments, and vote records into the database.

### Market Sentiment (`scbdb-sentiment`)

Pipeline for aggregating and scoring market sentiment signals alongside product and legislative
data.

### Brand Profiler (`scbdb-profiler`)

Library crate that owns brand signal ingestion from multiple external sources. Added in Phase 6
as part of the Brand Intelligence Layer.

Responsibilities:

- Per-source collectors for YouTube (Data API v3), Twitter/X (via CLI wrapper), and RSS feeds.
- Each collector fetches new content, deduplicates against existing `brand_signals` rows via the
  `(brand_id, signal_type, external_id)` unique constraint, and upserts signals to the database.
- Signals are embedded via TEI (Text Embeddings Inference) after upsert; the resulting Qdrant
  point ID is stored in `brand_signals.qdrant_point_id`.
- Exposes a top-level `ingest_signals(pool, config, brand_id, feed_urls, youtube_channel_id,
  twitter_handle)` entry point consumed by scheduler jobs registered in `scbdb-server`.
- Individual collector failures are captured in `BrandProfileRunResult::errors` rather than
  propagating -- the pipeline continues past non-fatal failures.

Dependencies: `scbdb-core`, `scbdb-db`, `reqwest`, `tokio`, `serde_json`.

### Phase 4 Infrastructure

The following components are integrated in the Phase 4 sentiment pipeline:

- **Qdrant** vector index -- signal dedup and future semantic search (`scbdb_sentiment`
  collection)
- **TEI** embedding generation -- Qwen3-Embedding-0.6B, 1024-dimensional vectors

### Post-MVP Components

The following components are part of the roadmap but out of MVP scope:

- Spider fallback crawler for non-Shopify sources

## Frontend (`web/`)

**Vite**-powered **React 19** SPA styled with **Tailwind CSS 4+** and **shadcn/ui** components.
Communicates with the Axum backend over REST. Provides dashboards for browsing competitor
products, tracking legislation, comparing pricing, viewing market sentiment, and monitoring
scrape runs.

### Phase 6 Frontend Additions

Phase 6 adds a brand intelligence UI layer:

- **`/brands`** -- Brand list page. Displays all tracked brands as cards with tier badge,
  relationship tag (portfolio vs. competitor), and a completeness progress bar sourced from the
  `brand_completeness` read model computed in `scbdb-db`.
- **`/brands/:slug`** -- Brand profile page. Full-detail view with three tabs:
  - **Feed** -- Chronological signal stream from `brand_signals` (tweets, YouTube videos, press
    releases, RSS items). Implemented by `BrandSignalFeed`.
  - **Content** -- Structured content: funding events, media appearances, sponsorships, lab
    tests. Implemented by `BrandContentTab`.
  - **Recon** -- Competitive intelligence: distributor map, legal proceedings, competitor
    relationships, domain roster. Implemented by `BrandReconTab`.
