# CLAUDE.md — scbdb-scraper

Crate type: library. Owns all I/O against external storefronts and store
locator pages. Nothing in this crate touches the database.

## Purpose

Two primary responsibilities:

1. **Shopify product collection** — fetch `products.json` from any Shopify
   storefront with cursor-based pagination, retry/backoff, and normalization
   into `scbdb_core` domain types.

2. **Store locator collection** — crawl a brand's "where to buy" page,
   detect one of 13 supported locator widgets/formats, and extract
   `RawStoreLocation` records.

Secondary responsibilities: brand logo discovery (`logo.rs`).

## Public API

```rust
// Product collection
let client = ShopifyClient::new(timeout_secs, user_agent, max_retries, backoff_base_secs)?;
let products: Vec<ShopifyProduct> = client.fetch_all_products(shop_url, 250, delay_ms).await?;
let normalized: NormalizedProduct = normalize_product(product, shop_url)?;

// Store locator
let locations: Vec<RawStoreLocation> = fetch_store_locations(locator_url, timeout_secs, user_agent).await?;
let key: String = make_location_key(brand_id, &location); // SHA-256 dedup key
validate_store_locations_trust(&locations)?;             // quality gate

// Logo
let logo: Option<String> = fetch_brand_logo_url(shop_url, timeout_secs, user_agent).await?;
```

## Module Map

| Module | Visibility | Purpose |
|--------|-----------|---------|
| `client/mod.rs` | `pub` | `ShopifyClient` — single-page HTTP fetch with retry |
| `client/fetch_all.rs` | `pub(super)` | Multi-page loop: follows `Link` cursors until `rel="next"` is absent; also houses `fetch_all_products_browser_profile` (Chrome-like UA for 403 brands) |
| `client/origin.rs` | `pub` | `extract_store_origin` — strips path from `shop_url` to build root URL |
| `types.rs` | `pub` | `ShopifyProductsResponse`, `ShopifyProduct`, `ShopifyVariant`, `ShopifyImage` |
| `normalize.rs` | `pub` | `normalize_product` — raw Shopify → `NormalizedProduct`/`NormalizedVariant` |
| `parse.rs` | `pub(crate)` | `parse_thc_mg`, `parse_cbd_mg`, `parse_size` — title/HTML dosage extraction |
| `parse_helpers.rs` | `pub(crate)` | Byte-scanning primitives used by `parse.rs` |
| `rate_limit.rs` | `pub(crate)` | `retry_with_backoff` — exponential backoff loop |
| `pagination.rs` | `pub` | `extract_next_cursor` — parses Shopify `Link` response header |
| `logo.rs` | `pub` | `fetch_brand_logo_url` — heuristic logo extraction from storefront HTML |
| `locator/mod.rs` | `pub` | `fetch_store_locations` — orchestrates 13 extraction strategies |
| `locator/fetch.rs` | `pub(crate)` | `fetch_html`, `fetch_text`, `fetch_json` — low-level HTTP for locator |
| `locator/types.rs` | `pub` | `RawStoreLocation`, `LocatorError` |
| `locator/trust.rs` | `pub` | `validate_store_locations_trust`, `make_location_key` |
| `locator/grid.rs` | `pub(crate)` | `GridConfig`, `generate_grid`, `STRATEGIC_US_POINTS` |
| `locator/formats/` | `pub(super)` | One module per locator provider (13 total) |

## Shopify products.json Endpoint

```
GET https://{store}.myshopify.com/products.json?limit=250&page_info={cursor}
```

- `shop_url` in `brands.yaml` may be a collection URL like
  `https://drinkcann.com/collections/all`; `extract_store_origin` strips the
  path before constructing the request.
- `limit` max is 250 (Shopify cap). Callers pass this; the client does not
  enforce it.
- Shopify uses cursor-based pagination via the `Link` response header, not
  offset/page. The cursor is opaque (base64url); never increment it manually.
- The last page has no `rel="next"` link in the `Link` header.

### Known field quirks (documented in `types.rs`)

| Field | Observed behavior |
|-------|-------------------|
| `tags` | JSON array of strings, NOT comma-separated string |
| `compare_at_price` | `null` when not on sale (not `"0.00"`) |
| `status` | May be absent from public endpoint; default to `"active"` |
| `available` on variant | May be absent; default to `true` (optimistic) |
| `product_type` | May be empty string; normalize to `None` |
| THC/CBD dosage | Not a structured field — parsed from variant title and `body_html` |

## Retry / Backoff Strategy

Implemented in `rate_limit.rs` as `retry_with_backoff`.

**Retriable errors:**
- `ScraperError::RateLimited` (HTTP 429)
- `ScraperError::Http` (network / TLS failure)
- `ScraperError::UnexpectedStatus` with `status >= 500` (502/503/504 from CDN)

**Non-retriable errors (propagated immediately):**
- `ScraperError::NotFound` (404)
- `ScraperError::UnexpectedStatus` with `status < 500` (e.g., 403)
- `ScraperError::Deserialize`
- `ScraperError::Normalization`
- `ScraperError::PaginationLimit`

**Backoff formula:** `backoff_base_secs * 2^attempt` seconds with ±25% jitter.
For `RateLimited`, delay is `max(computed_backoff, retry_after_secs)` so the
server-supplied `Retry-After` header is always honoured.

With `max_retries = 3`, the operation is attempted at most 4 times total
(initial + 3 retries).

## fetch_all_products — All-or-Nothing Semantics

`fetch_all_products` discards products from earlier pages if any page fails.
This is intentional. Partial product lists produce incorrect deltas when
compared against previous full snapshots in the DB. Either the full catalog
comes back or the whole batch is dropped.

Pagination guard: `MAX_PAGES = 200`. Hitting this limit returns
`ScraperError::PaginationLimit` — it guards against cycling cursors, not
catalog size.

**Browser fallback:** `fetch_all_products_browser_profile` uses a Chrome-like
`User-Agent` for stores that return 403 to the default scraper UA.

## Product Normalization (`normalize.rs`)

`normalize_product(product, shop_url)` → `NormalizedProduct`

Key decisions:

- **Default variant**: variant with `position == 1`. Falls back to first-by-index
  when no position data exists, or when no variant claims position 1 (e.g., a
  deleted variant).
- **Product type**: empty string treated as `None`.
- **Status**: absent field defaults to `"active"`.
- **Source URL**: constructed as `{origin}/products/{handle}`.
- **Primary image**: prefers the image associated with the default variant's ID,
  then position-1 image, then `product.image`, then first in gallery.
- **SKU**: empty string treated as `None`.
- **Currency**: hardcoded `"USD"` — Shopify's products.json does not expose
  per-variant currency.

### Dosage extraction (`parse.rs`, `parse_helpers.rs`)

Dosage is not a structured field. Extraction is best-effort with this fallback
chain applied per variant:

1. `parse_thc_mg(&variant.title)` — looks for `"5mg THC"`, `"THC 5mg"`, bare
   `"5mg"` patterns in the variant title.
2. `parse_thc_from_html(&product.body_html)` — strips HTML tags, decodes
   entities, then runs `parse_thc_mg`. Used for brands like BREZ that embed
   dosage only in the product description.
3. `parse_thc_mg(&product.title)` — used for brands like Better Than Booze
   that encode dosage in the product name but not variant titles.

**Important limitation:** When the HTML fallback is used, the same dosage value
is applied to every variant of the product. If a product has multiple dosage
strengths across variants with bare titles, the first THC value found in the
HTML is attributed to all — which will be wrong. This is a documented
trade-off in `normalize_variant` doc comments.

`parse_cbd_mg` is separate from `parse_thc_mg` and requires an explicit CBD
label. It does NOT fall back to bare `"Nmg"` patterns to avoid misattributing
CBD to the THC `dosage_mg` field.

Byte-scanning in `parse_helpers.rs` uses manual iteration rather than regex to
stay allocation-free in the hot path.

## Store Locator (`locator/`)

`fetch_store_locations(locator_url, timeout_secs, user_agent)` tries 13
extraction strategies in priority order:

| Priority | Strategy | Key signal |
|----------|----------|------------|
| 1 | Locally.com | `locallyWidgetCompanyId` or `company_id` query param |
| 2 | Storemapper | `data-storemapper-token` or `token=` in API URL |
| 3 | Stockist | Stockist widget tag; also checks linked `/pages/dealers` page |
| 4 | Storepoint | Storepoint widget ID |
| 5 | Roseperl/Secomapp | WTB JS URL |
| 6 | VTInfo | `finder.vtinfo.com` iframe embed |
| 7 | AskHoodie | `hoodieEmbedWtbV2` embed ID |
| 8 | BeverageFinder | BeverageFinder API key |
| 9 | Agile Store Locator | WordPress plugin AJAX config; may follow linked store-locator page |
| 10 | StoreRocket | StoreRocket account ID |
| 11 | Destini | `lets.shop` / Destini locator config |
| 12 | JSON-LD | `<script type="application/ld+json">` with `LocalBusiness` / `Store` types |
| 13 | Embedded JSON | Raw JSON arrays in `<script>` tags |

Returns `Ok(vec![])` when the page is reachable but no strategy yields results.
Returns `Err(LocatorError::AllAttemptsFailed)` when the page itself cannot be
fetched.

### HTML Fetching (`locator/fetch.rs`)

Three attempts with backoff `[0ms, 300ms, 900ms]`. Each attempt:

1. `curl -Lsf` with browser UA (avoids anti-bot stacks that block reqwest)
2. reqwest with caller-supplied UA
3. reqwest with browser fallback UA

Cloudflare challenge pages are detected and discarded as unusable. Locator
hint keywords (`storemapper`, `stockist`, `beveragefinder`, etc.) force-accept
a response even when it otherwise looks like a challenge page.

### Geographic Grid (`locator/grid.rs`)

`STRATEGIC_US_POINTS` — 9 hardcoded city centers for Destini and VTInfo
sweeps. Charlotte is at index 1 to guarantee Southeast coverage before VTInfo's
100-result dedup break fires.

`GridConfig::conus_coarse()` — 200-mile grid across CONUS (~168 points) with
accepted 41-mile dead zones at cell corners.

`GridConfig::sc_region()` — Tighter SC + neighbors grid at 30-mile step
(~82 points). Available for future SC-specific tasks.

### Trust Validation (`locator/trust.rs`)

`validate_store_locations_trust` enforces a quality gate before stored data
is mutated:

- Named providers (`locally`, `stockist`, `storemapper`, etc., `destini`,
  `jsonld`) — always trusted.
- `json_embed` — trusted only when `count >= 5` AND `>=80%` of records have
  name + (address OR city+state OR coordinates).
- Empty scrape — always rejected.

`make_location_key(brand_id, location)` — SHA-256 dedup hash over
`brand_id || name || city || state || zip` (name/city lowercased, state
uppercased).

## Logo Extraction (`logo.rs`)

Fetches the storefront homepage and scores candidates by source and size:

| Source | Base score |
|--------|-----------|
| `og:logo` meta | 600 |
| `<img>` with `logo` in class/id/alt | 500 |
| `og:image` meta | 340 |
| `<link rel="icon">` | 80 |

Score adjustments: SVG +120, PNG +100, `.ico` −260, `favicon` in URL −220,
`apple-touch-icon` −130, dimensions ≤32px −260.

Returns `None` rather than failing when no logo is found.

## Error Types

| Error | Retry? | Meaning |
|-------|--------|---------|
| `ScraperError::Http` | yes | reqwest network failure |
| `ScraperError::RateLimited` | yes | HTTP 429 |
| `ScraperError::NotFound` | no | HTTP 404 |
| `ScraperError::UnexpectedStatus(>=500)` | yes | CDN/server error |
| `ScraperError::UnexpectedStatus(<500)` | no | Client error (403, etc.) |
| `ScraperError::Deserialize` | no | Bad JSON from endpoint |
| `ScraperError::Normalization` | no | Missing variants or unparseable price |
| `ScraperError::PaginationLimit` | no | Exceeded `MAX_PAGES = 200` |
| `LocatorError::Http` | — | reqwest error in locator pipeline |
| `LocatorError::AllAttemptsFailed` | — | All UA/curl attempts returned non-2xx |

## Dry-Run Mode

This crate does not implement dry-run logic. Dry-run is enforced by the
**caller** (the `collect` command in `scbdb-cli`) which calls
`normalize_product` and logs results without writing to the DB. The scraper
itself always makes real HTTP requests.

## Concurrency Model

`scbdb-scraper` makes no decisions about concurrency across brands. Each
public function is an async fn; the caller in `scbdb-cli` drives parallelism
(typically `FuturesUnordered` or `tokio::spawn` per brand). The `inter_request_delay_ms`
parameter in `fetch_all_products` governs delay between successive *pages* of
the same brand, not between brands.

## Testing

### Dependency

`wiremock` is the test HTTP server. It's a `dev-dependency` only.

### Test file layout

Tests are co-located via `#[path = ...]` attributes, not a separate `tests/`
directory:

| Source | Test file |
|--------|-----------|
| `client/mod.rs` | `src/client_test.rs` |
| `normalize.rs` | `src/normalize_test.rs` |
| `parse.rs` | `src/parse_test.rs` |
| `rate_limit.rs` | inline `mod tests` |
| `pagination.rs` | inline `mod tests` |
| `locator/mod.rs` | inline `mod tests` |
| `logo.rs` | inline `mod tests` |

### Running tests

```bash
# All scraper tests (no network — wiremock serves local HTTP)
cargo test -p scbdb-scraper

# Specific module
cargo test -p scbdb-scraper normalize
cargo test -p scbdb-scraper pagination
cargo test -p scbdb-scraper rate_limit
```

### Mocking HTTP for new tests

Use `wiremock`:

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

let server = MockServer::start().await;
Mock::given(method("GET"))
    .and(path("/products.json"))
    .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
    .mount(&server)
    .await;

let client = ShopifyClient::new(10, "test-agent", 0, 0).unwrap();
let products = client.fetch_all_products(&server.uri(), 250, 0).await.unwrap();
```

Set `max_retries = 0` and `backoff_base_secs = 0` in tests to eliminate sleep
delays.

## Workspace Dependencies

| Crate | Usage |
|-------|-------|
| `scbdb-core` | `NormalizedProduct`, `NormalizedVariant`, `NormalizedImage` — the output types of normalization |
| `reqwest` | HTTP client (with `json`, `gzip`, `rustls-tls` features via workspace) |
| `serde` / `serde_json` | Deserialize Shopify JSON; serialize `raw_data` in `RawStoreLocation` |
| `tokio` | Async runtime; `tokio::time::sleep` for backoff and inter-page delay |
| `tracing` | Structured logging throughout; no `println!` |
| `thiserror` | `ScraperError` and `LocatorError` derive |
| `sha2` | SHA-256 for `make_location_key` |
| `rand` | ±25% jitter in `retry_with_backoff` |
| `regex` | Logo extraction and locator HTML pattern matching |

## Gotchas

- **`extract_store_origin` must be called before building product URLs.** A
  `shop_url` of `"https://drinkcann.com/collections/all"` would otherwise
  produce `https://drinkcann.com/collections/all/products.json`.

- **`dotenvy` must NOT be called here.** Library crates do not load `.env`.
  Only `scbdb-cli` and `scbdb-server` entrypoints call `dotenvy::dotenv()`.

- **Shopify tags are a JSON array, not a comma-separated string.** The legacy
  Liquid API returned a CSV string; `products.json` returns `["tag1", "tag2"]`.
  `#[serde(default)]` handles stores with no tags (empty array).

- **`compare_at_price` is `null`, not `"0.00"`, when no sale is active.** Do
  not normalize null to zero.

- **`fetch_all_products` is all-or-nothing.** On any page error, products from
  already-fetched pages are discarded. The caller should not cache or act on
  partial results.

- **The HTML dosage fallback is applied uniformly to all variants.** If a
  product has variants with different dosage strengths but bare titles (no mg in
  variant title), the first THC value from `body_html` is attributed to every
  variant. This is a known, documented limitation.

- **VTInfo `STRATEGIC_US_POINTS` ordering matters.** Charlotte is at index 1
  to guarantee Southeast coverage is always searched before VTInfo breaks its
  loop at 100 deduplicated results. Do not reorder this slice casually.

- **`locator/fetch.rs` spawns `curl` as a subprocess.** This is intentional:
  `curl`'s TLS fingerprint is more convincing to anti-bot stacks than reqwest's
  default profile. Tests that mock HTTP at the `reqwest` level will not
  intercept the `curl` path.

- **`parse_helpers.rs` uses manual byte scanning, not regex.** This avoids
  heap allocations per variant in the hot normalization path. When extending
  dosage/size parsing, follow the same byte-scan pattern rather than adding
  `Regex::new` calls inline.
