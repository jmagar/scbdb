# Locations Dashboard Panel

## Overview

The Locations Dashboard is a React panel accessible from the fifth tab ("Locations") on the main API dashboard. It surfaces the data collected by the [Store Locator Crawler](./STORE_LOCATOR.md) in three views:

1. **Headline stat bar** — four KPIs across all tracked brands
2. **Interactive US coverage map** — state-level intensity heatmap (all 50 states + DC)
3. **Per-brand cards + state breakdown table** — granular distribution data

---

## API Endpoints

Both endpoints are protected (require bearer auth in production; open in development).

### `GET /api/v1/locations/summary`

Per-brand location stats for all brands with at least one active location.

**Response shape:**
```json
{
  "data": [
    {
      "brand_name": "Cann",
      "brand_slug": "cann",
      "active_count": 312,
      "new_this_week": 12,
      "states_covered": 18,
      "locator_source": "locally",
      "last_seen_at": "2026-02-21T02:00:00Z"
    }
  ],
  "meta": { "request_id": "...", "timestamp": "..." }
}
```

**SQL summary**: Aggregate over `store_locations` grouped by brand, filtered to `is_active = TRUE`. Uses `COUNT(*) FILTER (WHERE ...)` for `new_this_week` and `states_covered`. Correlated subquery picks the most-common `locator_source` per brand.

### `GET /api/v1/locations/by-state`

State-level location counts across all active locations.

**Response shape:**
```json
{
  "data": [
    { "state": "CA", "brand_count": 6, "location_count": 412 },
    { "state": "TX", "brand_count": 5, "location_count": 287 }
  ],
  "meta": { "request_id": "...", "timestamp": "..." }
}
```

**SQL summary**: `GROUP BY sl.state` with `COUNT(DISTINCT brand_id)` and `COUNT(*)`, ordered by `location_count DESC`. Skips rows where `state IS NULL OR state = ''`.

---

## UI Components

### Stat Bar

Four headline numbers computed client-side from the summary response:

| Stat | Source |
|------|--------|
| Active locations | `SUM(active_count)` across all brands |
| New this week | `SUM(new_this_week)` across all brands |
| States covered | `byState.data.length` (unique states in the by-state response) |
| Brands tracked | `summary.data.length` |

### US Coverage Map

A zero-dependency CSS tile grid (no npm map library). All 50 states + DC are positioned in an 11-column × 9-row grid approximating US geography.

**Tile intensity classes:**

| Class | Locations | Color |
|-------|-----------|-------|
| `tile-empty` | 0 | Gray (`#ebebeb`) |
| `tile-low` | 1–5 | Light teal (`#c8eae5`) |
| `tile-mid` | 6–25 | Medium teal (`#7ecbc0`) |
| `tile-high` | 26–100 | Strong teal (`#2fa394`) |
| `tile-max` | 100+ | Deep accent (`#0f7a6d`) |

Hover over any state to see a tooltip: state abbreviation, location count, brand count.

### Brand Cards

One card per brand, sorted by `active_count DESC`. Shows:
- Active location count
- New locations in the last 7 days
- Number of distinct US states covered
- Locator source badge (`Locally.com`, `Storemapper`, `JSON-LD`, `Embedded JSON`)
- Last seen timestamp

### State Breakdown Table

Mini-table of all states with active locations, sorted by `location_count DESC`. Shows location count and brand count per state.

---

## Empty State

When no location data has been collected yet (e.g., immediately after first deploy), all tile map cells render gray and the brand cards section is replaced by:

```text
No location data yet. Run collect locations to populate.
```

Run `scbdb-cli collect locations` to populate. See [STORE_LOCATOR.md](./STORE_LOCATOR.md) for collection instructions.

---

## Crate + File Layout

```text
crates/scbdb-db/src/
└── locations.rs             — list_locations_dashboard_summary(),
                               list_locations_by_state(),
                               LocationsDashboardRow, LocationsByStateRow

crates/scbdb-server/src/api/
└── locations.rs             — list_locations_summary handler,
                               list_locations_by_state handler

web/src/
├── types/api.ts             — LocationBrandSummary, LocationsByState
├── lib/api/dashboard.ts     — fetchLocationsSummary(), fetchLocationsByState()
├── hooks/use-dashboard-data.ts — useLocationsSummary(), useLocationsByState()
├── components/
│   ├── locations-panel.tsx  — LocationsPanel, StateTileMap (inline)
│   └── dashboard-page.tsx   — "locations" tab wiring
└── styles.css               — .tile-*, .locations-*, .source-badge
```

---

## Data Freshness

- **Stale time**: 60 seconds (matches all other dashboard hooks — defined in `use-dashboard-data.ts`)
- **Collection cadence**: Weekly (every Sunday 02:00 UTC via `scbdb-server` scheduler)
- **Manual refresh**: `scbdb-cli collect locations`

---

## Useful Queries

**Top states by coverage (all brands):**
```sql
SELECT state, brand_count, location_count
FROM (
  SELECT sl.state,
         COUNT(DISTINCT sl.brand_id) AS brand_count,
         COUNT(*) AS location_count
  FROM store_locations sl
  WHERE sl.is_active = TRUE AND sl.state IS NOT NULL
  GROUP BY sl.state
) s
ORDER BY location_count DESC
LIMIT 10;
```

**New locations this week (territory gain):**
```sql
SELECT b.name, sl.state, COUNT(*) AS new_locs
FROM store_locations sl
JOIN brands b ON b.id = sl.brand_id
WHERE sl.is_active = TRUE
  AND sl.first_seen_at > NOW() - INTERVAL '7 days'
GROUP BY b.name, sl.state
ORDER BY new_locs DESC;
```

---

## Known Limitations

- **State normalization** — The API displays whatever 2-letter code the scraper captured. If a locator source returns a full state name (e.g., "California" instead of "CA"), it will not match a tile and will appear in the state table as a non-standard row. Normalize in `raw_to_new_location()` in the CLI if this occurs.
- **DC tile** — District of Columbia is at grid position [8, 5]. It renders correctly if `state = 'DC'` in the DB.
- **Mobile tooltip clipping** — Tooltip for states near the right edge (ME, NH, RI, CT, MA) may clip on narrow viewports. A future pass should clamp the tooltip X position.
- **react-simple-maps not used** — The current implementation is a CSS tile grid, not a proper geographic projection. Qualitatively conveys state coverage; does not show actual state shapes or sizes.
