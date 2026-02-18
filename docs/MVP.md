# MVP Scope

> **Note:** The original PRD (Section 5.2) described a TypeScript/Node.js stack. The project
> has been redesigned as a Rust/Cargo workspace with a React frontend. This document
> reflects the current Rust-based architecture. See [TECHNOLOGY.md](TECHNOLOGY.md) and
> [ARCHITECTURE.md](ARCHITECTURE.md) for the authoritative stack reference.

## Phases

### Phase 1 — Full Repo Scaffold

Monorepo structure, Cargo workspace, Vite frontend scaffold, CI config, shared types, database schema, and `brands.yaml` seed data.

**Deliverables:**

- [ ] Cargo workspace with all 7 crates stubbed (`scbdb-cli`, `scbdb-server`, `scbdb-core`, `scbdb-db`, `scbdb-scraper`, `scbdb-legiscan`, `scbdb-sentiment`)
- [ ] `scbdb-core` with domain model structs and error types (Product, Competitor, Bill, etc.)
- [ ] `scbdb-db` with sqlx connection pool setup and initial migration (competitors, products, price_history, bills, bill_events, collection_log tables)
- [ ] `scbdb-cli` skeleton with `competitors list` / `competitors add` / `competitors show` subcommands
- [ ] Vite + React 19 + Tailwind CSS 4 + shadcn/ui scaffold in `web/`
- [ ] TanStack Router and TanStack Query configured
- [ ] `docker-compose.yml` for local PostgreSQL
- [ ] `.env.example` with required environment variables
- [ ] `justfile` with `build`, `dev`, `test`, `check`, `migrate` tasks
- [ ] `lefthook.yml` with pre-commit checks (fmt, clippy, eslint, tsc, prettier)
- [ ] GitHub Actions CI pipeline (`.github/workflows/ci.yml`)
- [ ] `Dockerfile` with multi-stage Rust build

**Acceptance criteria:** `cargo build --workspace` compiles. `cargo test --workspace` passes. `docker compose up -d && sqlx migrate run` creates all tables. `cargo run -p scbdb-cli -- competitors list` returns brands from `brands.yaml`.

---

### Phase 2 — Rust CLI (Shopify Scraper)

clap-based CLI with subcommands for scraping, brand management, and data export. Shopify `products.json` collector with pagination and normalization.

**Deliverables:**

- [ ] `scbdb-scraper` — HTTP client that fetches `{domain}/products.json`, handles pagination (page=1,2,...N), and normalizes Shopify product/variant schema into internal Product model
- [ ] Rate limiting and backoff for scraper requests (tower `RateLimitLayer` or custom)
- [ ] CLI subcommands: `collect products --competitor <slug>`, `collect products --all`, `collect products --tier <n>`
- [ ] Pricing snapshot: `collect pricing --competitor <slug>` records current prices to `price_history`
- [ ] Data export: `report competitive --format csv` (basic product matrix)
- [ ] Collection logging — every scrape run recorded in `collection_log` with status, record count, and duration
- [ ] wiremock-based tests for Shopify scraper (pagination, error handling, normalization)
- [ ] `#[sqlx::test]` integration tests for product persistence

**Acceptance criteria:** `cargo run -p scbdb-cli -- collect products --tier 1` scrapes all Tier 1 brands, normalizes products into the database, and logs the collection run. `cargo test --workspace` passes including scraper and database tests.

---

### Phase 3 — LegiScan Extraction

LegiScan API integration for tracking cannabis-related legislation. Ingest bills, amendments, and vote data into the database. CLI subcommands for querying legislative status.

**Deliverables:**

- [ ] `scbdb-legiscan` — LegiScan API client with bill search, bill detail, amendment, and vote endpoints
- [ ] Regulatory collector: `collect regs` polls LegiScan for all tracked SC + federal bills
- [ ] Bill ingestion — maps LegiScan responses to internal Bill/BillEvent models and persists to database
- [ ] CLI subcommands: `regs status` (all tracked bills), `regs show <bill-number>` (detail), `regs timeline` (chronological events)
- [ ] Regulatory status markdown report: `report regulatory --format markdown`
- [ ] wiremock-based tests for LegiScan client
- [ ] `#[sqlx::test]` integration tests for bill persistence and querying

**Acceptance criteria:** `cargo run -p scbdb-cli -- collect regs` ingests all tracked SC bills from LegiScan. `cargo run -p scbdb-cli -- regs status` displays a table of bill statuses. `cargo run -p scbdb-cli -- report regulatory --format markdown` generates a readable summary.

---

### Phase 4 — Market Sentiment

Sentiment analysis pipeline for market signals. Aggregate and score sentiment data alongside product and legislative data.

> **Design note:** Data sources and scoring methodology for the sentiment pipeline are not
> yet specified. Before implementation, define: (1) what data sources to consume (news APIs,
> RSS feeds, Reddit, social media), (2) what "sentiment score" means in this context
> (positive/negative brand mentions, market trend indicators, regulatory risk signals), and
> (3) how scores connect to the CLI and frontend.

**Deliverables:**

- [ ] Define sentiment data sources and scoring methodology (design doc)
- [ ] `scbdb-sentiment` — ingestion and scoring pipeline
- [ ] CLI subcommands: `collect sentiment --brand <slug>`, `sentiment show <slug>`
- [ ] Sentiment data persisted to database (new table or extension of existing schema)
- [ ] Tests for sentiment scoring logic

**Acceptance criteria:** Sentiment data is collected, scored, and queryable via CLI. Scoring methodology is documented.

---

### Phase 5 — Web Dashboard

Axum API server + Vite/React 19 frontend. shadcn/ui dashboards for browsing competitor products, tracking legislation, and viewing market sentiment.

> **Note:** The PRD lists the web dashboard as a non-goal for v1. This phase is included
> here for completeness but is explicitly deferred until the CLI-first workflow is validated
> with the team.

**Deliverables:**

- [ ] `scbdb-server` — Axum REST API exposing products, competitors, bills, sentiment, and collection status
- [ ] API key authentication middleware (Bearer token, hashed keys in database)
- [ ] CORS and rate limiting middleware (tower-http)
- [ ] Frontend pages: competitor list, competitor detail (product table), bill tracker, sentiment dashboard
- [ ] Mobile-first responsive layouts (see [DEVELOPMENT.md](DEVELOPMENT.md) for responsive rules)
- [ ] TanStack Query hooks for all API endpoints
- [ ] Vitest + React Testing Library tests for components
- [ ] API integration tests for Axum handlers

**Acceptance criteria:** `cargo run -p scbdb-server` serves the API. `cd web && npm run dev` renders the dashboard. Product data, bill statuses, and sentiment scores are visible in the browser. All tests pass.

---

## Features Deferred from PRD

The following features from the original PRD are **not in scope** for the current MVP phases. They may be revisited after the core CLI and dashboard are validated:

| Feature | PRD Section | Reason Deferred |
|---|---|---|
| Qdrant vector search / semantic `ask` commands | 5.1, 5.4, 8.1 | Replaced by direct SQL queries in the Rust redesign; may be revisited if semantic search adds value beyond structured queries |
| TEI embeddings server | 5.2 | Dependent on Qdrant; deferred with it |
| Spider web scraping fallback | 5.2, 7 | Most tracked brands use Shopify; non-Shopify brands can be handled with targeted scraping if needed |
| XLSX report generation | 5.2, 8.2 | No Rust equivalent to ExcelJS identified yet; CSV and markdown reports cover initial needs |
| Brand metadata extraction (company info schema) | 7.2 | Secondary to product catalog data; can be added after core scraping works |
| Automated scheduling (node-cron / tokio-cron-scheduler) | 8.1, 11 | External cron/systemd timers are simpler for a homelab deployment; in-process scheduling is over-engineering for v1 |
| Email/Slack report delivery | 11 Phase 4 | Deferred until team adoption validates the report formats |
| Multi-collection Qdrant strategy | 5.4 | Deferred with Qdrant |
