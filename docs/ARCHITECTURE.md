# Architecture

## Repository Structure

Cargo workspace monorepo with the Vite frontend co-located at the top level.

```
scbdb/
├── Cargo.toml              # workspace root
├── Cargo.lock
├── .cargo/
│   └── config.toml         # workspace-wide cargo settings
├── crates/
│   ├── scbdb-cli/          # binary — clap CLI entrypoint
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── scbdb-server/       # binary — axum HTTP server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── scbdb-core/         # lib — shared domain types, models, error types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── scbdb-db/           # lib — database access, migrations, queries
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── scbdb-scraper/      # lib — Shopify products.json collector
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── scbdb-legiscan/     # lib — LegiScan API client and ingestion
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   └── scbdb-sentiment/    # lib — market sentiment pipeline
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── web/                    # Vite + React 19 frontend
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── components/
│       └── lib/
├── config/
│   └── brands.yaml         # seed data — competitor shop URLs
├── migrations/             # SQL migration files
└── justfile                # task runner (build, dev, migrate, etc.)
```

## Workspace Layout

### Binaries

| Crate | Type | Purpose |
|---|---|---|
| `scbdb-cli` | bin | clap CLI — subcommands for scraping, brand management, legislative queries, data export |
| `scbdb-server` | bin | Axum HTTP server — REST API consumed by the frontend |

### Libraries

| Crate | Type | Purpose |
|---|---|---|
| `scbdb-core` | lib | Shared domain models, error types, configuration, and common traits |
| `scbdb-db` | lib | Database layer — connection pooling, migrations, query builders |
| `scbdb-scraper` | lib | Shopify `products.json` collector — pagination, normalization, rate limiting |
| `scbdb-legiscan` | lib | LegiScan API client — bill/amendment/vote ingestion |
| `scbdb-sentiment` | lib | Market sentiment aggregation and scoring |

### Dependency Graph

```
scbdb-cli ──────┬──▶ scbdb-scraper ──▶ scbdb-core
                ├──▶ scbdb-legiscan ──▶ scbdb-core
                ├──▶ scbdb-sentiment ─▶ scbdb-core
                └──▶ scbdb-db ────────▶ scbdb-core

scbdb-server ───┬──▶ scbdb-db ────────▶ scbdb-core
                ├──▶ scbdb-scraper ──▶ scbdb-core
                ├──▶ scbdb-legiscan ──▶ scbdb-core
                └──▶ scbdb-sentiment ─▶ scbdb-core
```

Both binaries depend on the same set of library crates. No library crate depends on a binary crate. `scbdb-core` is the leaf dependency with zero internal deps.

## Backend (Rust)

### CLI (`scbdb-cli`)

Rust binary built with **clap** for command parsing. Single entry point with subcommands for collection, reporting, legislative queries, and competitor management.

### API Server (`scbdb-server`)

**Axum**-based HTTP server exposing a REST API consumed by the frontend. Serves product data, competitor listings, legislative tracking, market sentiment, and scrape status.

### Data Collection

#### Shopify Scraper (`scbdb-scraper`)

Custom scraper that fetches `{domain}/products.json` from Shopify-powered competitor storefronts. Handles pagination, normalizes the Shopify product/variant schema into the internal product model, and persists structured data to the database.

This is the primary ingestion path — most tracked brands run Shopify storefronts (see `brands.yaml` for shop URLs).

#### LegiScan Extraction (`scbdb-legiscan`)

Integration with the LegiScan API for tracking cannabis-related legislation. Ingests bills, amendments, and vote records into the database.

### Market Sentiment (`scbdb-sentiment`)

Pipeline for aggregating and scoring market sentiment signals alongside product and legislative data.

## Frontend (`web/`)

**Vite**-powered **React 19** SPA styled with **Tailwind CSS 4+** and **shadcn/ui** components. Communicates with the Axum backend over REST. Provides dashboards for browsing competitor products, tracking legislation, comparing pricing, viewing market sentiment, and monitoring scrape runs.
