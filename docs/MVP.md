# MVP Scope

## Phases

### Phase 1 — Full Repo Scaffold

Monorepo structure, Cargo workspace, Vite frontend scaffold, CI config, shared types, database schema, and `brands.yaml` seed data.

### Phase 2 — Rust CLI

clap-based CLI with subcommands for scraping, brand management, and data export. Shopify `products.json` collector with pagination and normalization.

### Phase 3 — LegiScan Extraction

LegiScan API integration for tracking cannabis-related legislation. Ingest bills, amendments, and vote data into the database. CLI subcommands for querying legislative status.

### Phase 4 — Market Sentiment

Sentiment analysis pipeline for market signals. Aggregate and score sentiment data alongside product and legislative data.

### Phase 5 — Web Dashboard

Axum API server + Vite/React 19 frontend. shadcn/ui dashboards for browsing competitor products, tracking legislation, and viewing market sentiment.
