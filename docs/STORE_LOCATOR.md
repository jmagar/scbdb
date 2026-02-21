# Store Locator Crawler

## Overview

The store locator crawler builds a live registry of where each brand's products are sold at retail. It fetches each brand's "where to buy" page, detects which locator service is in use, and pulls structured location data — recording `first_seen_at` per location across continuous scheduled runs. The strategic goal is **territory monitoring**: detect when competitors gain new retail distribution.

It runs via `scbdb-cli collect locations` and is scheduled weekly by `scbdb-server`. The implementation lives in `crates/scbdb-scraper/src/locator/` (module directory with `mod.rs`, `fetch.rs`, `types.rs`, `grid.rs`, `trust.rs`, and `formats/` submodule for per-service extractors) and `crates/scbdb-db/src/locations.rs`.

---

## Extraction Pipeline

```text
fetch_store_locations(locator_url)
    │
    ├─ 1. Fetch page HTML with reqwest (user-agent rotation, timeout)
    │
    ├─ 2. Locally.com
    │      Detect: html contains "locally.com" or "locallyWidgetCompanyId"
    │      Extract: company_id via regex
    │      Call: GET api.locally.com/stores/json?company_id={id}&take=10000
    │      → RawStoreLocation[] (locator_source = "locally")
    │
    ├─ 3. Storemapper
    │      Detect: html contains "storemapper"
    │      Extract: token via regex (data attribute, URL, or JS var)
    │      Call: GET storemapper.co/api/stores?token={token}
    │      → RawStoreLocation[] (locator_source = "storemapper")
    │
    ├─ 4. Schema.org JSON-LD
    │      Find: all <script type="application/ld+json"> blocks
    │      Filter: @type == "LocalBusiness", "Store", or "FoodEstablishment"
    │              (handles @type as string or array)
    │      Map: streetAddress, addressLocality, addressRegion, postalCode,
    │           addressCountry, geo.latitude, geo.longitude, telephone
    │      → RawStoreLocation[] (locator_source = "jsonld")
    │
    ├─ 5. Embedded JSON fallback
    │      Scan: all <script> tag contents
    │      Detect: JSON arrays where objects have name + city/lat/address fields
    │      Extract: balanced bracket walk → serde_json parse
    │      → RawStoreLocation[] (locator_source = "json_embed")
    │
    └─ 6. No locator found
           tracing::warn! logged; Ok(vec![]) returned
```

Strategies are tried in order; the first that returns results wins. API calls (Locally.com, Storemapper) return complete datasets directly — no JS rendering required.

---

## Territory Change Detection

Each location is identified by a **stable dedup key**:

```text
location_key = SHA-256(brand_id ‖ name.lower().trim() ‖ city.lower().trim() ‖ state.upper().trim() ‖ zip.trim())
```

Computed in Rust before every upsert, so the same physical store produces the same key across runs regardless of minor upstream data variation.

On each collection run per brand:

1. **Upsert** all scraped locations — new rows get `first_seen_at = NOW()`; existing rows get `last_seen_at = NOW()` and `is_active = TRUE`.
2. **Deactivate** any location whose `location_key` is absent from the current scrape — `is_active = FALSE`. An empty result deactivates all.

This gives you a queryable record of:

- When a brand **first appeared** in a store (distribution gain)
- When a brand **left** a store (distribution loss)
- Which brands are active in a given state right now

---

## Auto-Discovery

When a brand has no `store_locator_url` configured in `brands.yaml`, the crawler probes common Shopify URL patterns via HEAD request:

```text
/pages/where-to-buy
/pages/store-locator
/pages/find-us
/pages/locations
/pages/retailers
/locator
/stores
```

The first path that returns a 2xx response is used. Discovered URLs are stored in `brands.store_locator_url` in the DB (the yaml remains authoritative; re-seeding overwrites back to the yaml value).

---

## Database Schema

### `store_locations`

| Column | Type | Notes |
|--------|------|-------|
| `id` | `BIGINT GENERATED ALWAYS AS IDENTITY` | PK |
| `public_id` | `UUID` | External-facing ID |
| `brand_id` | `BIGINT` | FK → `brands.id` |
| `location_key` | `TEXT` | SHA-256 dedup key; unique per brand |
| `name` | `TEXT` | Store name |
| `address_line1` | `TEXT` | Street address |
| `city` | `TEXT` | |
| `state` | `TEXT` | 2-letter US state code or province |
| `zip` | `TEXT` | |
| `country` | `TEXT` | Default `'US'` |
| `latitude` | `NUMERIC(9,6)` | |
| `longitude` | `NUMERIC(9,6)` | |
| `phone` | `TEXT` | |
| `external_id` | `TEXT` | Locally.com or Storemapper native ID |
| `locator_source` | `TEXT` | `locally`, `storemapper`, `jsonld`, `json_embed` |
| `raw_data` | `JSONB` | Full source object for future enrichment |
| `first_seen_at` | `TIMESTAMPTZ` | First collection run that found this location |
| `last_seen_at` | `TIMESTAMPTZ` | Most recent run that found it active |
| `is_active` | `BOOLEAN` | `FALSE` when absent from the latest scrape |
| `created_at` | `TIMESTAMPTZ` | |
| `updated_at` | `TIMESTAMPTZ` | |

Unique constraint: `(brand_id, location_key)`.

Indexes: `brand_id`, `state`, `first_seen_at DESC`, `(brand_id, is_active)`.

### `brands.store_locator_url`

Added in migration `20260221000200`. Nullable TEXT column. Seeded from `brands.yaml` via `just seed`.

### `collection_runs.run_type`

The `CHECK` constraint was extended to include `'locations'` alongside `products`, `pricing`, `regs`, and `sentiment`.

---

## Brand Configuration

Add `store_locator_url` to any brand in `config/brands.yaml`:

```yaml
- name: Cann
  relationship: competitor
  tier: 1
  domain: drinkcann.com
  shop_url: https://drinkcann.com/collections/all
  store_locator_url: https://drinkcann.com/pages/where-to-buy
```

Omit the field to rely on auto-discovery. Pre-configured brands:

| Brand | URL |
|-------|-----|
| High Rise | `https://highrisebev.com/pages/where-to-buy` |
| Cann | `https://drinkcann.com/pages/where-to-buy` |
| BRĒZ | `https://drinkbrez.com/pages/where-to-buy` |
| Cycling Frog | `https://cyclingfrog.com/pages/where-to-buy` |
| Wynk | `https://drinkwynk.com/pages/where-to-buy` |
| Cantrip | `https://drinkcantrip.com/pages/where-to-buy` |

---

## CLI Reference

```bash
# Dry-run — preview brands without DB writes
scbdb-cli collect locations --dry-run

# Collect all brands that have a store_locator_url (or auto-discover)
scbdb-cli collect locations

# Single brand
scbdb-cli collect locations --brand cann
```

**Output format:**

```text
Collecting store locations for 18 brands...
  ✓ cann          312 active (+12 new, 0 lost)  [locally]
  ✓ brez           89 active (+1 new, 0 lost)   [storemapper]
  ✓ cycling-frog   47 active (+0 new, 2 lost)   [jsonld]
  ✗ wynk          scrape failed: no parseable locator
Run complete: 1,847 total active locations, 14 new this run
```

---

## Scheduler

`scbdb-server` registers a weekly locations job at startup:

- **Schedule**: Every Sunday at 02:00 UTC (`0 0 2 * * SUN`)
- **Scope**: All brands with `store_locator_url IS NOT NULL`
- **Implementation**: `crates/scbdb-server/src/scheduler.rs`

The scheduler uses `tokio-cron-scheduler` and runs entirely in-process — no external job queue required.

---

## Useful Queries

**New retail distribution since yesterday:**
```sql
SELECT b.name, sl.name, sl.city, sl.state, sl.locator_source, sl.first_seen_at
FROM store_locations sl
JOIN brands b ON b.id = sl.brand_id
WHERE sl.first_seen_at > NOW() - INTERVAL '1 day'
  AND sl.is_active = TRUE
ORDER BY sl.first_seen_at DESC;
```

**Active locations by brand and source:**
```sql
SELECT b.name, sl.locator_source, COUNT(*) AS active,
       COUNT(*) FILTER (WHERE sl.first_seen_at > NOW() - INTERVAL '7 days') AS new_this_week
FROM store_locations sl
JOIN brands b ON b.id = sl.brand_id
WHERE sl.is_active = TRUE
GROUP BY b.name, sl.locator_source
ORDER BY active DESC;
```

**State-level competitive presence:**
```sql
SELECT sl.state, COUNT(DISTINCT sl.brand_id) AS brands, COUNT(*) AS locations
FROM store_locations sl
WHERE sl.is_active = TRUE
GROUP BY sl.state
ORDER BY brands DESC;
```

---

## Error Handling

| Failure | Behavior |
|---------|---------|
| HTTP non-2xx on locator page | Falls through to `Ok(vec![])` with `WARN` logged |
| No strategy matches | `WARN` logged; brand recorded as failed in `collection_run_brands` |
| Locally.com API error | `LocatorError::Http` returned; CLI logs and continues to next brand |
| Storemapper API error | Same as above |
| DB upsert fails | `WARN` logged; brand recorded as failed; run continues |
| `store_locator_url` not set and discovery fails | Brand skipped; WARN logged |

---

## Crate Layout

```text
crates/scbdb-scraper/src/
└── locator/                 — module directory
    ├── mod.rs               — public API, strategy orchestration
    ├── fetch.rs             — HTTP fetching with user-agent rotation
    ├── types.rs             — RawStoreLocation, LocatorError
    ├── grid.rs              — location grid/key utilities
    ├── trust.rs             — trust scoring for extracted locations
    └── formats/             — per-service extraction strategies
        ├── locally.rs       — Locally.com API extraction
        ├── storemapper.rs   — Storemapper API extraction
        ├── jsonld.rs        — Schema.org JSON-LD extraction
        ├── embed.rs         — Embedded JSON fallback
        └── ...              — additional format extractors

crates/scbdb-db/src/
└── locations.rs             — upsert_store_locations(), deactivate_missing_locations(),
                               list_new_locations_since(), list_active_locations_by_brand(),
                               NewStoreLocation, StoreLocationRow

crates/scbdb-cli/src/collect/
└── locations.rs             — run_collect_locations(), per-brand orchestration,
                               discover_locator_url(), summary output

crates/scbdb-server/src/
├── api/locations.rs         — list_locations_summary, list_locations_by_state handlers
└── scheduler.rs             — build_scheduler(), weekly locations job

migrations/
├── 20260221000200_store_locator_url.up.sql
├── 20260221000200_store_locator_url.down.sql
├── 20260221000300_store_locations.up.sql
└── 20260221000300_store_locations.down.sql
```
