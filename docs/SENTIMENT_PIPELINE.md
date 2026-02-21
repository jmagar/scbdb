# Sentiment Pipeline

## Overview

The sentiment pipeline collects multi-source brand signals, scores them, and writes one snapshot per brand into `sentiment_snapshots`.

Run command:
```bash
scbdb-cli sentiment collect [--brand <slug>] [--dry-run]
```

Primary implementation:
- `crates/scbdb-sentiment/src/pipeline.rs`
- `crates/scbdb-sentiment/src/sources/mod.rs`
- `crates/scbdb-cli/src/sentiment/mod.rs`

## Sources

Current source fan-out (`collect_signals`):
- `google_news` (Google News RSS)
- `bing_news` (Bing RSS)
- `yahoo_news` (Yahoo RSS)
- `brand_newsroom` (brand-owned domains)
- `reddit_post` / `reddit_comment` (Reddit)
- `twitter` (bird CLI, optional)

All source failures are fail-open; the brand run continues with remaining sources.

## Brand Newsroom Discovery

`brand_newsroom` uses deterministic discovery from brand base URL (`domain` first, fallback `shop_url`):

1. `robots.txt` sitemap references
2. `/sitemap.xml`
3. common newsroom index paths (`/news`, `/press`, `/blog`, etc.)

Guardrails:
- `MAX_SITEMAPS_PER_BRAND`
- `MAX_INDEX_PAGES_PER_BRAND`
- `MAX_ARTICLES_PER_BRAND`
- `MIN_TEXT_LEN`
- `MAX_LLM_FALLBACKS_PER_BRAND`

Extraction fallback priority:
1. `og:title` + `meta[name=description]`
2. `<title>` + `meta[name=description]`
3. first `<h1>` + first meaningful paragraph
4. optional LLM JSON extraction (`title` + `summary`) when deterministic extraction fails

URLs are canonicalized (drop query/fragment, normalize trailing slash) before emission.

### Optional LLM extraction mode

For weak/noisy pages, an LLM fallback can be enabled:

- `SENTIMENT_NEWSROOM_LLM_ENABLED=1`
- `OPENAI_API_KEY=<key>`
- optional `SENTIMENT_NEWSROOM_LLM_MODEL` (default: `gpt-4o-mini`)

Behavior:
- only used when deterministic extraction fails
- bounded by `MAX_LLM_FALLBACKS_PER_BRAND`
- fail-open (LLM errors never fail the brand run)

## Snapshot Metadata

Each snapshot stores transparency metadata:
- `source_counts`: per-source signal counts
- `top_signals`: strongest evidence rows (`source`, `url`, `score`, `text_preview`)
- `brand_slug`, `captured_at`, `version`

Example metadata shape:
```json
{
  "version": 1,
  "brand_slug": "cann",
  "source_counts": {
    "google_news": 50,
    "reddit_post": 20,
    "brand_newsroom": 4
  },
  "top_signals": [
    {
      "source": "google_news",
      "url": "https://example.com/story",
      "score": 0.7,
      "text_preview": "..."
    }
  ],
  "captured_at": "2026-02-21T05:00:00Z"
}
```

## Scoring and Storage

For each collected signal:
1. embed text via TEI
2. dedup by URL in Qdrant
3. lexicon-score signal text
4. aggregate mean score per brand

Pipeline output:
- `score`
- `signal_count`
- `source_counts`
- `top_signals`

## Failure Behavior

- Source request/parsing failures: warning/debug logs, continue.
- TEI failure for a brand: brand run fails, collection continues to next brand.
- Qdrant upsert failure: signal can still be scored for snapshot; warning logged.
- Empty signal set: neutral score `0.0` with empty source/evidence metadata.
