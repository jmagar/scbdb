# CLAUDE.md — scbdb-db

Database access layer for SCBDB. Owns all `sqlx` queries, pool management, migrations, and seeding. This crate is a library consumed by `scbdb-cli` and `scbdb-server` — it has no binary entrypoint.

## Crate Purpose

- PostgreSQL connection pool creation and health checks
- All SQL queries for every domain table (no inline SQL in consumer crates)
- Migration runner (wraps `sqlx::migrate!`)
- Brand seeding from `BrandConfig`

## Environment Variables

| Variable | Required | Notes |
|----------|----------|-------|
| `DATABASE_URL` | Yes | Full `postgres://` connection string |
| `SCBDB_DB_MAX_CONNECTIONS` | No | Default: 10 |
| `SCBDB_DB_MIN_CONNECTIONS` | No | Default: 1 |
| `SCBDB_DB_ACQUIRE_TIMEOUT_SECS` | No | Default: 10 |

Pool settings are read by `scbdb_core::load_app_config_from_env()` and passed into `PoolConfig::from_app_config()`. Do not read `DATABASE_URL` directly in this crate — receive it as a parameter or via the `AppConfig`.

**CRITICAL: Do NOT call `dotenvy::dotenv()` anywhere in this crate.** Only binary entrypoints (`scbdb-cli`, `scbdb-server`) load `.env`. Library crates that call `dotenvy::dotenv()` will interfere with test harnesses and CI.

## Workspace Dependencies

| Crate | Why |
|-------|-----|
| `scbdb-core` | `AppConfig`, `NormalizedProduct`, `NormalizedVariant`, `BrandConfig`, `ConfigError` |

The dependency is one-way: `scbdb-db` → `scbdb-core`. Nothing else.

## Pool Entry Points

`PoolConfig` carries the pool-tuning values parsed from `AppConfig`:

```rust
pub struct PoolConfig {
    pub max_connections: u32,        // SCBDB_DB_MAX_CONNECTIONS (default: 10)
    pub min_connections: u32,        // SCBDB_DB_MIN_CONNECTIONS (default: 1)
    pub acquire_timeout_secs: u64,   // SCBDB_DB_ACQUIRE_TIMEOUT_SECS (default: 10)
}
// Constructed via: PoolConfig::from_app_config(&AppConfig)
```

```rust
// Connect using explicit URL + config
scbdb_db::connect_pool(database_url: &str, config: PoolConfig) -> Result<PgPool, sqlx::Error>

// Connect reading everything from env (calls scbdb_core::load_app_config_from_env)
scbdb_db::connect_pool_from_env() -> Result<PgPool, DbError>

// Run all pending migrations; returns count applied
scbdb_db::run_migrations(pool: &PgPool) -> Result<usize, MigrateError>

// Verify live connection (SELECT 1)
scbdb_db::ping(pool: &PgPool) -> Result<(), sqlx::Error>
scbdb_db::health_check(pool: &PgPool) -> Result<(), DbError>
```

## Module Map

Each module owns one domain or one related group of tables. All public types and functions are re-exported from `lib.rs` — consumers call `scbdb_db::upsert_product(...)`, not `scbdb_db::products::upsert_product(...)`.

| Module | Tables touched | Key operations |
|--------|---------------|----------------|
| `brands` | `brands` | `list_active_brands`, `get_brand_by_slug`, `create_brand`, `update_brand`, `deactivate_brand`, `update_brand_logo`, `update_brand_store_locator_url`, `list_brands_with_locator` |
| `products` | `products`, `product_variants`, `price_snapshots` | `upsert_product`, `upsert_variant`, `insert_price_snapshot_if_changed`, `get_last_price_snapshot` |
| `bills` | `bills` | `upsert_bill`, `list_bills`, `get_bill_by_jurisdiction_number`, `get_bill_by_public_id` |
| `bill_events` | `bill_events` | `upsert_bill_event`, `list_bill_events`, `list_bill_events_batch`, `list_bill_events_by_public_id` |
| `bill_texts` | `bill_texts`, `bill_text_hash_cache` | `upsert_bill_text`, `list_bill_texts_by_public_id`, `get_bills_stored_hashes` |
| `collection_runs` | `collection_runs`, `collection_run_brands` | Full state machine: `create` → `start` → `complete`/`fail`; `upsert_collection_run_brand` |
| `sentiment` | `sentiment_snapshots` | `insert_sentiment_snapshot`, `list_sentiment_snapshots`, `get_latest_sentiment_by_brand` |
| `locations` | `store_locations` | `upsert_store_locations`, `deactivate_missing_locations`, `get_active_location_keys_for_brand`, `list_active_location_pins`, `list_active_locations_by_brand`, `list_locations_by_state`, `list_locations_dashboard_summary`, `list_new_locations_since` |
| `brand_profiles` | `brand_profiles`, `brand_social_handles`, `brand_domains` | `get_brand_profile`, `upsert_brand_profile`, `overwrite_brand_profile`, `replace_brand_social_handles`, `replace_brand_domains`, `list_brands_without_profiles`, `list_brand_social_handles` |
| `brand_signals` | `brand_signals`, `brand_domains`, `brand_social_handles` | `upsert_brand_signal`, `list_brand_signals` (cursor-paginated), `list_brand_feed_urls`, `list_brands_needing_signal_refresh`, `list_brands_with_stale_handles` |
| `brand_completeness` | `brand_profiles`, `brand_social_handles`, `brand_domains`, `brand_signals`, `brand_funding_events`, `brand_lab_tests`, `brand_legal_proceedings`, `brand_sponsorships`, `brand_distributors`, `brand_media_appearances` | `get_brand_completeness`, `get_all_brands_completeness` — 0-100 weighted score across all intel dimensions |
| `brand_intel` | 8 tables (funding events, lab tests, legal proceedings, sponsorships, distributors, competitor relationships, newsletters, media appearances) | insert + list for each table |
| `api_queries` | views (`view_products_dashboard`, `view_pricing_summary`), `sentiment_snapshots`, joins | `list_products_dashboard`, `list_price_snapshots_dashboard`, `list_pricing_summary`, `list_sentiment_summary`, `list_sentiment_snapshots_dashboard` |
| `seed` | `brands`, `brand_social_handles`, `brand_domains` | `seed_brands` (full transactional seed), `upsert_brand_social_handles`, `upsert_brand_domains` |

## Migrations

Migrations live at `<workspace-root>/migrations/`. The `MIGRATOR` static in `lib.rs` resolves to that directory via a path relative to `Cargo.toml`:

```rust
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
```

### Naming Convention

```
YYYYMMDDNNNNNN_descriptive_snake_case.up.sql
YYYYMMDDNNNNNN_descriptive_snake_case.down.sql
```

Where `NNNNNN` is a 6-digit counter: `000100`, `000200`, etc. Group by date. See existing files for the pattern.

### Rules — Never Break These

- **Append-only**: Never modify an already-applied migration. Add a new one instead.
- **Always ship a `.down.sql`**: Even if it's just `-- intentionally left empty` (prefer a proper rollback).
- **Schema docs must stay current**: After adding a migration, update `docs/DATABASE_SCHEMA.md` to reflect the new tables or columns.
- **`sqlx::migrate!` path is relative to `crates/scbdb-db/Cargo.toml`**: `../../migrations` resolves to `<workspace-root>/migrations`. Tests use the identical path via `#[sqlx::test(migrations = "../../migrations")]`.

### Adding a Migration

1. Create `migrations/YYYYMMDD0N_description.up.sql` and `.down.sql`
2. Run `just migrate` to apply
3. Run `just migrate-status` to verify
4. Update `docs/DATABASE_SCHEMA.md`
5. Add/update query functions in the relevant module
6. Update the `RETURNING` column lists in any affected queries (sqlx is strict — if the schema adds a column and you don't SELECT it, `FromRow` derivation will fail at compile time if the field is in the struct)

## sqlx Gotchas

**`SELECT 1` returns `i32`, not `i64`.**
PostgreSQL integer literals are `int4`. Use `query_scalar::<_, i32>`. The `ping()` function in `lib.rs` demonstrates the correct pattern:
```rust
sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pool).await?;
```

**`RETURNING` column lists must be exhaustive.**
`sqlx::query_as::<_, MyRow>` maps columns positionally to struct fields via `FromRow`. If the struct has a field the query doesn't return, compilation fails. Always write explicit column lists in `SELECT` and `RETURNING` — never `SELECT *`.

**Nullable columns as `Option<Option<T>>` in `update_brand`.**
Partial updates on nullable columns need two levels of `Option`: `None` = don't touch the column, `Some(None)` = set to NULL, `Some(Some(v))` = set to v. The implementation uses a bool flag + value pair bound into a `CASE WHEN $flag THEN $val ELSE col END` expression.

**`brand_signal_type` is a PostgreSQL enum.**
When binding to or selecting from `signal_type`, cast explicitly: `$2::brand_signal_type` on insert, `signal_type::TEXT` on select. Omitting the cast causes a type mismatch error at runtime.

**Competitor relationships have canonical ordering.**
`brand_competitor_relationships` enforces `brand_id < competitor_brand_id` at the DB layer. The insert function in `brand_intel/competitor_relationships.rs` sorts the two IDs before binding to prevent duplicate rows from reversed-argument calls.

**Numeric fields in `upsert_variant` use a precision boundary.**
`dosage_mg`, `cbd_mg`, and `size_value` are bound as `f64` and cast to fixed-scale `NUMERIC` inside SQL (`$7::numeric(8,2)`, etc.). Scrape-time floating values are rounded at persistence time. This is intentional and documented.

**Price snapshot dedup is atomic.**
`insert_price_snapshot_if_changed` uses a CTE (`WITH last AS (...)`) so the SELECT and conditional INSERT happen in a single round-trip. The older SELECT-then-INSERT pattern had a TOCTOU race; the CTE form does not.

**`bill_events` dedup uses `NULLS NOT DISTINCT`.**
The unique index `idx_bill_events_dedup` was created with `NULLS NOT DISTINCT` so that two events with `event_date = NULL` are considered equal. Use `ON CONFLICT DO NOTHING` — not `WHERE NOT EXISTS` — because the latter is not atomic.

**`bills.introduced_date` is set-once.**
The `upsert_bill` conflict clause deliberately omits `introduced_date` from the update set. A second upsert with a different date will not overwrite the original value.

**`api_queries` reads from views.**
`list_products_dashboard` and `list_pricing_summary` query `view_products_dashboard` and `view_pricing_summary` respectively. These views are created in migration `20260221000500_api_views.up.sql`. If the view is missing (e.g., during a partial migration), these queries will fail with a "relation does not exist" error.

## Error Type

```rust
pub enum DbError {
    MissingDatabaseUrl,
    NotFound,
    InvalidCollectionRunTransition { id: i64, expected_status: &'static str },
    Config(scbdb_core::ConfigError),
    Sqlx(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
}
```

`InvalidCollectionRunTransition` is returned when a state machine guard fails (e.g., completing a run that is still `queued`). All state transitions check `rows_affected() == 0` and return this error rather than silently succeeding.

## Testing

### Offline tests — `tests/integration.rs`

No database required. Tests construct row types directly and verify struct field types and pool config logic. Run with:

```bash
cargo test -p scbdb-db
```

### Live tests — `tests/live.rs`

Each test function is annotated `#[sqlx::test(migrations = "../../migrations")]`. The sqlx test harness spins up a fresh PostgreSQL instance per test, runs all migrations, and tears it down when the test completes. These tests require Docker (or a local Postgres with `DATABASE_URL` set).

```bash
# Run all live tests (requires Docker or local Postgres)
cargo test -p scbdb-db --test live

# Run a specific test
cargo test -p scbdb-db --test live collection_run_lifecycle_queued_to_succeeded
```

Live test helpers (`insert_test_brand`, `make_normalized_product`, `make_normalized_variant`) are defined at the top of `tests/live.rs`. Add new helpers there rather than duplicating setup SQL inside individual tests.

### Adding a new query module

1. Create `src/my_module.rs` with a module-level doc comment and `use crate::DbError`.
2. Define `Row` structs with `#[derive(Debug, Clone, sqlx::FromRow)]`.
3. Define `New*` structs (input types) without `FromRow`.
4. Write async query functions returning `Result<T, DbError>`.
5. Add `pub mod my_module;` to `lib.rs`.
6. Add `pub use my_module::{...}` re-exports to `lib.rs`.
7. Add live tests in `tests/live.rs` under a clearly labeled section comment.

### Adding a new migration + query

1. Create the migration file (see naming convention above).
2. If new columns are added to an existing table, update all `SELECT` and `RETURNING` lists in the module that owns that table. `sqlx::FromRow` will fail to compile if columns are missing.
3. If the struct gains a new field, update all construction sites in `tests/integration.rs` (the offline struct smoke tests).

## Code Conventions

- **No inline SQL in consumer crates.** All SQL belongs here.
- **Explicit column lists always.** No `SELECT *` or `RETURNING *`.
- **`query_as` for multi-column rows, `query_scalar` for single values.**
- **Use transactions for multi-statement writes.** See `seed.rs` (`seed_brands`) and `brand_profiles.rs` (`replace_brand_social_handles`, `replace_brand_domains`) for the pattern.
- **Soft-delete pattern.** `brands` uses `is_active = false` + `deleted_at = NOW()`. All list queries filter `deleted_at IS NULL`. Do not hard-delete rows from these tables.
- **`updated_at = NOW()` on every UPDATE.** Always set it explicitly; there are no triggers handling this.
- **`COALESCE` for preserve-existing semantics, `EXCLUDED.*` for overwrite semantics.** Document which one applies in the function doc comment.
- **Google-style doc comments with `# Errors` section.** Every `pub async fn` must document what it returns and what failure conditions map to which `DbError` variant.
