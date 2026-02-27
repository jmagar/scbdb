# CLAUDE.md — scbdb-core

## Crate Purpose

`scbdb-core` is the **shared domain foundation** for the entire scbdb workspace. It owns:

- All public domain types (brands, products, variants, images)
- Application configuration loading and validation
- The single `ConfigError` error type used across all crates for config failures
- Slug generation logic (single source of truth)

Every other crate in the workspace depends on this one. It has **zero internal dependencies on other scbdb crates** and must stay that way — it is the bottom of the dependency graph.

## Module Layout

```
src/
├── lib.rs          # Re-exports public surface; defines ConfigError
├── app_config.rs   # AppConfig struct + Environment enum (no I/O)
├── config.rs       # load_app_config / load_app_config_from_env (env-var parsing)
├── config_test.rs  # Unit tests for config parsing (isolated via HashMap injection)
├── brands.rs       # BrandConfig, BrandsFile, Relationship, load_brands, slug_from_name
├── brands_test.rs  # Tests for slug logic, validation, YAML parsing, real file load
└── products.rs     # NormalizedProduct, NormalizedVariant, NormalizedImage + methods
```

## Public API Surface

Everything exported from `lib.rs`:

| Export | Source | Description |
|--------|--------|-------------|
| `AppConfig` | `app_config` | Full parsed runtime config (DB, scraper, server, etc.) |
| `Environment` | `app_config` | `Development` \| `Test` \| `Production` |
| `ConfigError` | `lib` | Enum of all configuration failure modes |
| `BrandConfig` | `brands` | Single brand entry from `brands.yaml` |
| `BrandsFile` | `brands` | Top-level YAML wrapper (`{ brands: Vec<BrandConfig> }`) |
| `Relationship` | `brands` | `Portfolio` \| `Competitor` |
| `load_brands(path)` | `brands` | Parse + validate `brands.yaml`; returns `ConfigError` on failure |
| `load_app_config()` | `config` | Load `AppConfig` from env vars (reads process env) |
| `load_app_config_from_env()` | `config` | Same — delegates to `build_app_config`; prefer for clarity |
| `NormalizedProduct` | `products` | Scraped product, normalized for storage |
| `NormalizedVariant` | `products` | Single purchasable variant of a product |
| `NormalizedImage` | `products` | Product image with optional variant associations |

Internal-only (not re-exported at crate root):

- `brands::slug_from_name(name: &str) -> String` — accessible via `scbdb_core::brands::slug_from_name`
- `config::build_app_config(lookup)` — private; used for testable env-var injection

## Type Reference

### ConfigError

```rust
pub enum ConfigError {
    MissingEnvVar(String),                    // required env var absent
    InvalidEnvVar { var: String, reason: String }, // present but unparseable/invalid
    BrandsFileIo { path: String, source: io::Error }, // can't read brands.yaml
    BrandsFileParse(#[source] serde_yaml::Error),     // YAML parse failure
    Validation(String),                       // semantic validation (tiers, slugs, dupes)
}
```

Use `#[from] scbdb_core::ConfigError` in downstream error enums (see `scbdb-db`).

### AppConfig

```rust
pub struct AppConfig {
    pub database_url: String,               // required: DATABASE_URL
    pub env: Environment,                   // SCBDB_ENV (default: development)
    pub bind_addr: SocketAddr,              // SCBDB_BIND_ADDR (default: 0.0.0.0:3000)
    pub log_level: String,                  // SCBDB_LOG_LEVEL (default: info)
    pub brands_path: PathBuf,              // SCBDB_BRANDS_PATH (default: ./config/brands.yaml)
    pub legiscan_api_key: Option<String>,   // LEGISCAN_API_KEY (optional)
    pub db_max_connections: u32,            // SCBDB_DB_MAX_CONNECTIONS (default: 10)
    pub db_min_connections: u32,            // SCBDB_DB_MIN_CONNECTIONS (default: 1)
    pub db_acquire_timeout_secs: u64,       // SCBDB_DB_ACQUIRE_TIMEOUT_SECS (default: 10)
    pub scraper_request_timeout_secs: u64,  // SCBDB_SCRAPER_REQUEST_TIMEOUT_SECS (default: 30)
    pub legiscan_request_timeout_secs: u64, // SCBDB_LEGISCAN_REQUEST_TIMEOUT_SECS (default: 30)
    pub scraper_user_agent: String,         // SCBDB_SCRAPER_USER_AGENT (default: scbdb/0.1 ...)
    pub scraper_max_concurrent_brands: usize, // SCBDB_SCRAPER_MAX_CONCURRENT_BRANDS (default: 1)
    pub scraper_inter_request_delay_ms: u64,  // SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS (default: 250)
    pub scraper_max_retries: u32,           // SCBDB_SCRAPER_MAX_RETRIES (default: 3)
    pub scraper_retry_backoff_base_secs: u64, // SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS (default: 5)
}
```

`AppConfig::fmt` is manually implemented to **redact** `database_url` and `legiscan_api_key` from debug output — never exposes secrets in logs.

Cross-field validation: `db_min_connections <= db_max_connections` is enforced at parse time.

### BrandConfig

```rust
pub struct BrandConfig {
    pub name: String,
    pub relationship: Relationship,          // Portfolio | Competitor
    pub tier: u8,                            // must be 1, 2, or 3
    pub domain: Option<String>,
    pub shop_url: Option<String>,
    pub store_locator_url: Option<String>,
    pub notes: Option<String>,
    pub social: HashMap<String, String>,     // platform → handle, #[serde(default)]
    pub domains: Vec<String>,               // all known domains, #[serde(default)]
    pub twitter_handle: Option<String>,      // canonical Twitter/X handle, #[serde(default)]
}
```

`twitter_handle` is **separate from** `social["twitter"]`. The sentiment pipeline uses `twitter_handle` as the authoritative source for brand-timeline collection.

### NormalizedProduct / NormalizedVariant / NormalizedImage

See `products.rs` for full field docs. Key boundary notes:

- `source_product_id` and `source_variant_id` are stored as `String` (Shopify IDs exceed `i64` range)
- `price` and `compare_at_price` are `String` (exact decimal, no floating-point rounding)
- `dosage_mg`, `cbd_mg`, `size_value` are `f64` at scrape time; DB layer converts to `NUMERIC(8,2)` / `NUMERIC(10,2)` at write time

`NormalizedProduct` convenience methods:

```rust
product.variant_count() -> usize
product.has_available_variants() -> bool
product.default_variant() -> Option<&NormalizedVariant>  // position-1 (is_default = true)
```

## Slug Generation

`slug_from_name(name: &str) -> String` is the **single source of truth** for brand slugs across the workspace. `BrandConfig::slug()` delegates to it.

Algorithm: lowercase → keep ASCII alphanumeric + hyphens, spaces → hyphens, strip everything else → collapse consecutive hyphens → strip leading/trailing hyphens.

Gotcha: non-ASCII characters (including accented letters like `Ē`, `ñ`) are **silently stripped**, not transliterated. `"BRĒZ"` → `"brz"` (tested in `brands_test.rs`). This is intentional — document it in brand entries if the resulting slug looks wrong.

## Config Loading Pattern

`build_app_config` takes a `Fn(&str) -> Result<String, VarError>` lookup function. This decouples parsing logic from the actual environment, making config tests hermetic:

```rust
// In tests: inject a HashMap, no set_var/remove_var needed
let map = HashMap::from([("DATABASE_URL", "postgres://...")]);
let cfg = build_app_config(|k| map.get(k).copied().map(String::from).ok_or(VarError::NotPresent));
```

**dotenvy policy:** `build_app_config` and `load_app_config_from_env` do **not** call `dotenvy::dotenv()`. Only binary entrypoints (`scbdb-cli::main`, `scbdb-server::main`) load `.env`. This crate must never call `dotenvy`.

## Who Depends on This Crate

| Crate | What it uses |
|-------|-------------|
| `scbdb-cli` | `AppConfig`, `Environment`, `load_app_config`, `load_brands`, `NormalizedProduct`, `NormalizedVariant` |
| `scbdb-server` | `AppConfig`, `Environment`, `load_app_config`, `slug_from_name` |
| `scbdb-db` | `AppConfig`, `ConfigError` (via `#[from]`), `BrandConfig`, `NormalizedProduct`, `NormalizedVariant`, `load_app_config_from_env` |
| `scbdb-scraper` | `NormalizedImage`, `NormalizedProduct`, `NormalizedVariant` |
| `scbdb-sentiment` | Declared dependency (no active imports yet — reserved for future config threading) |
| `scbdb-profiler` | Declared dependency (no active imports yet — reserved for future config threading) |

## Rules for Adding New Types

1. **No I/O in type definitions.** Types are data. Load functions (`load_brands`, `load_app_config`) handle I/O. Never add `async fn` or network calls to this crate.
2. **No other scbdb crate dependencies.** This crate is the dependency floor. Adding `scbdb-db` or `scbdb-scraper` here creates cycles.
3. **All public types derive `Debug`, `Clone`, `Serialize`, `Deserialize`** unless there is a specific reason not to (e.g., `AppConfig` has manual `Debug` to redact secrets).
4. **Validate at parse time.** If a new config field has cross-field constraints (like `min <= max`), enforce them inside `build_app_config` before returning `AppConfig`.
5. **Serialization stability.** Fields used in DB storage or API responses are part of the serialized contract. Renaming a field requires a serde alias (`#[serde(alias = "old_name")]`) or a migration. Do not rename without considering downstream impact.
6. **`#[serde(default)]` for optional collections.** New `Vec<T>` or `HashMap<K,V>` fields on `BrandConfig` must carry `#[serde(default)]` so existing `brands.yaml` entries without the field remain valid.
7. **Slugs are stable identifiers.** Changing `slug_from_name` behavior is a **breaking change** — slugs are stored in the DB as primary keys for brands. Any algorithm change must be coordinated with a migration.
8. **`must_use` on pure query methods.** Methods like `variant_count()`, `has_available_variants()`, `default_variant()`, `slug()` all carry `#[must_use]`. Maintain this for new pure methods.

## Build and Test in Isolation

```bash
# Build only this crate
cargo build -p scbdb-core

# Run all tests (no DB, no network required)
cargo test -p scbdb-core

# Check with clippy (CI-equivalent)
cargo clippy -p scbdb-core -- -D warnings

# Check formatting
cargo fmt --check -p scbdb-core
```

The `load_brands_from_real_file` test in `brands_test.rs` reads `../../config/brands.yaml` relative to `CARGO_MANIFEST_DIR`. It requires the root `config/brands.yaml` to exist but has no other external dependencies.

Config tests are fully hermetic — they use `HashMap`-backed `build_app_config` injection and require no environment variables.

## Conventions Specific to This Crate

- **Test files are siblings, not submodules.** Tests live in `brands_test.rs` and `config_test.rs` and are included via `#[path = "..."]` attributes in their parent modules. New test files follow the same pattern.
- **`thiserror` for all error variants.** Use `#[error("...")]` and `#[source]` / `#[from]` attributes consistently. Never `impl Error` by hand.
- **No `unwrap()` or `expect()` in library code.** `brands_test.rs` and `products.rs` tests use `expect()` for clarity in tests only.
- **`serde_json` is a dev-dependency only.** Production code serializes via `serde_yaml` (for brands config). `serde_json` is test-only for roundtrip verification.
