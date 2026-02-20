# Phase 4: Sentiment Pipeline

## Document Metadata

- Version: 1.1
- Status: Complete
- Last Updated (EST): 00:00:00 | 02/20/2026 EST

## Objective

Implement sentiment signal collection, scoring, vector storage, and snapshot persistence for tracked brands.

## Target Outcomes

- Google News RSS and Reddit signal collection operational per brand.
- Domain-specific lexicon scorer producing normalized scores in [-1.0, 1.0].
- Signal embeddings stored in Qdrant for dedup and future semantic search.
- Sentiment snapshots persisted per brand in `sentiment_snapshots`.
- CLI commands for collection, status, and reporting.

## Deliverables

- `scbdb-sentiment` crate (sources, scorer, embeddings, vector store, pipeline)
- `scbdb-cli sentiment` subcommands (collect, status, report)
- Migration `20260220000600_sentiment_snapshots`
- Unit and integration tests covering scorer, RSS parsing, and vector ID stability

## Resolved Decisions

| Decision | Resolution |
|----------|-----------|
| Signal sources | Google News RSS (no key) + Reddit API (client-credentials OAuth) |
| Crawling engine | reqwest (HTTP) — Spider library not wired as service |
| Scoring methodology | Domain-specific lexicon; mean of signal scores; clamped [-1.0, 1.0] |
| Embedding use | TEI for vector generation; stored in Qdrant for dedup + future semantic search |
| Scoring primary | Lexicon (deterministic); embeddings stored for future cosine-distance enrichment |
| Qdrant collection | `scbdb_sentiment` (separate from Axon's `cortex`) |
| Qdrant point ID | First 8 bytes of SHA256(url) as u64 |
| Reddit scope | Subreddits: delta8, hemp, cannabis, delta_8, hempflowers + keyword search |
| Dry-run behavior | Skip DB writes AND Qdrant writes; preview brand list and estimated signal count |
| Per-brand failure | Continue to next; record in collection_run_brands |
| Empty signal set | score=0.0, signal_count=0; still persisted as a valid neutral snapshot |
| TEI batch size | 64 texts per /embed call |
| Reddit search limit | 25 posts per brand (one page; no pagination for MVP) |

## Signal Flow

```
Google News RSS (reqwest + XML)  ──┐
Reddit API (reqwest + OAuth)     ──┤─▶ SentimentSignal { text, url, source, brand_slug }
                                   │
                            for each signal:
                              1. TEI embed (POST /embed → Vec<f32> 1024-dim)
                              2. Qdrant upsert (dedup by url hash, skip if exists)
                              3. Lexicon score → f32 in [-1.0, 1.0]
                                   │
                            aggregate: mean(scores)
                                   │
                            INSERT sentiment_snapshots (brand_id, score, signal_count, metadata)
```

## Module Layout

```
crates/scbdb-sentiment/src/
├── lib.rs          — public exports
├── error.rs        — SentimentError (Http, Xml, Reddit, Qdrant, Tei, Normalization)
├── types.rs        — SentimentSignal, BrandSentimentResult, SentimentConfig
├── scorer.rs       — lexicon_score(text: &str) -> f32; LEXICON const (~50 domain terms)
├── sources/
│   ├── mod.rs      — collect_signals(config, brand, brand_name) -> Vec<SentimentSignal>
│   ├── rss.rs      — fetch_google_news_rss(brand_slug, brand_name) -> Vec<SentimentSignal>
│   └── reddit.rs   — RedditClient { token }, search_brand_mentions(brand_name)
├── embeddings.rs   — TeiClient { url }; embed(texts: &[&str]) -> Vec<Vec<f32>>
├── vector_store.rs — QdrantClient { url, collection }; ensure_collection(), upsert_signal(), signal_exists()
└── pipeline.rs     — run_brand_sentiment(config, brand) -> BrandSentimentResult
```

## Infrastructure

| Component | Config Var | Value |
|-----------|-----------|-------|
| TEI (embeddings) | `SENTIMENT_TEI_URL` | `http://localhost:52000` |
| Qdrant (vectors) | `SENTIMENT_QDRANT_URL` | `http://localhost:53333` |
| Qdrant collection | `SENTIMENT_QDRANT_COLLECTION` | `scbdb_sentiment` |
| Reddit OAuth | `REDDIT_CLIENT_ID`, `REDDIT_CLIENT_SECRET` | — |
| Reddit user agent | `REDDIT_USER_AGENT` | `scbdb/0.1.0` |

- TEI model: Qwen3-Embedding-0.6B, 1024-dimensional vectors
- Qdrant distance: Cosine
- Qdrant point ID: first 8 bytes of SHA256(url) as big-endian u64

## Scoring

The lexicon scorer operates on lowercase words with domain-specific weights:

- **Positive examples**: great (+0.4), excellent (+0.5), approved (+0.5), legal (+0.4), safe (+0.4)
- **Negative examples**: ban (-0.6), illegal (-0.7), recall (-0.7), dangerous (-0.6), lawsuit (-0.5)
- **Range**: clamped to [-1.0, 1.0]
- **Aggregation**: mean of all signal scores for the brand
- **Empty set**: score = 0.0 (neutral), still persisted

## Database

```sql
CREATE TABLE sentiment_snapshots (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    brand_id BIGINT NOT NULL REFERENCES brands(id),
    captured_at TIMESTAMPTZ NOT NULL,
    score NUMERIC(6,3) NOT NULL,
    signal_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_sentiment_snapshots_brand_captured ON sentiment_snapshots (brand_id, captured_at DESC);
```

Migration: `20260220000600_sentiment_snapshots`

## CLI Contract

```bash
# Collect sentiment signals
scbdb-cli sentiment collect                     # all active brands
scbdb-cli sentiment collect --brand cann        # single brand
scbdb-cli sentiment collect --dry-run           # preview without DB/Qdrant writes

# Query results
scbdb-cli sentiment status                      # recent scores for all brands
scbdb-cli sentiment status --brand cann         # single brand history

# Report
scbdb-cli sentiment report                      # markdown report all brands
scbdb-cli sentiment report --brand cann         # markdown report single brand
```

Exit codes: `0` = success, `1` = error. Partial failures continue; all-brands-failed exits `1`.

## Testing

| Module | Tests |
|--------|-------|
| `scorer.rs` | 9 unit tests: empty, whitespace, unknown, positive, negative, mixed, clamp both bounds, punctuation |
| `sources/rss.rs` | Valid RSS parse, empty feed, malformed XML |
| `vector_store.rs` | `url_to_point_id` stability, different URLs produce different IDs |

## Verification

```bash
just ci                            # full gate: fmt + clippy + test
just migrate                       # apply 20260220000600_sentiment_snapshots
scbdb-cli sentiment collect --dry-run --brand cann   # preview; no DB writes
scbdb-cli sentiment collect --brand cann             # full run (requires REDDIT_* + TEI + Qdrant)
scbdb-cli sentiment status
scbdb-cli sentiment report

# Infrastructure checks
curl http://localhost:53333/collections/scbdb_sentiment
curl -X POST http://localhost:52000/embed -H 'Content-Type: application/json' -d '{"inputs":["test"]}'
```
