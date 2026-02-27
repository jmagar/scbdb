# Sentiment Dashboard

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 00:00:00 | 02/21/2026 EST

## Purpose

Defines the Sentiment tab data flow and transparency behavior for the dashboard.

## Data Flow

```text
sentiment_snapshots
  -> scbdb-db api queries
  -> scbdb-server /api/v1/sentiment/{summary,snapshots}
  -> web dashboard hooks
  -> web/src/components/sentiment-panel.tsx
```

## API Shape

Summary and snapshot items include:
- `brand_name`
- `brand_slug`
- `score`
- `signal_count`
- `captured_at`
- `metadata`

`metadata` includes:
- `source_counts?: Record<string, number>`
- `top_signals?: SentimentEvidence[]`

## UI Behavior

`SentimentPanel` renders:
- Market-level stats (positive/neutral/negative counts, market average)
- Brand-level sentiment cards
- Momentum deltas when previous snapshots exist
- **Data Transparency** section:
  - Source mix aggregated from `metadata.source_counts`
  - Evidence links derived from `metadata.top_signals`

Transparency behavior is generic; no source hardcoding is required for rendering counts/evidence.

## Source Transparency Expectations

Expected sources include (not exhaustive):
- `google_news`
- `bing_news`
- `yahoo_news`
- `reddit_post` / `reddit_comment`
- `brand_newsroom`
- `twitter`

If a new source is added upstream, it appears automatically in source mix when present in metadata.

## Testing

Coverage is validated in:
- `web/src/components/dashboard-page.test.tsx`

The sentiment fixture must include `brand_newsroom` in `metadata.source_counts` to ensure newsroom transparency remains visible.
