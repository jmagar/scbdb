# Phase 2: Collection CLI

## Document Metadata

- Version: 1.2
- Status: Complete
- Last Updated (EST): 09:30:00 | 02/19/2026 EST

## Objective

Implement CLI-driven product and pricing collection workflows.

## Target Outcomes

- Shopify ingestion path operational.
- Product and variant normalization persisted.
- Pricing snapshots captured reliably.
- Collection run audit trail available.

## Deliverables

- CLI collection commands
- Scraper client + normalization rules
- Persistence integration
- Collection run status reporting

## Resolved Decisions

| Decision | Resolution |
|---|---|
| `collect` subcommand hierarchy | `collect products` and `collect pricing` as sub-subcommands; bare `collect products` defaults to all active brands |
| Default brand scope | All brands with `is_active = true` |
| Dosage extraction source | Best-effort title parsing (regex for "Xmg THC", "Xmg CBD", "Xoz"); full extraction deferred to Phase 4 LLM pipeline |
| Price snapshot dedup | Write new snapshot only when price changes from last snapshot for that variant |
| Per-brand failure behavior | Continue to next brand; record failure in `collection_run_brands.error_message`; run succeeds if at least one brand succeeds |
| Rate limiting | Configurable via env vars (`SCBDB_SCRAPER_INTER_REQUEST_DELAY_MS` default 250ms, `SCBDB_SCRAPER_MAX_RETRIES` default 3, `SCBDB_SCRAPER_RETRY_BACKOFF_BASE_SECS` default 5) |

## Shopify Ingestion Path

- Public endpoint: `{shop_url}/products.json?limit=250` — no API key required
- Pagination: Shopify Link header with `page_info` cursor; iterate until no `rel="next"` in Link header
- All brands fetched sequentially (configurable concurrency, default 1) with configurable inter-request delay

## Normalization Contract

| Shopify Field | DB Column | Notes |
|---|---|---|
| `product.id` | `products.source_product_id` | Stored as string |
| `product.title` | `products.name` | |
| `product.handle` | `products.handle` | New in Phase 2 migration |
| `product.body_html` | `products.description` | Raw HTML; stripping deferred |
| `product.status` | `products.status` | Default "active" if absent |
| `product.tags` | `products.tags` | Shopify sends JSON array of strings (not comma-separated); deserialized directly |
| `variant.id` | `product_variants.source_variant_id` | Stored as string |
| `variant.title` | `product_variants.title` | e.g. "12oz / 5mg THC" |
| `variant.sku` | `product_variants.sku` | |
| `variant.price` | `price_snapshots.price` | Shopify sends as decimal string |
| `variant.compare_at_price` | `price_snapshots.compare_at_price` | null if no sale |
| `variant.available` | `product_variants.is_available` | Default true if absent |
| `variant.position == 1` | `product_variants.is_default = true` | |
| Parsed from `variant.title` | `product_variants.dosage_mg` | `(\d+\.?\d*)\s*mg(?:\s+thc)?` |
| Parsed from `variant.title` | `product_variants.cbd_mg` | `(\d+\.?\d*)\s*mg\s+cbd` |
| Parsed from `variant.title` | `product_variants.size_value/unit` | `(\d+\.?\d*)\s*(oz\|ml)` |

## Internal Module Layout (`scbdb-scraper`)

| Module | Purpose |
|---|---|
| `client.rs` | `ShopifyClient` wrapping `reqwest::Client` with timeout + User-Agent |
| `types.rs` | Shopify response types (`ShopifyProductsResponse`, `ShopifyProduct`, `ShopifyVariant`) |
| `pagination.rs` | `extract_next_cursor()` — parses `Link` header for next page cursor |
| `normalize.rs` | `normalize_product()` — maps Shopify types → `scbdb_core::NormalizedProduct` |
| `error.rs` | `ScraperError` enum |
| `rate_limit.rs` | `retry_with_backoff` — exponential backoff retry executor; classifies `RateLimited` and `Http` errors as retriable, all others as non-retriable |

## Persistence Layer (`scbdb-db`)

New modules: `collection_runs.rs` and `products.rs`. Functions to implement:

**`collection_runs.rs`:** `create_collection_run`, `start_collection_run`, `complete_collection_run`, `fail_collection_run`, `get_collection_run`, `list_collection_runs`, `upsert_collection_run_brand`, `list_collection_run_brands`

**`products.rs`:** `upsert_product`, `upsert_variant`, `get_last_price_snapshot`, `insert_price_snapshot_if_changed`

## Testing Contract

- **Scraper:** wiremock tests in `crates/scbdb-scraper/tests/` — cover: empty product list, single page, multi-page pagination (Link header parsing), 429 rate limit response, 404 not found, malformed JSON
- **Persistence:** `#[sqlx::test]` in `crates/scbdb-db/tests/` — cover: product upsert idempotency, variant upsert, price snapshot dedup (no insert on unchanged price, insert on changed price), collection run lifecycle (queued → running → succeeded/failed)
- **Normalization:** unit tests in `normalize.rs` — cover: dosage/CBD/size parsing, empty tags, missing variants error, default variant selection

## CLI Contract

```bash
scbdb-cli collect products              # collect all active brands
scbdb-cli collect products --brand cann # collect single brand by slug
scbdb-cli collect products --dry-run    # preview without DB writes
scbdb-cli collect pricing               # snapshot all active brands
scbdb-cli collect pricing --brand cann  # snapshot single brand
```

Exit codes: `0` = success or partial success (per-brand failures are recorded in the DB and logged; the CLI exits `0` unless every brand fails). `1` = fatal error (all brands failed, or a run-level DB error occurred).
