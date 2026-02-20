# Sentiment Pipeline

## Overview

The sentiment pipeline collects brand-mention signals from Google News and Reddit, embeds them into a vector store for deduplication, scores them with a domain-specific lexicon, and writes a per-brand score snapshot to PostgreSQL.

It runs via `scbdb-cli sentiment collect` and is implemented in `crates/scbdb-sentiment`.

---

## Pipeline Steps (per brand)

```
collect_signals()
    │
    ├─ Google News RSS  →  up to 25 signals (title + description)
    └─ Reddit OAuth     →  up to 25 signals (title + selftext snippet)
            │
            ▼
    TeiClient::embed()
        POST /embed  →  1024-dim vectors (Qwen3-Embedding-0.6B)
        batched at 64 texts per request
            │
            ▼
    for each (signal, embedding):
        lexicon_score(signal.text)         →  score ∈ [-1.0, 1.0]
        QdrantClient::signal_exists(url)   →  SHA-256(url)[0..8] as u64 point ID
        if new → QdrantClient::upsert_signal()
            │
            ▼
    mean(scores)  →  BrandSentimentResult { score, signal_count }
            │
            ▼
    INSERT INTO sentiment_snapshots
```

---

## Sources

### Google News RSS

- **Endpoint**: `https://news.google.com/rss/search?q={brand_name}+hemp+OR+cbd+beverage`
- **Auth**: none
- **Output**: up to 25 `<item>` elements parsed as `SentimentSignal`
- **Signal text**: `title + " " + description` (HTML stripped)
- **TLS**: rustls (no issues)

### Reddit

- **Auth**: client-credentials OAuth via `POST https://www.reddit.com/api/v1/access_token`
  - Credentials: `REDDIT_CLIENT_ID`, `REDDIT_CLIENT_SECRET`
  - Token valid 24 hours; a fresh token is fetched per brand per run
- **Search endpoint**: `GET https://oauth.reddit.com/r/{subreddits}/search?q={brand_name}&restrict_sr=true&limit=25&sort=new`
- **Subreddits**: `delta8+hemp+cannabis+delta_8+hempflowers+CBD`
- **Signal text**: post title + up to 280 chars of selftext (if non-empty and not `[deleted]`)
- **TLS**: **native-tls (OpenSSL)** — `oauth.reddit.com` fingerprints rustls connections and returns 403; OpenSSL passes

---

## Deduplication

Point IDs in Qdrant are derived deterministically from the signal URL:

```
point_id = u64::from_be_bytes(SHA-256(url)[0..8])
```

Before upserting, the pipeline checks `GET /collections/{collection}/points/{id}`. If the point exists, the signal is skipped (logged at DEBUG). This means re-running the pipeline is idempotent — new articles are stored, old ones are skipped.

---

## Embedding

- **Model**: Qwen3-Embedding-0.6B via TEI (Text Embeddings Inference)
- **Dimension**: 1024
- **Distance**: Cosine
- **Batch size**: 64 texts per `/embed` request
- **Collection**: `SENTIMENT_QDRANT_COLLECTION` (default: `scbdb_sentiment`)
- **Collection creation**: automatic on first run if absent

Embeddings are stored in Qdrant alongside payload fields:

| Field | Value |
|-------|-------|
| `brand_slug` | e.g. `cann` |
| `source` | `google_news` or `reddit` |
| `url` | source URL (used for dedup) |
| `text` | scored text |
| `score` | lexicon score |

---

## Lexicon Scorer

The scorer splits text into words, strips leading/trailing non-alphabetic characters, lowercases, and looks up each word in a static domain lexicon. Scores accumulate and are clamped to `[-1.0, 1.0]`.

**Positive terms** (selection): `great` +0.4, `approved` +0.5, `legal` +0.4, `love` +0.5, `best` +0.5, `safe` +0.4

**Negative terms** (selection): `banned` -0.6, `illegal` -0.7, `recall` -0.7, `lawsuit` -0.5, `shutdown` -0.6, `prohibition` -0.6

See `crates/scbdb-sentiment/src/scorer.rs` for the full lexicon.

The final brand score is the **mean** across all signals. An empty signal set returns `0.0`.

---

## Configuration

All settings are read from environment variables at startup:

| Variable | Purpose | Example |
|----------|---------|---------|
| `SENTIMENT_TEI_URL` | TEI server URL | `http://localhost:18080` |
| `SENTIMENT_QDRANT_URL` | Qdrant server URL | `http://localhost:53333` |
| `SENTIMENT_QDRANT_COLLECTION` | Collection name | `scbdb_sentiment` |
| `REDDIT_CLIENT_ID` | Reddit OAuth app ID | `sB5I2MeHhv2H...` |
| `REDDIT_CLIENT_SECRET` | Reddit OAuth secret | `PjKYfxfed7iq...` |
| `REDDIT_USER_AGENT` | Reddit API user-agent | `scbdb/0.1.0` |

All six are required. Missing vars cause a startup error listing what's absent.

---

## CLI Reference

```bash
# Collect and score all active brands
scbdb-cli sentiment collect

# Show recent scores
scbdb-cli sentiment status [--brand <slug>]

# Generate markdown report
scbdb-cli sentiment report
```

---

## Error Handling

| Failure | Behavior |
|---------|---------|
| Google News fetch fails | WARN logged; brand continues with Reddit-only signals |
| Reddit token exchange fails | WARN logged; brand continues with RSS-only signals |
| Reddit search returns non-2xx | WARN logged; brand continues with RSS-only signals |
| TEI embed fails | Pipeline returns `Err` for that brand; CLI logs and continues to next |
| Qdrant upsert fails | WARN logged; signal is still scored and included in mean |
| Qdrant existence check fails | WARN logged; upsert attempted anyway |
| No signals collected | Returns neutral score `0.0`; snapshot is still written |

---

## Crate Layout

```
crates/scbdb-sentiment/src/
├── lib.rs           — public API surface
├── pipeline.rs      — run_brand_sentiment() orchestration
├── types.rs         — SentimentSignal, BrandSentimentResult, SentimentConfig
├── scorer.rs        — lexicon_score() + LEXICON table
├── embeddings.rs    — TeiClient (POST /embed, batching)
├── vector_store.rs  — QdrantClient (ensure, exists, upsert) + url_to_point_id()
├── error.rs         — SentimentError
└── sources/
    ├── mod.rs       — collect_signals() fan-out
    ├── rss.rs       — Google News RSS fetch + XML parse
    └── reddit.rs    — RedditClient (OAuth token exchange + search)
```
