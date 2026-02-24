# CLAUDE.md — scbdb-server

## Purpose

`scbdb-server` is the Axum HTTP API server binary for SCBDB. It exposes all REST
endpoints consumed by the web frontend and CLI, runs the background scheduler, and
applies pending database migrations automatically on startup.

Binary name: `scbdb-server`
Entry point: `src/main.rs`
Default bind address: `0.0.0.0:3000` (override with `SCBDB_BIND_ADDR`)

---

## Startup Sequence

`main()` performs these steps in order:

1. `dotenvy::dotenv().ok()` — load `.env` (only this binary may call this; library crates must not)
2. `scbdb_core::load_app_config()` — parse env vars into `AppConfig`
3. Initialize `tracing_subscriber` with `RUST_LOG` or `config.log_level`
4. `scbdb_db::connect_pool()` — establish sqlx PgPool using `PoolConfig`
5. `scbdb_db::run_migrations()` — apply pending migrations automatically
6. `scheduler::build_scheduler()` — start background cron jobs (return value must be held; dropping it kills the scheduler)
7. `AuthState::from_env()` — configure bearer auth from `SCBDB_API_KEYS`
8. `build_app()` — assemble the Axum router with all middleware
9. `axum::serve()` with graceful shutdown on SIGINT / SIGTERM

---

## Environment Variables

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `POSTGRES_PASSWORD` | Yes | — | Password component (used by Docker / sqlx) |
| `SCBDB_BIND_ADDR` | No | `0.0.0.0:3000` | TCP bind address |
| `SCBDB_ENV` | No | `development` | `"development"` disables mandatory auth; any other value (e.g. `"production"`) requires `SCBDB_API_KEYS` to be set |
| `SCBDB_LOG_LEVEL` | No | `info` | Tracing filter (overridden by `RUST_LOG`) |
| `SCBDB_API_KEYS` | Conditional | — | Comma-separated bearer tokens; **required outside development** |
| `TEI_URL` | No | `""` | Text Embeddings Inference endpoint for brand profiler |
| `YOUTUBE_API_KEY` | No | — | YouTube Data API key for brand intake scheduler job |
| `BRAND_INTAKE_CRON` | No | `0 0 6 * * *` | Override cron for brand intake job |
| `RUST_LOG` | No | — | Standard tracing filter (takes priority over `SCBDB_LOG_LEVEL`) |

---

## Authentication (`SCBDB_API_KEYS`)

Implemented in `src/middleware.rs` as `AuthState`.

- `SCBDB_API_KEYS` is a comma-separated list of plaintext bearer tokens.
- When **unset** in `SCBDB_ENV=development`: auth is disabled, all endpoints open. Logs a warning.
- When **unset** outside development: startup fails with an error.
- When **set**: all non-`/api/v1/health` endpoints require `Authorization: Bearer <token>`.
- Token comparison is via `HashSet` lookup (not constant-time — see `SECURITY TODO` in `middleware.rs`).
- Rate limiting keys off the authenticated token when auth is enabled; falls back to `X-Forwarded-For` or `"global"`.

---

## Middleware Stack

Assembled in `src/api/mod.rs` via Tower `ServiceBuilder`. Layer ordering matters:
**in `ServiceBuilder`, the LAST `.layer()` is outermost (runs first)**.

### Outer layer (applied to all routes via `build_app`)
```
ServiceBuilder
  .layer(CorsLayer)            ← runs last in response, first in request
  .layer(request_id)           ← runs first for all requests (outermost)
```
`request_id` middleware: reads `x-request-id` header if present, otherwise generates a UUIDv4.
Stores as a request extension (`RequestId`) and echoes on the response header.

### Inner layer (applied only to protected routes via `protected_router`)
```
ServiceBuilder
  .layer(enforce_rate_limit)   ← outermost for protected routes
  .layer(require_bearer_auth)  ← innermost (auth checked before rate limit counts)
```
Rate limit: 120 requests per 60-second window per client (token or IP).

### Public route
`GET /api/v1/health` — exempt from auth and rate limiting.

---

## Route Table

All routes live under `/api/v1/`. The `{slug}` and `{bill_id}` segments are path parameters.
`bill_id` is a UUID string.

### Public

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| `GET` | `/api/v1/health` | `health` | DB connectivity check |

### Products

| Method | Path | Handler | Query Params |
|--------|------|---------|--------------|
| `GET` | `/api/v1/products` | `products::list_products` | `brand_slug`, `relationship`, `tier`, `limit` |

### Pricing

| Method | Path | Handler | Query Params |
|--------|------|---------|--------------|
| `GET` | `/api/v1/pricing/snapshots` | `pricing::list_pricing_snapshots` | `brand_slug`, `from`, `to`, `limit` |
| `GET` | `/api/v1/pricing/summary` | `pricing::list_pricing_summary` | — |

### Regulatory (Bills)

| Method | Path | Handler | Query / Path Params |
|--------|------|---------|---------------------|
| `GET` | `/api/v1/bills` | `bills::list_bills` | `jurisdiction`, `limit` |
| `GET` | `/api/v1/bills/{bill_id}/events` | `bills::list_bill_events` | `bill_id` (UUID) |
| `GET` | `/api/v1/bills/{bill_id}/texts` | `bills::list_bill_texts` | `bill_id` (UUID) |

### Sentiment

| Method | Path | Handler | Query Params |
|--------|------|---------|--------------|
| `GET` | `/api/v1/sentiment/summary` | `sentiment::list_sentiment_summary` | — |
| `GET` | `/api/v1/sentiment/snapshots` | `sentiment::list_sentiment_snapshots` | `limit` |

### Locations

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| `GET` | `/api/v1/locations/summary` | `locations::list_locations_summary` | Per-brand active count, states, weekly growth |
| `GET` | `/api/v1/locations/by-state` | `locations::list_locations_by_state` | Brand + location counts per state |
| `GET` | `/api/v1/locations/pins` | `locations::list_location_pins` | Lat/lon + metadata for map rendering |

### Brands — Read

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| `GET` | `/api/v1/brands` | `brands::list_brands` | Returns completeness scores via parallel DB calls |
| `GET` | `/api/v1/brands/{slug}` | `brands::get_brand` | Full profile incl. social handles, domains, completeness |
| `GET` | `/api/v1/brands/{slug}/signals` | `brands::list_brand_signals` | Cursor-paginated; query: `type`, `limit`, `cursor` |
| `GET` | `/api/v1/brands/{slug}/funding` | `brands::list_funding` | Funding events |
| `GET` | `/api/v1/brands/{slug}/lab-tests` | `brands::list_lab_tests` | Lab test results |
| `GET` | `/api/v1/brands/{slug}/legal` | `brands::list_legal` | Legal proceedings |
| `GET` | `/api/v1/brands/{slug}/sponsorships` | `brands::list_sponsorships` | Sponsorship deals |
| `GET` | `/api/v1/brands/{slug}/distributors` | `brands::list_distributors` | Distribution relationships |
| `GET` | `/api/v1/brands/{slug}/competitors` | `brands::list_competitors` | Competitor relationships |
| `GET` | `/api/v1/brands/{slug}/media` | `brands::list_media` | Media appearances |

### Brands — Write

| Method | Path | Handler | Body |
|--------|------|---------|------|
| `POST` | `/api/v1/brands` | `brands::create_brand` | `CreateBrandRequest` |
| `PATCH` | `/api/v1/brands/{slug}` | `brands::update_brand` | `UpdateBrandRequest` (sparse PATCH) |
| `DELETE` | `/api/v1/brands/{slug}` | `brands::deactivate_brand` | — (soft-delete) |
| `PUT` | `/api/v1/brands/{slug}/profile` | `brands::upsert_brand_profile` | `UpsertProfileRequest` |
| `PUT` | `/api/v1/brands/{slug}/social` | `brands::upsert_brand_social` | `{ handles: { platform: handle } }` |
| `PUT` | `/api/v1/brands/{slug}/domains` | `brands::upsert_brand_domains` | `{ domains: [url, ...] }` |

---

## Response Envelope

Every successful response uses `ApiResponse<T>`:
```json
{
  "data": <T>,
  "meta": {
    "request_id": "<uuid or forwarded x-request-id>",
    "timestamp": "<RFC3339 UTC>"
  }
}
```

Every error response uses `ApiError`:
```json
{
  "error": {
    "code": "<error_code>",
    "message": "<human message>"
  },
  "meta": { ... }
}
```

Error code → HTTP status mapping (in `ApiError::into_response`):
- `not_found` → 404
- `unauthorized` → 401
- `bad_request` / `validation_error` → 400
- `conflict` → 409
- `rate_limited` → 429
- anything else → 500

Middleware errors (auth/rate-limit) use a separate `MiddlewareErrorBody` struct and do NOT go through `ApiError::into_response` — they are constructed as raw `(StatusCode, Json(...))` tuples.

---

## Pagination

Signals endpoint uses cursor-based pagination (keyed on row `id`):
- `?limit=N` — clamped to 1–200, defaults to 50 (`normalize_limit`)
- `?cursor=<id>` — fetch rows after this id
- Response includes `next_cursor: Option<i64>` (null when no more pages)
- Implementation: fetch `limit + 1` rows; if `len > limit`, there is a next page

All other list endpoints accept `?limit` with the same clamp/default but do not paginate with cursors.

---

## Background Scheduler

`src/scheduler/mod.rs` + `src/scheduler/brand_intel.rs`

The `JobScheduler` handle returned by `build_scheduler()` must be stored for the lifetime of the process. Dropping it cancels all jobs.

| Job | Cron | Description |
|-----|------|-------------|
| Locations | `0 0 2 * * SUN` | Scrape store locations for all brands with `store_locator_url`; upsert new, deactivate missing |
| Brand intake | `0 0 6 * * *` (override: `BRAND_INTAKE_CRON`) | Run profiler intake for brands without a profile (RSS, YouTube, Twitter signals + TEI embeddings) |
| Signal refresh | `0 0 4 * * *` | Re-run intake for brands whose signals are >24 hours stale |
| Handle refresh | `0 0 5 * * SUN` | Log brands with social handles unchecked for 7+ days (full verification not yet implemented) |

Scheduler job failure policy: individual brand failures are logged and skipped; they do not abort the entire batch run.

Sentiment jobs consume `SentimentConfig::from_env()` vars at job execution time — `SENTIMENT_TEI_URL`, `SENTIMENT_QDRANT_URL`, `SENTIMENT_QDRANT_COLLECTION`, `REDDIT_CLIENT_ID`, `REDDIT_CLIENT_SECRET`, `REDDIT_USER_AGENT` (required), plus optional `TWITTER_AUTH_TOKEN` / `TWITTER_CT0`. These are not listed in the server env table above because they are consumed by `scbdb-sentiment`, not by this binary directly.

Location job safety: if a scrape returns 0 locations or fails trust validation, the job skips upsert and deactivation for that brand entirely (partial results are treated as a transient failure, not as evidence that locations disappeared).

---

## Source Layout

```
src/
├── main.rs                     # Binary entry point, startup sequence
├── middleware.rs               # AuthState, RateLimitState, request_id fn, require_bearer_auth fn, enforce_rate_limit fn
├── api/
│   ├── mod.rs                  # AppState, ApiResponse, ApiError, build_app, protected_router, CORS, health handler
│   ├── products.rs             # GET /products
│   ├── pricing.rs              # GET /pricing/snapshots + /pricing/summary
│   ├── bills.rs                # GET /bills + /bills/{id}/events + /bills/{id}/texts
│   ├── sentiment.rs            # GET /sentiment/summary + /sentiment/snapshots
│   ├── locations.rs            # GET /locations/summary + /by-state + /pins
│   └── brands/
│       ├── mod.rs              # Re-exports, resolve_brand helper
│       ├── list.rs             # GET /brands
│       ├── detail.rs           # GET /brands/{slug}
│       ├── signals.rs          # GET /brands/{slug}/signals (cursor-paginated)
│       ├── write.rs            # POST /brands, PATCH /brands/{slug}, DELETE /brands/{slug}
│       ├── write_enrichment.rs # PUT /brands/{slug}/profile|social|domains
│       └── intel/
│           ├── mod.rs          # GET /brands/{slug}/funding|lab-tests|legal|sponsorships|distributors|competitors|media
│           └── types.rs        # Response item structs for the intel endpoints
└── scheduler/
    ├── mod.rs                  # build_scheduler, locations job
    └── brand_intel.rs          # brand_intake, signal_refresh, handle_refresh jobs
```

---

## Workspace Crate Dependencies

| Crate | What it provides to scbdb-server |
|-------|----------------------------------|
| `scbdb-core` | `load_app_config()`, `AppConfig`, `Environment`, `brands::slug_from_name` |
| `scbdb-db` | `connect_pool`, `run_migrations`, `health_check`, all query functions, `BrandRow`, `NewStoreLocation`, `DbError` |
| `scbdb-scraper` | `fetch_store_locations`, `validate_store_locations_trust`, `make_location_key` |
| `scbdb-profiler` | `IntakeConfig`, `intake::ingest_signals` |

---

## Build and Run in Isolation

```bash
# From workspace root
cargo build -p scbdb-server
cargo run -p scbdb-server

# Or via justfile
just serve          # starts API server + Vite dev server together
just bootstrap      # db-up → migrate → ping → seed (run first)
```

Direct server start (no justfile):
```bash
DATABASE_URL=postgres://scbdb:password@localhost:15432/scbdb \
SCBDB_BIND_ADDR=0.0.0.0:3000 \
cargo run -p scbdb-server
```

Health check:
```bash
curl http://localhost:3000/api/v1/health
```

---

## Testing

Tests are inline (`#[cfg(test)]`) in `src/api/mod.rs` and `src/middleware.rs`.

Integration tests use `#[sqlx::test(migrations = "../../migrations")]` — sqlx provisions a
temporary database per test, applies migrations, and tears it down automatically.

Unit tests (no DB) test serialization, `normalize_limit`, `ApiError` status mapping, and
bearer token extraction.

Run all tests:
```bash
cargo test -p scbdb-server
```

For integration tests, `DATABASE_URL` must point to a running PostgreSQL instance (sqlx
creates test databases on it). The justfile `just test` handles this.

---

## Validation Conventions

Brand write handlers validate inline (no external validator crate):
- `relationship` must be `"portfolio"` or `"competitor"`
- `tier` must be `1`, `2`, or `3`
- URL fields validated via `reqwest::Url::parse`
- Unique constraint violations on slug (PG error `23505`) mapped to `conflict` / 409

`PATCH /brands/{slug}` uses `Option<Option<T>>` for nullable fields:
- `None` = field not in request body (keep current value)
- `Some(None)` = explicitly clear the field
- `Some(Some(v))` = set to value

---

## Gotchas

- **Scheduler handle lifetime** — `build_scheduler` returns a `JobScheduler`. Assigning it to `_` would immediately drop it and kill all jobs. It must be bound to a named variable (`let _scheduler = ...`) that lives for the process lifetime.
- **Middleware ordering** — In `ServiceBuilder`, layers are applied inside-out. The last `.layer()` is the outermost wrapper (runs first on inbound, last on outbound). Auth and rate-limit are applied per-router (protected only); CORS and request_id are applied globally.
- **Auth in tests** — Integration tests call `AuthState::from_env(true)` (passing `is_development = true`) so missing `SCBDB_API_KEYS` does not fail the test runner.
- **`dotenvy` policy** — Only `main.rs` calls `dotenvy::dotenv()`. Library crates (`scbdb-db`, `scbdb-core`, etc.) must never call it.
- **Locations partial-scrape guard** — The scheduler skips upsert and deactivation if a scrape returns 0 locations. This is intentional — an empty result is treated as a transient failure, not as proof that all locations closed.
- **Signal cursor pagination** — The `next_cursor` value is the `id` of the last item returned, not an offset. Pass it as `?cursor=<id>` on the next request.
- **`normalize_limit`** — Shared helper in `api/mod.rs`. Defaults to 50, clamps to 1–200. All list endpoints use it.
