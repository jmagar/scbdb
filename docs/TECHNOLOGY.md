# Technology Stack

## Backend

- **Rust** — Primary language for backend, CLI, collectors, and data pipeline
- **Axum** — HTTP framework for the REST/API layer
- **clap** — Argument parsing and command routing for the CLI

## Database

- **PostgreSQL** — Primary data store for all persistent data (products, brands, legislation, sentiment scores, scrape history)
- **sqlx** — Compile-time checked async Rust driver for PostgreSQL (no ORM — raw SQL with type safety)

## Frontend

- **Vite** — Dev server and build tooling
- **React 19** — UI framework
- **TypeScript** — Frontend language
- **Tailwind CSS 4+** — Utility-first styling
- **shadcn/ui** — Component library (built on Radix UI primitives)

## Data Collection

- **Custom Shopify scraper** — Purpose-built HTTP client that pulls `products.json` from Shopify storefronts and normalizes the response into the internal product schema
- **LegiScan API** — Legislative data extraction for cannabis-related bills and votes

## Analysis

- **Market sentiment pipeline** — Aggregation and scoring of market signals

## Monorepo & Task Running

- **Cargo workspace** — Manages all Rust crates as a single workspace with shared dependencies
- **just** — Command runner for orchestrating cross-language tasks (build, dev, test, check, migrate)

## Linting

- **clippy** — Rust linter, run with `-D warnings` (all warnings are errors)
- **ESLint 9+** (flat config) — TypeScript/React linter with `eslint-plugin-react-hooks` and `eslint-plugin-jsx-a11y`

## Type Checking

- **rustc** — The Rust compiler is the type checker; `cargo check` for fast feedback
- **tsc --noEmit** — Explicit TypeScript type checking (Vite strips types without checking them; this must run separately in CI)

## Formatting

- **rustfmt** — Rust formatter via `cargo fmt --all` — zero configuration, one canonical style
- **Prettier** — Formats TypeScript, TSX, JSON, Markdown, and CSS across `web/` and root docs
