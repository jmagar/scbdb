# Sentiment Dashboard

## Document Metadata

- Version: 1.0
- Status: Active
- Last Updated (EST): 21:30:00 | 02/20/2026 EST

## Purpose

Describes the Sentiment tab in the web dashboard — its data flow, API endpoints, UI components, and CSS classes.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Database Layer](#database-layer)
4. [API Endpoints](#api-endpoints)
5. [Frontend Data Layer](#frontend-data-layer)
6. [UI Components](#ui-components)
7. [CSS Reference](#css-reference)
8. [Score Semantics](#score-semantics)

---

## Overview

The Sentiment tab surfaces brand sentiment data collected by the `scbdb-sentiment` pipeline. It has two sections:

- **Brand Score Cards** — one card per active brand showing its most recent sentiment score, signal count, and a visual meter bar.
- **Recent Runs** — a mini-table of the last 8 sentiment snapshots across all brands in chronological descending order.

---

## Architecture

The feature follows the same layered pattern as the Pricing tab:

```
sentiment_snapshots table
        ↓
scbdb-db: list_sentiment_summary / list_sentiment_snapshots_dashboard
        ↓
scbdb-server: GET /api/v1/sentiment/summary, GET /api/v1/sentiment/snapshots
        ↓
web/src/lib/api/dashboard.ts: fetchSentimentSummary / fetchSentimentSnapshots
        ↓
web/src/hooks/use-dashboard-data.ts: useSentimentSummary / useSentimentSnapshots
        ↓
web/src/components/sentiment-panel.tsx: SentimentPanel
```

---

## Database Layer

### Schema

```sql
sentiment_snapshots (
  id           BIGINT        PRIMARY KEY,
  brand_id     BIGINT        NOT NULL REFERENCES brands(id),
  captured_at  TIMESTAMPTZ   NOT NULL,
  score        NUMERIC(6,3)  NOT NULL,   -- range [-1.000, 1.000]
  signal_count INTEGER       NOT NULL,
  metadata     JSONB,
  created_at   TIMESTAMPTZ   NOT NULL
)
```

### Row Types (`crates/scbdb-db/src/api_queries.rs`)

```rust
pub struct SentimentSummaryRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}

pub struct SentimentSnapshotDashboardRow {
    pub brand_name: String,
    pub brand_slug: String,
    pub score: Decimal,
    pub signal_count: i32,
    pub captured_at: DateTime<Utc>,
}
```

Separate row types are used despite identical shapes — summary uses `DISTINCT ON (brand_id)` semantics while snapshots return an ordered feed.

### Query Functions

**`list_sentiment_summary(pool)`**

Returns the most recent snapshot per active brand using PostgreSQL's `DISTINCT ON`:

```sql
SELECT b.name AS brand_name, b.slug AS brand_slug,
       ss.score, ss.signal_count, ss.captured_at
FROM (
    SELECT DISTINCT ON (brand_id)
        brand_id, score, signal_count, captured_at
    FROM sentiment_snapshots
    ORDER BY brand_id, captured_at DESC, id DESC
) ss
JOIN brands b ON b.id = ss.brand_id
WHERE b.deleted_at IS NULL AND b.is_active = TRUE
ORDER BY b.name
```

**`list_sentiment_snapshots_dashboard(pool, limit)`**

Returns recent snapshots across all brands:

```sql
SELECT b.name AS brand_name, b.slug AS brand_slug,
       ss.score, ss.signal_count, ss.captured_at
FROM sentiment_snapshots ss
JOIN brands b ON b.id = ss.brand_id
WHERE b.deleted_at IS NULL
ORDER BY ss.captured_at DESC, ss.id DESC
LIMIT $1
```

---

## API Endpoints

Both endpoints require `Authorization: Bearer <api_key>` when `SCBDB_API_KEYS` is set.

### `GET /api/v1/sentiment/summary`

Returns the most recent sentiment score for each active brand.

**Response:**

```json
{
  "data": [
    {
      "brand_name": "Cann",
      "brand_slug": "cann",
      "score": "0.420",
      "signal_count": 18,
      "captured_at": "2026-02-20T00:00:00Z"
    }
  ],
  "meta": { "request_id": "req_abc", "timestamp": "..." }
}
```

Note: `score` is a JSON string (not number) because `rust_decimal::Decimal` serializes as a decimal string to preserve precision.

### `GET /api/v1/sentiment/snapshots`

Returns recent sentiment snapshots, newest first.

**Query parameters:**

| Param | Type | Default | Max | Description |
|-------|------|---------|-----|-------------|
| `limit` | integer | 50 | 200 | Number of snapshots to return |

**Response:** Same shape as `/sentiment/summary`.

---

## Frontend Data Layer

### TypeScript Types (`web/src/types/api.ts`)

```typescript
export type SentimentSummaryItem = {
  brand_name: string;
  brand_slug: string;
  score: string;        // decimal string, e.g. "0.420"
  signal_count: number;
  captured_at: string;  // ISO 8601
};

export type SentimentSnapshotItem = {
  brand_name: string;
  brand_slug: string;
  score: string;
  signal_count: number;
  captured_at: string;
};
```

### Fetch Functions (`web/src/lib/api/dashboard.ts`)

```typescript
fetchSentimentSummary()   → Promise<SentimentSummaryItem[]>
fetchSentimentSnapshots() → Promise<SentimentSnapshotItem[]>  // passes limit=30
```

### Hooks (`web/src/hooks/use-dashboard-data.ts`)

```typescript
useSentimentSummary()   // queryKey: ["sentiment-summary"]
useSentimentSnapshots() // queryKey: ["sentiment-snapshots"]
```

Both use `STALE_TIME_MS` (5 minutes) and TanStack Query's default refetch behavior.

---

## UI Components

### `SentimentPanel` (`web/src/components/sentiment-panel.tsx`)

Receives two query result objects as props:

```typescript
type Props = {
  summary:   { isLoading: boolean; isError: boolean; data: SentimentSummaryItem[] | undefined };
  snapshots: { isLoading: boolean; isError: boolean; data: SentimentSnapshotItem[] | undefined };
};
```

**Renders:**

1. Loading/error states for both sections.
2. Brand score cards — one `<article class="data-card">` per brand:
   - Header: brand name + `sentiment-badge sentiment-badge--{positive|negative|neutral}` with formatted score.
   - Meter bar: `sentiment-meter` track with `sentiment-meter-indicator` positioned by `scorePct(score)`.
   - Detail list: signal count + last updated date.
3. Recent Runs mini-table (`role="table"`, `aria-label="recent-sentiment-snapshots"`) — last 8 snapshots.

### Score Helper Functions (`web/src/components/dashboard-utils.tsx`)

```typescript
// Formats "-0.42" → "-0.42", "0.42" → "+0.42"
formatScore(value: string): string

// Maps score to badge modifier class
scoreClass(value: string): "positive" | "negative" | "neutral"
// positive: score > +0.05
// neutral:  |score| < 0.05
// negative: score < -0.05

// Maps [-1, 1] to [0%, 100%] for meter indicator position
scorePct(value: string): number
```

---

## CSS Reference

Defined in `web/src/styles.css`.

### Badge

```css
.sentiment-badge               /* base pill: IBM Plex Mono, border-radius 99px */
.sentiment-badge--positive     /* green: var(--accent-soft) bg, var(--accent) text */
.sentiment-badge--negative     /* red: #fdecea bg, var(--warn) text */
.sentiment-badge--neutral      /* grey: #f0f0f0 bg, var(--muted) text */
```

### Meter Bar

```css
.sentiment-meter               /* 6px gradient track: red → neutral → green */
.sentiment-meter-indicator     /* 12px circle dot, positioned absolutely by scorePct() */
```

The gradient runs left (negative/red `#bd3e2b`) through a neutral centre (`#d8dedc`, 45–55%) to right (positive/green `#0f7a6d`).

### Stats Grid

The top stats grid uses 4 columns on desktop (`repeat(4, minmax(0, 1fr))`), stacking to 1 column on mobile. The Sentiment stat card is the fourth entry.

---

## Score Semantics

Scores come from the `scbdb-sentiment` pipeline (Google News RSS + Reddit sources), normalized to `[-1.000, 1.000]` via the TEI embedding model and stored in `sentiment_snapshots.score`.

| Range | Interpretation | Badge class |
|-------|---------------|-------------|
| `> +0.05` | Net positive sentiment | `--positive` (green) |
| `-0.05` to `+0.05` | Neutral / insufficient signal | `--neutral` (grey) |
| `< -0.05` | Net negative sentiment | `--negative` (red) |

The `signal_count` field indicates how many articles/posts contributed to the score. Low signal counts (`< 5`) should be interpreted with caution.
