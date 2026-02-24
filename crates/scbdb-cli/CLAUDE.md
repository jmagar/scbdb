# CLAUDE.md — scbdb-cli

## Purpose

`scbdb-cli` is the primary operator interface for SCBDB. It is the only binary that calls `dotenvy::dotenv()` to load `.env` on startup — no library crate in the workspace may do this. All operational workflows (data collection, regulatory ingestion, sentiment analysis, database management) are driven from here.

## Entry Point

`src/main.rs` — defines the top-level `Cli` struct (clap `Parser`) and the `Commands` enum. Startup sequence:

1. `dotenvy::dotenv().ok()` — load `.env` (failures are silently ignored; missing vars surface later)
2. `tracing_subscriber` init via `RUST_LOG` or `SCBDB_LOG_LEVEL` (default: `info`)
3. `Cli::parse()` — clap parses argv
4. `load_config_or_exit()` / `connect_or_exit()` — config and DB pool created before any subcommand executes; failure prints a human-readable hint and calls `std::process::exit(1)`

Two shared helpers live directly in `main.rs` and are used across all submodules:

- `load_config_or_exit() -> AppConfig` — calls `scbdb_core::load_app_config()`, exits on error
- `connect_or_exit() -> PgPool` — calls `scbdb_db::connect_pool_from_env()`, prints actionable hints per `DbError` variant, exits on error
- `fail_run_best_effort(pool, run_id, context, message)` — marks a collection run as failed; logs any secondary error and swallows it (never propagated)

## Module Map

```
src/
├── main.rs                    # CLI definition, startup, Db subcommands inline
├── tests.rs                   # Root-level parser unit tests (#[cfg(test)] mod tests)
├── collect/
│   ├── mod.rs                 # CollectCommands enum + run_collect_* public fns
│   ├── runner.rs              # BrandOutcome, CollectionTotals, run_collection skeleton
│   ├── verify_images.rs       # run_collect_verify_images — concurrent HEAD/GET checks
│   ├── brand/
│   │   ├── mod.rs             # build_shopify_client, persist_normalized_products,
│   │   │                      #   collect_brand_products, collect_brand_pricing
│   │   ├── pipeline.rs        # collect_brand_core — fetch→logo→normalize→filter→persist
│   │   └── (brand_test.rs)    # #[sqlx::test] integration tests (path alias)
│   ├── locations/
│   │   ├── mod.rs             # run_collect_locations entry point, BrandLocationOutcome
│   │   ├── brand.rs           # collect_brand_locations — resolve→scrape→validate→upsert
│   │   ├── helpers.rs         # load_brands_for_locations, record_brand_failure,
│   │   │                      #   log_location_changeset, raw_to_new_location
│   │   └── url.rs             # resolve_locator_url, discover_locator_url, LOCATOR_PATHS
│   └── (collect_test.rs)      # #[sqlx::test] integration tests (path alias)
├── regs/
│   ├── mod.rs                 # RegsCommands enum, fmt_date helper
│   ├── discovery.rs           # discover_candidates — getMasterList / all-sessions backfill
│   ├── ingest.rs              # run_regs_ingest — 3-phase pipeline (discover/hash/fetch)
│   └── query.rs               # run_regs_status, run_regs_timeline, run_regs_report
└── sentiment/
    ├── mod.rs                 # SentimentCommands enum, run_sentiment_collect,
    │                          #   load_brands_for_sentiment, select_brand_base_url
    └── query.rs               # run_sentiment_status, run_sentiment_report
```

## Subcommands

### `collect products`

Fetches Shopify `products.json` for all active brands (or one with `--brand <slug>`), normalizes, and upserts products + variants + price snapshots.

Flags: `--brand <slug>` (optional), `--dry-run` (prints target brands, no DB writes)

Eligibility filter: brands must have `shop_url` set; brands without it are skipped with a warning (or rejected with an error when `--brand` targets one specifically).

### `collect pricing`

Fetches current Shopify catalog, upserts any new products/variants encountered, and records `price_snapshots` rows only for variants whose price changed since the last snapshot.

Flags: `--brand <slug>` (optional)

### `collect verify-images`

Checks HTTP 200 reachability of all product `primary_image_url` and brand `logo_url` values in the database. Non-200s are logged as warnings. Does not update any rows.

Flags: `--brand <slug>` (optional), `--concurrency <n>` (default: 12)

### `collect locations`

Collects retail store locations for all active brands. For brands without a configured `store_locator_url`, auto-discovery probes common URL paths (defined in `LOCATOR_PATHS` in `url.rs`). Active locations are upserted; locations absent from the current scrape are deactivated.

Flags: `--brand <slug>` (optional), `--dry-run` (prints target brands + URL strategy, no DB writes)

### `regs ingest`

Three-phase pipeline: (1) discover candidates via `getMasterList`, (2) hash-check against stored bills to skip unchanged ones, (3) fetch + upsert only new/changed bills. Operates within a configurable API request budget.

Flags:
- `--state <STATE>` (repeatable, default: `SC`; use `US` for Congress)
- `--keyword <KW>` (repeatable, default: `hemp`)
- `--max-pages <n>` (default: 3, currently unused — discovery uses MasterList, not search pages)
- `--max-requests <n>` (default: 5000; protects the 30k/month LegiScan quota)
- `--all-sessions` (backfill all historical sessions, not just current)
- `--dry-run`

Requires: `LEGISCAN_API_KEY` in environment.

### `regs status`

Prints a table of tracked bills. Flags: `--state <STATE>`, `--limit <n>` (default: 20)

### `regs timeline`

Prints chronological event history for a bill. Required flags: `--state <STATE>`, `--bill <BILL_NUMBER>` (e.g., `HB1234`)

### `regs report`

Generates a markdown regulatory report to stdout. Flag: `--state <STATE>`

### `sentiment collect`

Collects signals (Google News RSS, Reddit) for each brand, embeds via TEI, deduplicates in Qdrant, scores with the lexicon, and persists a `sentiment_snapshots` row.

Flags: `--brand <slug>` (optional), `--dry-run`

Eligibility: any active brand (no `shop_url` required, unlike product collection). `select_brand_base_url` prefers `domain` over `shop_url` when both are set.

Requires: `SCBDB_TEI_URL`, `SCBDB_QDRANT_URL` (or their equivalents consumed by `scbdb_sentiment::SentimentConfig::from_env()`).

### `sentiment status`

Prints recent sentiment snapshots. Flag: `--brand <slug>`

### `sentiment report`

Generates a markdown sentiment report to stdout. Flag: `--brand <slug>`

### `db ping`

Calls `scbdb_db::health_check()` to verify database connectivity.

### `db migrate`

Applies pending sqlx migrations. Prints count applied.

### `db seed`

Seeds brands from `config/brands.yaml` (path from `AppConfig.brands_path` / `SCBDB_BRANDS_PATH`).

### `report`

Not implemented. Exits with code 1 and an error message. (Phase 5 placeholder.)

## Key Patterns

### Collection Run Lifecycle

Every data-collection subcommand (products, pricing, locations, sentiment, regs) follows this state machine:

```
create_collection_run → start_collection_run → [brand loop] → complete_collection_run
                                            ↘ fail_collection_run (on fatal error)
```

Per-brand failures are logged and counted but do not abort the run unless **all** brands fail (in which case the run is marked failed and an error is returned). The `fail_run_best_effort` helper in `main.rs` is the canonical way to mark a run failed; it swallows secondary errors to avoid masking the original.

### Brand Outcome Semantics (`BrandOutcome`)

`BrandOutcome::Ok { succeeded: false }` means orchestration succeeded but the underlying collection for that brand failed (e.g. network error). The brand's failure was already recorded in `collection_run_brands` by `collect_brand_core`. The outer runner must not write a second status row.

`BrandOutcome::Ok { succeeded: true }` means the brand completed cleanly; the caller records the success row.

### 403 Fallback for Known Storefronts

`KNOWN_403_FALLBACK_BRANDS` in `pipeline.rs` lists brand slugs that return HTTP 403 on the standard user-agent. These receive a browser-profile retry via `ShopifyClient::fetch_all_products_browser_profile`. Currently hardcoded (`["cycling-frog"]`). Adding a new 403 brand requires a code change; the ideal fix is a flag in `config/brands.yaml` (tracked as a TODO in `pipeline.rs`).

### Beverage Filter

After Shopify normalization, `pipeline.rs` drops products with no variant having `dosage_mg` or `size_value`. This excludes merch, accessories, gift cards, and insurance items published alongside drink catalogs.

### Location Auto-Discovery

When `brands.store_locator_url` is NULL, `url.rs` probes `LOCATOR_PATHS` against the brand's `domain` with a 5-second HEAD timeout. Discovered URLs are **not** persisted (tracked as `P1 TODO` in `url.rs`), so auto-discovery reruns on every collection cycle for those brands.

### `collection_run_brands.error_message` Dual-Use

The `error_message` column is reused for informational notes on partial successes. A `[NOTE]` prefix distinguishes notes from actual errors (e.g., `[NOTE] browser-profile fallback succeeded`). No dedicated `note` column exists in the schema.

### LegiScan Request Budget

The `max_requests` counter tracks every `getMasterList` and `getBill` call. When the budget is reached, discovery stops early and the run is marked succeeded with whatever was collected. `LegiscanError::QuotaExceeded` (API-level quota, not the local budget) aborts immediately as an error.

### `fmt_date` in `regs/mod.rs`

Returns `"—"` (em-dash, `\u{2014}`) for `None` dates. Referenced by both `query.rs` and `mod.rs` display code.

### `select_brand_base_url` in `sentiment/mod.rs`

Prefers `brand.domain` over `brand.shop_url`; returns `None` when both are empty or unset. Tested inline in `sentiment/mod.rs` tests.

## Workspace Dependencies

| Crate | What this crate uses from it |
|-------|------------------------------|
| `scbdb-core` | `AppConfig`, `load_app_config()`, `load_brands()`, `NormalizedProduct`, `NormalizedVariant`, `Environment` |
| `scbdb-db` | `PgPool`, `connect_pool_from_env()`, `connect_pool()`, `PoolConfig`, `BrandRow`, `NewStoreLocation`, `DbError`, collection run lifecycle fns, all upsert/query fns, `SentimentSnapshotRow` |
| `scbdb-scraper` | `ShopifyClient`, `fetch_brand_logo_url()`, `normalize_product()`, `fetch_store_locations()`, `validate_store_locations_trust()`, `make_location_key()`, `RawStoreLocation`, `ScraperError` |
| `scbdb-legiscan` | `LegiscanClient`, `LegiscanError`, `normalize_bill()`, `normalize_bill_events()`, `normalize_bill_texts()`, `MasterListEntry` |
| `scbdb-sentiment` | `SentimentConfig`, `run_brand_sentiment()` |

## Build and Test

```bash
# Build this crate only
cargo build --bin scbdb-cli

# Build workspace (all crates)
just build

# Run all tests (Rust + web)
just test

# Run only this crate's tests (unit + integration)
cargo test -p scbdb-cli

# Run only unit/parser tests (no DB required)
cargo test -p scbdb-cli --lib

# Run integration tests (requires live DB via DATABASE_URL)
cargo test -p scbdb-cli --test '*'

# Lint and format check
just check                                        # full workspace
cargo clippy -p scbdb-cli -- -D warnings          # this crate only
cargo fmt --check                                  # format check

# Run the binary directly (dev)
cargo run --bin scbdb-cli -- <subcommand> [flags]
```

## Test Conventions

- **Parser unit tests** (`src/tests.rs`, `src/sentiment/mod.rs`): use `Cli::try_parse_from([...])`, no DB, no async. Pattern-match on the parsed variant. These run without any infrastructure.
- **Integration tests** (`src/collect/collect_test.rs`, `src/collect/brand_test.rs`): use `#[sqlx::test(migrations = "../../migrations")]`. The `migrations` path is relative to the crate root and points to the workspace-level `migrations/` directory. These require a live PostgreSQL instance.
- **Test file path aliases**: `collect/mod.rs` declares `#[path = "collect_test.rs"] mod tests;` and `collect/brand/mod.rs` declares `#[path = "../brand_test.rs"] mod tests;` to co-locate test fixtures with the code they test without placing them in a `tests/` directory.

## Required Environment Variables

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | PostgreSQL connection string |
| `POSTGRES_PASSWORD` | Used by Docker Compose for the `scbdb-postgres` container |
| `LEGISCAN_API_KEY` | Required only for `regs ingest`; other subcommands proceed without it |
| `SCBDB_BRANDS_PATH` | Path to `config/brands.yaml` (default: `./config/brands.yaml`) |
| `SCBDB_LOG_LEVEL` | Log level when `RUST_LOG` is unset (default: `info`) |
| `RUST_LOG` | Standard tracing filter; takes priority over `SCBDB_LOG_LEVEL` when set (e.g. `RUST_LOG=scbdb_cli=debug`) |

Sentiment collection additionally requires env vars consumed by `scbdb_sentiment::SentimentConfig::from_env()` (TEI URL, Qdrant URL).
