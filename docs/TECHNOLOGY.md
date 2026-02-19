# Technology Stack

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

## Backend

- **Rust** — Primary language for backend, CLI, collectors, and data pipeline
- **tokio** — Async runtime underpinning Axum, sqlx, reqwest, and all async code. Use the full feature set (`features = ["full"]`).
- **Axum** — HTTP framework for the REST/API layer
- **clap** — Argument parsing and command routing for the CLI
- **serde** / **serde_json** — Serialization and deserialization for all JSON: Shopify responses, LegiScan payloads, Axum request/response bodies, config files. Derive `Serialize` and `Deserialize` on all domain types in `scbdb-core`.
- **reqwest** — HTTP client for Shopify scraper and LegiScan API. Use with `rustls-tls` feature (no OpenSSL dependency).
- **dotenvy** — Loads `.env` files into the process environment at startup. Used in both `scbdb-cli` and `scbdb-server` to resolve `DATABASE_URL` and API keys.
- **chrono** — Date/time handling for `TIMESTAMPTZ` columns, Shopify timestamps, and LegiScan dates. Has native sqlx and serde integration (`features = ["serde"]`).
- **uuid** — UUID generation for external-facing identifiers. Sequential `BIGINT` primary keys stay internal; public API responses use UUIDs to avoid exposing row counts and ordering.
- **tower** `RateLimitLayer` — Server-side rate limiting middleware for the Axum API. Applied globally or per-route via tower's layered middleware stack (already using tower-http for tracing).
- **tokio-cron-scheduler** (optional, post-MVP) — In-process job scheduling for recurring scrape runs. For MVP, external cron/systemd timers are preferred.

## Database

- **PostgreSQL** — Primary data store for all persistent data (products, brands, legislation, sentiment scores, scrape history)
- **sqlx** — Compile-time checked async Rust driver for PostgreSQL (no ORM — raw SQL with type safety)

## Frontend

- **Vite** — Dev server and build tooling
- **React 19** — UI framework
- **TypeScript** — Frontend language
- **Tailwind CSS 4+** — Utility-first styling
- **shadcn/ui** — Component library (built on Radix UI primitives)
- **TanStack Router** — Type-safe client-side routing with file-based route generation. Provides full type inference for route params, search params, and loaders.
- **TanStack Query** (React Query) — Server state management: caching, background refetching, optimistic updates, and loading/error states. All API data flows through TanStack Query hooks — no `useEffect` + `useState` fetch patterns.

## Data Collection

- **Custom Shopify scraper** — Purpose-built HTTP client that pulls `products.json` from Shopify storefronts and normalizes the response into the internal product schema
- **LegiScan API** — Legislative data extraction for cannabis-related bills and votes

## Post-MVP Planned Components

- **Spider** — fallback crawling for non-Shopify sources
- **Qdrant** — semantic vector retrieval index
- **TEI** — embedding generation service

## Analysis

- **Market sentiment pipeline** — Aggregation and scoring of market signals

## Authentication

- **API key authentication** — All API endpoints require a valid API key passed via the `Authorization: Bearer <key>` header. Keys are stored as hashed values in the database. No session management, no JWTs — stateless per-request validation.
- Key generation and management is handled through `scbdb-cli` subcommands.

## Networking & Middleware

- **tower-http `CorsLayer`** — CORS middleware for the Axum server. Required in development (Vite on `:5173`, Axum on `:3000`). Configured per-environment: permissive in dev, restrictive in production.
- **tower `RateLimitLayer`** — Request rate limiting applied at the Axum router level. Prevents API abuse before deployment goes public.

## Error Handling

- **thiserror** — Derive macros for custom error enums in library crates (`scbdb-core`, `scbdb-scraper`, `scbdb-db`, etc.)
- **anyhow** — Convenient error propagation in binary crates only (`scbdb-cli`, `scbdb-server`). Not used in libraries.

## Logging & Tracing

- **tracing** — Structured, span-based instrumentation for async Rust. The standard for tokio/axum applications.
- **tracing-subscriber** — Configures output format, filtering (`RUST_LOG`), and log levels.
- **tower-http TraceLayer** — Axum middleware for automatic HTTP request/response tracing.

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

## Testing

### Rust

- **cargo test** — Built-in test runner for unit and integration tests
- **tokio::test** — Async test runtime for anything touching sqlx, reqwest, or axum handlers
- **wiremock** — HTTP mock server for testing Shopify/LegiScan clients without hitting real APIs
- **sqlx::test** — `#[sqlx::test]` macro gives each test a clean, migrated PostgreSQL database — no manual setup/teardown

### Frontend

- **Vitest** — Test runner for all frontend unit and component tests
- **React Testing Library** — Component behavior testing — asserts on what users see, not implementation details

## Migrations

- **sqlx migrate** — Built-in sqlx migration runner. Timestamped `.sql` files in `migrations/`. No separate migration tool needed.

## Pre-commit

- **lefthook** — Git hooks manager. Single Go binary, no runtime dependencies. Runs Rust and Node checks in parallel.

## Containerization

- **Docker Compose** — Local development environment. Runs PostgreSQL  with a single `docker compose up`. No manual database installation required.
- **Dockerfile** — Multi-stage build for the Rust backend: build stage compiles release binaries, runtime stage uses a minimal base image. Frontend is built separately and served as static assets.

## CI/CD

- **GitHub Actions** — Continuous integration pipeline triggered on pull requests and pushes to `main`. Runs the same checks as lefthook pre-commit plus integration tests against a real PostgreSQL service container:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test --workspace`
  - `cd web && npx tsc --noEmit`
  - `cd web && npx eslint .`
  - `cd web && npx prettier --check .`
  - `cd web && npx vitest run`
