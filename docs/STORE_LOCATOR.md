# Store Locator: Architecture & Brand Status

The store locator crawler builds a live registry of where each brand's products are sold at retail. It fetches each brand's "where to buy" page, detects which locator service is in use, and pulls structured location data. The strategic goal is **territory monitoring**: detect when competitors gain or lose retail distribution.

Runs via `scbdb-cli collect locations`, scheduled weekly by `scbdb-server`. Implementation lives in `crates/scbdb-scraper/src/locator/` and `crates/scbdb-db/src/locations/`.

---

## Table of Contents

1. [Detection Pipeline](#detection-pipeline)
2. [Format Reference](#format-reference)
3. [Grid Search System](#grid-search-system)
4. [Territory Change Detection](#territory-change-detection)
5. [Trust Scoring](#trust-scoring)
6. [Brand Status](#brand-status)
7. [Onboarding Unknown Formats](#onboarding-unknown-formats)
8. [Database Schema](#database-schema)
9. [CLI Reference](#cli-reference)
10. [Useful Queries](#useful-queries)
11. [Crate Layout](#crate-layout)

---

## Detection Pipeline

`fetch_store_locations(locator_url)` in `locator/mod.rs` tries 13 extraction strategies in priority order. First one that returns a non-empty result wins.

```text
fetch_store_locations(locator_url)
    │
    ├─  1. Locally.com widget
    │      Signal: "locally.com" or "locallyWidgetCompanyId" in HTML
    │      Extract: company_id via regex
    │      Fetch: GET api.locally.com/stores/json?company_id={id}&take=10000
    │
    ├─  2. Storemapper widget (token variant)
    │      Signal: "storemapper" in HTML
    │      Extract: token from data-storemapper-token, API URL, or JS var
    │      Fetch: GET storemapper.co/api/stores?token={token}
    │
    ├─  2b. Storemapper widget (user-id variant)
    │      Extract: data-storemapper-id attribute
    │      Fetch: GET storemapper.co/api/stores?user_id={id}
    │
    ├─  3. Stockist widget
    │      Signal: stockist widget tag in HTML
    │      Fallback: also checks linked /pages/dealers page
    │      Fetch: Stockist API with widget tag
    │
    ├─  4. Storepoint widget
    │      Signal: storepoint widget-id in HTML
    │      Fetch: Storepoint API
    │
    ├─  5. Roseperl / Secomapp WTB
    │      Signal: roseperl/secomapp where-to-buy JS URL in HTML
    │      Fetch: WTB JSON endpoint
    │
    ├─  6. VTInfo iframe
    │      Signal: "finder.vtinfo.com" in HTML
    │      Extract: custID (+ optional UUID) from iframe src or script var
    │      Fetch: iframe HTML → POST search form × 9 strategic US cities
    │      Search breaks at 100 deduplicated results (see Grid Search System)
    │
    ├─  7. AskHoodie widget
    │      Signal: askhoodie embed ID in HTML
    │      Fetch: AskHoodie API
    │
    ├─  8. BeverageFinder widget
    │      Signal: beveragefinder key in HTML
    │      Fetch: BeverageFinder API
    │
    ├─  9. WordPress Agile Store Locator
    │      Signal: "agile-store-locator" in HTML
    │      Fallback: also checks linked /store-locator subpage
    │      Fetch: WordPress AJAX admin-ajax.php endpoint
    │
    ├─ 10. StoreRocket widget
    │      Signal: storerocket account ID discoverable from HTML or JS
    │      Fetch: StoreRocket API
    │
    ├─ 11. Destini / lets.shop
    │      Signal: lets.shop domain or destini JS in HTML or linked resources
    │      Extract: alpha_code + locator_id from bootstrap JSON
    │      Fetch: Knox API × 168 CONUS grid points (see Grid Search System)
    │      Dedup: 4-decimal lat/lng fingerprint
    │
    ├─ 12. Schema.org JSON-LD
    │      Find: <script type="application/ld+json"> blocks
    │      Filter: @type in {LocalBusiness, Store, FoodEstablishment, ...}
    │             (@type as string or array both supported)
    │
    ├─ 13. Embedded JSON fallback
    │      Scan: all <script> tag contents
    │      Detect: JSON arrays where objects have name + city/lat/address fields
    │      Extract: balanced bracket walk → serde_json parse
    │
    └─ No match → tracing::warn! "no parseable locator found"; Ok(vec![])
```

---

## Format Reference

| # | Source ID | Detection | API Style | Notes |
|---|-----------|-----------|-----------|-------|
| 1 | `locally` | `locally.com` in HTML | REST GET, single call | Full dataset per call |
| 2 | `storemapper` | `storemapper` in HTML | REST GET, single call | Token or user-id variant |
| 3 | `stockist` | widget tag in HTML | REST | Also probes /pages/dealers |
| 4 | `storepoint` | widget-id in HTML | REST | |
| 5 | `roseperl` | Secomapp WTB JS URL | REST | Shopify app |
| 6 | `vtinfo` | `finder.vtinfo.com` in HTML | POST form × 9 cities | Breaks at 100 deduped results |
| 7 | `askhoodie` | embed ID in HTML | REST | |
| 8 | `beveragefinder` | key in HTML | REST | |
| 9 | `agile_store_locator` | `agile-store-locator` in HTML | WordPress AJAX | Also probes /store-locator subpage |
| 10 | `storerocket` | account discoverable from HTML | REST | |
| 11 | `destini` | lets.shop domain | Knox POST × 168 grid points | CONUS grid, ~84s runtime |
| 12 | `jsonld` | `<script type="application/ld+json">` | Static HTML | Fallback, lower confidence |
| 13 | `json_embed` | JSON arrays in `<script>` tags | Static HTML | Last resort, lowest confidence |

---

## Grid Search System

Two formats need systematic geographic coverage because their APIs are radius-limited:

### VTInfo — Strategic US Points

VTInfo searches from a fixed set of city centers. The loop **breaks at 100 deduplicated results** — so city order matters.

```rust
// locator/grid.rs — STRATEGIC_US_POINTS (order is intentional)
pub const STRATEGIC_US_POINTS: &[GridPoint] = &[
    { lat: 39.8283, lng: -98.5795 },  // [0] Kansas — US geographic center
    { lat: 35.2271, lng: -80.8431 },  // [1] Charlotte — SE coverage (SC/NC/GA/VA) — MUST be ≤ index 1
    { lat: 44.9778, lng: -93.2650 },  // [2] Minneapolis — Upper Midwest
    { lat: 34.0522, lng: -118.2437 }, // [3] Los Angeles — West Coast
    { lat: 40.7128, lng: -74.0060 },  // [4] New York — Northeast
    { lat: 41.8781, lng: -87.6298 },  // [5] Chicago — Great Lakes
    { lat: 29.7604, lng: -95.3698 },  // [6] Houston — Gulf Coast
    { lat: 39.7392, lng: -104.9903 }, // [7] Denver — Mountain West
    { lat: 33.4484, lng: -112.0740 }, // [8] Phoenix — Southwest
];
```

**Why Charlotte is at index 1:** VTInfo breaks its search loop the moment `dedup.len() >= 100`. For a large national brand (Cann, Uncle Arnie's, etc.), the first city alone may push past 100 results. Charlotte at index 1 guarantees Southeast coverage is always searched before the break fires.

**Pacing:** Per-brand randomized delay (350–750ms) between city requests.

### Destini — CONUS Grid

Destini uses Knox API with a configurable search radius (default 100 miles). A single-point search only captures one region. The crawler sweeps the entire continental US.

```rust
// locator/grid.rs — GridConfig::conus_coarse()
GridConfig {
    min_lat: 24.4, max_lat: 49.4,
    min_lng: -125.0, max_lng: -66.9,
    step_miles: 200.0,
    // → 168 grid points
}
```

**Coverage characteristics:**
- **168 points** at 200-mile spacing across CONUS
- **41-mile corner gap**: diagonal distance between adjacent cell centers = 141 mi > 100-mi radius. Dead zones exist at cell corners in rural areas. Accepted trade-off — these regions have negligible hemp beverage retail.
- **Canadian overshoot**: `max_lat: 49.4` + 0.5-step = ~10 grid points at ~50.5°N. Knox API filters non-US results server-side; no bad data enters the DB, but ~10 extra HTTP calls fire per brand.
- **Runtime**: 168 calls × 500ms pacing ≈ 84s per brand. Worst case with 30s per-point timeout: 168 × 30s = 84 min (bounded, not infinite).

**Deduplication**: Results across all 168 points are collected into a flat `Vec`, then `dedup_by_coordinates()` deduplicates by 4-decimal lat/lng fingerprint (`"{lat:.4},{lng:.4}"`). Coordinate-less stores bypass the dedup map and are kept unconditionally.

### SC Region Grid (available, unused)

```rust
GridConfig::sc_region() → 82 points, 30-mile step
// Bounds: lat 32.0–35.2, lng -83.4–-78.5
// Available for future SC-specific targeted scraping
```

---

## Territory Change Detection

Each location is identified by a stable dedup key:

```text
location_key = SHA-256(brand_id ‖ name.lower().trim() ‖ city.lower().trim() ‖ state.upper().trim() ‖ zip.trim())
```

Computed before every upsert — same physical store produces the same key across runs regardless of minor upstream data variation.

**Per collection run per brand:**
1. **Snapshot** active `location_key` set before upsert (`is_active = TRUE` only)
2. **Upsert** all scraped locations — new rows get `first_seen_at = NOW()`; existing rows get `last_seen_at = NOW()`, `is_active = TRUE`
3. **Deactivate** locations absent from current scrape — `is_active = FALSE`
4. **Log diff** — added/removed counts at INFO level

**Known log imprecision:** A store that was previously deactivated and reappears is logged as "new store locations detected" rather than "reactivated". The DB state is correct (the row was upserted). Distinguishing the two cases requires a second query including inactive rows.

---

## Trust Scoring

`locator/trust.rs` applies a pre-persistence trust gate. High-confidence providers (vtinfo, stockist, storemapper, destini, etc.) pass automatically. Low-confidence providers (json_embed, jsonld) require ≥3 locations with ≥2 having valid state data. Empty results always fail.

---

## Brand Status

Live as of 2026-02-21. `active_stores` reflects the current DB state.

| Brand | Slug | Tier | Locator URL | Format | Active Stores | Status |
|-------|------|------|-------------|--------|---------------|--------|
| High Rise | `high-rise` | Portfolio | highrisebev.com/pages/store-locator | `vtinfo` | 88 | ✅ |
| Cann | `cann` | 1 | drinkcann.com/pages/store-locator | `vtinfo` | 103 | ✅ |
| BRĒZ | `brz` | 1 | drinkbrez.com/pages/storelocator | `stockist` | 97 | ✅ |
| Cycling Frog | `cycling-frog` | 1 | cyclingfrog.com/pages/store-locator | **custom (CF-blocked)** | 0 | ❌ Cloudflare Managed Challenge blocks all scraping |
| Wynk | `wynk` | 1 | drinkwynk.com/pages/store-locator | `vtinfo` | 72 | ✅ |
| Uncle Arnie's | `uncle-arnies` | 1 | unclearnies.com/pages/store-locator | `vtinfo` | 114 | ✅ |
| Keef Brands | `keef-brands` | 1 | keefbrands.com/find-products | `storemapper` | 4,341 | ✅ |
| Cantrip | `cantrip` | 1 | drinkcantrip.com/pages/contact | `vtinfo` | 65 | ✅ (contact page works) |
| Wana | `wana` | 1 | wanabrands.com/find/ | `askhoodie` | 69 | ✅ |
| Ayrloom | `ayrloom` | 2 | ayrloom.com/pages/where-to-buy-new-york | `vtinfo` | 56 | ✅ |
| Better Than Booze | `better-than-booze` | 2 | drinkbetterthanbooze.com/pages/store-locator | `stockist` | 98 | ✅ |
| Drink Delta | `drink-delta` | 2 | drinkdelta.com/store-locator | `storerocket` | 5,679 | ✅ |
| Find Wunder | `find-wunder` | 2 | findwunder.com/guides/where-to-buy-cdn-drinks/ | `agile_store_locator` | 1,408 | ✅ |
| Green Monke | `green-monke` | 2 | greenmonkehemp.com/store-locator | `storepoint` | 1,214 | ✅ |
| Happy Flower | `happy-flower` | 2 | drinkhappyflower.com/pages/retailers | — | 0 | ⚠️ "Coming soon" placeholder — no locator configured |
| Island Chill | `island-chill` | 2 | islandbrandsusa.com/pages/beverage-finder | `beveragefinder` | 0 | ⚠️ Detected but BeverageFinder account is empty |
| Levity | `levity` | 2 | drinklevity.com/pages/store-locator | `roseperl` | 879 | ✅ |
| Mary Jones | `mary-jones` | 2 | — | — | 0 | ⚠️ No locator URL (retail-only) |
| Dad Grass | `dad-grass` | 2 | dadgrass.com/pages/locations | `stockist` | 93 | ✅ |
| Trail Magic | `trail-magic` | 2 | drinktrailmagic.com/pages/find | `vtinfo` | 58 | ✅ |
| Señorita Drinks | `seorita-drinks` | 3 | senoritadrinks.com/pages/store-locator | `vtinfo` | 89 | ✅ |
| Recess | `recess` | 3 | takearecess.com/where-to-buy | `destini` | 3,967 | ✅ |
| Buzzn | `buzzn` | 3 | drinkbuzzn.com/pages/store-locator-1 | `stockist` | 76 | ✅ |
| Adaptaphoria | `adaptaphoria` | 3 | — | — | 0 | ⚠️ No locator URL configured |

### Brands Needing Investigation

Investigated 2026-02-21. All 5 zero-store brands diagnosed; none are code bugs.

| Brand | Root Cause | Resolution Path |
|-------|-----------|-----------------|
| `cycling-frog` | Custom bounding-box API at `findyourride.cyclingfrog.com`. Entire domain (and API subdomain) behind Cloudflare Managed Challenge — blocks reqwest AND Playwright headless. URL in brands.yaml was wrong (`/pages/where-to-buy` → fixed to `/pages/store-locator`). | Would require a new `cycling_frog` format extractor + Cloudflare bypass (residential proxy or official API partnership). Not automated without CF bypass. |
| `island-chill` | BeverageFinder widget IS detected (key `690cda4f900046.11446315`). `embed-search.php` returns 0 stores for all US zip codes tested. | Brand hasn't populated their BeverageFinder account. Monitor — data will appear automatically once they add stores. |
| `happy-flower` | `/pages/retailers` page is a "coming soon" placeholder ("Happy Flower is coming soon to a retailer near you!"). No locator widget exists. | No locator configured yet. Re-check in future; update brands.yaml with actual locator URL when they launch retail. |
| `mary-jones` | Site returns 502 (server error). No `store_locator_url` in brands.yaml (retail-only brand). | Retail-only distribution; no locator to add. Skip. |
| `adaptaphoria` | No `store_locator_url` in brands.yaml. Standard locator paths return 404. | No public store locator. Skip. |

---

## Onboarding Unknown Formats

For brands where auto-detection fails, use the Playwright discovery script to intercept API calls:

```bash
cd scripts
pnpm install
npx playwright install chromium --with-deps

# Intercepts XHR/fetch responses and identifies location-like JSON
pnpm discover https://cyclingfrog.com/pages/where-to-buy 2>&1 | jq '.[].url'
```

**Output:** JSON array of intercepted requests with `url`, `method`, `postData`, and `sampleResponse`. Redirect output to `config/locators/<brand>.json` for review.

**If no calls intercepted**, the locator likely:
1. Requires a zip code trigger before loading stores (user interaction)
2. Embeds store data statically in HTML (check page source for `<script>` arrays or JSON-LD)
3. Is blocked by anti-bot (test in a real browser first)

**Adding a new format extractor:**
1. Create `crates/scbdb-scraper/src/locator/formats/<name>.rs`
2. Implement `extract_<name>_*` (HTML detection) and `fetch_<name>_stores` (API call)
3. Re-export from `formats/mod.rs`
4. Add strategy block to `fetch_store_locations` in `locator/mod.rs` at the appropriate priority position
5. Add `locator_source = "<name>"` string constant in your extractor

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
| `external_id` | `TEXT` | Provider-native store ID if available |
| `locator_source` | `TEXT` | Source ID string (see Format Reference) |
| `raw_data` | `JSONB` | Full source object for future enrichment |
| `first_seen_at` | `TIMESTAMPTZ` | First collection run that found this location |
| `last_seen_at` | `TIMESTAMPTZ` | Most recent run that confirmed it active |
| `is_active` | `BOOLEAN` | `FALSE` when absent from the latest scrape |
| `created_at` | `TIMESTAMPTZ` | |
| `updated_at` | `TIMESTAMPTZ` | |

Unique constraint: `(brand_id, location_key)`.
Indexes: `brand_id`, `state`, `first_seen_at DESC`, `(brand_id, is_active)`.

### `brands.store_locator_url`

Nullable TEXT. Seeded from `config/brands.yaml`. Auto-discovery falls back to probing common Shopify paths when null.

---

## CLI Reference

```bash
# Collect all brands
scbdb-cli collect locations

# Single brand
scbdb-cli collect locations --brand cann

# Debug — see which strategy fired and what it returned
RUST_LOG=scbdb_scraper=debug cargo run -p scbdb-cli -- collect locations --brand recess

# Dry-run (no DB writes; not yet implemented — use --brand in dev instead)
```

---

## Useful Queries

```sql
-- Active locations by brand and source
SELECT b.slug, sl.locator_source, COUNT(*) FILTER (WHERE sl.is_active) AS active_stores
FROM store_locations sl
JOIN brands b ON b.id = sl.brand_id
GROUP BY b.slug, sl.locator_source
ORDER BY active_stores DESC;

-- New retail distribution in the last 7 days
SELECT b.name, sl.name, sl.city, sl.state, sl.first_seen_at
FROM store_locations sl
JOIN brands b ON b.id = sl.brand_id
WHERE sl.first_seen_at > NOW() - INTERVAL '7 days'
  AND sl.is_active = TRUE
ORDER BY sl.first_seen_at DESC;

-- State-level competitive presence
SELECT sl.state, COUNT(DISTINCT sl.brand_id) AS brands, COUNT(*) AS locations
FROM store_locations sl
WHERE sl.is_active = TRUE
GROUP BY sl.state
ORDER BY brands DESC;

-- Brands with no detected locator format
SELECT b.slug, b.store_locator_url
FROM brands b
LEFT JOIN store_locations sl ON sl.brand_id = b.id AND sl.is_active = TRUE
WHERE b.store_locator_url IS NOT NULL
  AND sl.id IS NULL
ORDER BY b.slug;
```

---

## Crate Layout

```text
crates/scbdb-scraper/src/locator/
├── mod.rs               — fetch_store_locations(): 13-strategy detection cascade
├── fetch.rs             — HTTP fetching with user-agent rotation and retry
├── types.rs             — RawStoreLocation, LocatorError
├── grid.rs              — GridPoint, GridConfig (conus_coarse/sc_region), STRATEGIC_US_POINTS,
│                          generate_grid()
├── trust.rs             — validate_store_locations_trust(), make_location_key()
└── formats/
    ├── mod.rs           — re-exports all extractors
    ├── locally.rs       — Locally.com
    ├── storemapper.rs   — Storemapper (token + user-id variants)
    ├── stockist.rs      — Stockist
    ├── storepoint.rs    — Storepoint
    ├── roseperl.rs      — Roseperl / Secomapp
    ├── vtinfo/
    │   ├── mod.rs       — fetch_vtinfo_stores(), vtinfo_search_points()
    │   ├── vtinfo_http.rs  — HTTP client, retry/backoff, pacing
    │   └── vtinfo_parse.rs — HTML parsing, dedup key
    ├── askhoodie.rs     — AskHoodie
    ├── beveragefinder.rs — BeverageFinder
    ├── agile_store_locator.rs — WordPress Agile Store Locator
    ├── storerocket.rs   — StoreRocket
    ├── destini/
    │   ├── mod.rs       — config discovery
    │   ├── parse.rs     — fetch_destini_stores(), CONUS grid loop, dedup_by_coordinates()
    │   └── response.rs  — Knox response parsing
    ├── jsonld.rs        — Schema.org JSON-LD
    └── embed.rs         — Embedded JSON fallback

crates/scbdb-db/src/locations/
├── mod.rs               — public re-exports
├── read.rs              — get_active_location_keys_for_brand(), list_active_locations_by_brand()
└── write.rs             — upsert_store_locations(), deactivate_missing_locations()

crates/scbdb-cli/src/collect/locations/
├── mod.rs               — run_collect_locations(), per-brand orchestration
└── helpers.rs           — load_brands_for_locations(), log_location_changeset(), raw_to_new_location()

scripts/
└── discover-locator.ts  — Playwright XHR interception for unknown formats

migrations/
├── 20260221000200_store_locator_url.{up,down}.sql
└── 20260221000300_store_locations.{up,down}.sql
```
