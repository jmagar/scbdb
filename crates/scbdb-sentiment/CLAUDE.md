# CLAUDE.md — scbdb-sentiment

Crate-level context for the `scbdb-sentiment` library. Supplements the root
`CLAUDE.md`; read both before making changes here.

## Purpose

Market sentiment pipeline for hemp-derived THC beverage brands. Collects
brand-mention signals from multiple news and social sources, embeds them via
TEI, deduplicates in Qdrant, scores them with a domain-specific lexicon, and
returns a per-brand aggregated `BrandSentimentResult` ready for database
persistence by `scbdb-cli`.

This is a **library crate** (`lib`). It has no `main`, no `.env` loading, no
database access. The CLI (`scbdb-cli`) owns orchestration, DB writes, and
dry-run logic.

## Workspace Dependencies

| Dependency | Role |
|-----------|------|
| `scbdb-core` | Shared error/domain types |
| `scbdb-db` | Used by CLI caller, not by this crate directly |
| `scbdb-cli` | Calls `run_brand_sentiment` + handles dry-run / DB persistence |

External crates of note: `reqwest` (HTTP), `quick-xml` (RSS parsing),
`sha2` (point-ID hashing), `regex`, `percent-encoding`, `wiremock` (tests).

## Public API

```rust
// Primary entry point — called once per brand by scbdb-cli
pub async fn run_brand_sentiment(
    config: &SentimentConfig,
    brand_slug: &str,
    brand_name: &str,
    brand_base_url: Option<&str>,  // domain preferred over shop_url
    twitter_handle: Option<&str>,  // None = skip brand-timeline Twitter fetch
) -> Result<BrandSentimentResult, SentimentError>

// Standalone scorer — useful for testing lexicon changes in isolation
pub fn lexicon_score(text: &str) -> f32   // returns [-1.0, 1.0]
```

Re-exported from `lib.rs`:
- `SentimentError`
- `SentimentConfig`
- `SentimentSignal`
- `BrandSentimentResult`
- `SignalEvidence`

## Key Types

```rust
// A single collected item before/after scoring
pub struct SentimentSignal {
    pub text: String,        // title + snippet concatenated
    pub url: String,         // canonical source URL (dedup key)
    pub source: String,      // source tag — see Signal Sources table below
    pub brand_slug: String,
    pub score: f32,          // 0.0 until lexicon_score() runs
}

// Aggregated result returned to the CLI for DB persistence
pub struct BrandSentimentResult {
    pub brand_slug: String,
    pub score: f32,                            // mean lexicon score, 0.0 if no signals
    pub signal_count: usize,
    pub source_counts: BTreeMap<String, usize>, // per-source breakdown
    pub top_signals: Vec<SignalEvidence>,        // top 5 by |score|, 220-char preview
}

// Serializable evidence row stored in snapshot metadata
pub struct SignalEvidence {
    pub source: String,
    pub url: String,
    pub score: f32,
    pub text_preview: String,  // first 220 chars of signal.text
}

// Config built from env vars — call SentimentConfig::from_env() at startup
pub struct SentimentConfig {
    pub tei_url: String,
    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub reddit_client_id: String,
    pub reddit_client_secret: String,
    pub reddit_user_agent: String,
    pub twitter_auth_token: Option<String>,  // None = Twitter source silently skipped
    pub twitter_ct0: Option<String>,
}
```

## Pipeline Steps

`run_brand_sentiment` executes these steps in order:

1. **Ensure Qdrant collection** — creates collection if absent (1024-dim cosine).
2. **Collect signals** — fan-out to all sources (see below); source failures are fail-open.
3. **Dedup by URL** — `HashSet` dedup across sources before embedding.
4. **Embed** — `TeiClient::embed` in batches of 64 via `POST {TEI_URL}/embed`.
5. **Qdrant dedup + upsert** — point ID = first 8 bytes of `SHA-256(url)` as u64; skip upsert if already present; upsert failures are warn-and-continue.
6. **Score** — `lexicon_score` applied to each signal text.
7. **Aggregate** — arithmetic mean of all signal scores; `top_signals` = 5 highest `|score|`.

Empty signal set returns `score: 0.0` immediately after step 2, skipping embedding.

## Signal Sources

| Source tag | Module | Limit | Notes |
|-----------|--------|-------|-------|
| `google_news` | `sources/rss.rs` | 50 | Google News RSS search |
| `bing_news` | `sources/bing_rss.rs` | configurable | Bing News RSS search |
| `yahoo_news` | `sources/yahoo_rss.rs` | configurable | Yahoo News RSS search |
| `brand_newsroom` | `sources/brand_newsroom/` | 10 articles | Crawls brand-owned domain |
| `reddit_post` / `reddit_comment` | `sources/reddit.rs` | 60 total | OAuth client-credentials; hemp/cannabis subreddits |
| `twitter` | `sources/twitter.rs` | 50/query | `bird` CLI subprocess; skipped if creds absent |
| `twitter_brand` | `sources/twitter.rs` | 20 | Brand's own timeline; requires `twitter_handle` param |
| `twitter_replies` | `sources/twitter.rs` | 20/tweet × 10 tweets | Replies to brand's top 10 tweets |
| `gdelt_news` | `sources/gdelt.rs` | 40 | GDELT Doc API (no auth) |

All sources are fail-open: a source error logs a warning and collection
continues with remaining sources.

## Scoring Methodology

`lexicon_score` in `scorer.rs`:
- Splits text on whitespace; strips leading/trailing non-alphabetic chars from each word.
- Looks up each word (lowercased) in `LEXICON` — a static slice of `(&str, f32)` pairs.
- Sums matching weights; clamps result to `[-1.0, 1.0]`.
- Returns `0.0` for empty or fully unknown text.

Domain-specific lexicon covers regulatory language (ban, illegal, recall,
restrict, prohibition) and product-quality language (great, quality, loved,
thriving). Weights range from `-0.7` to `+0.5`. Aggregate score is the
arithmetic mean across all signals for a brand.

## Brand Newsroom Crawl

`sources/brand_newsroom/` crawls brand-owned domains for press/news content:

1. Fetch `robots.txt` → extract sitemap refs.
2. Fetch `/sitemap.xml` (always attempted).
3. Enumerate common newsroom paths (`/news`, `/press`, `/blog`, etc.) plus
   LLM-inferred paths (optional, see env vars below).
4. Extract article URLs; filter out e-commerce paths (`/products`, `/collections`, `/shop`).
5. Fetch up to `MAX_ARTICLES_PER_BRAND` (10) article pages.
6. Extract text via priority chain: JSON-LD Article/NewsArticle → `og:title` + `meta[name=description]` → `<title>` + description → `<h1>` + first paragraph. LLM fallback as last resort.

Hard limits per brand: 12 sitemaps, 10 index pages, 10 articles, 4 LLM enrich calls, 8 LLM-discovered seed URLs.

## Qdrant Integration

- Collection auto-created on first run; name from `SENTIMENT_QDRANT_COLLECTION`.
- Vector dimension: **1024** (Qwen3-Embedding-0.6B via TEI).
- Distance metric: **Cosine**.
- Point ID: `u64` derived from `SHA-256(url)[0..8]` — stable and deterministic.
- Payload fields stored per point: `brand_slug`, `source`, `url`, `text`, `score`.
- Existence check via `GET /collections/{col}/points/{id}` — 200 = exists, skip upsert.

## TEI Integration

- Endpoint: `POST {SENTIMENT_TEI_URL}/embed`
- Request body: `{"inputs": ["text1", "text2", ...]}`
- Response: `Vec<Vec<f32>>` — one embedding vector per input.
- Batch size: **64** texts per request.
- Error if TEI returns fewer embeddings than inputs (contract violation warning logged).

## Environment Variables

`SentimentConfig::from_env()` is called by the CLI at runtime. All six required
vars must be present or the call returns `Err` listing the missing names.

| Variable | Required | Default | Notes |
|---------|---------|---------|-------|
| `SENTIMENT_TEI_URL` | Yes | — | e.g. `http://localhost:52000` |
| `SENTIMENT_QDRANT_URL` | Yes | — | e.g. `http://localhost:53333` |
| `SENTIMENT_QDRANT_COLLECTION` | Yes | — | Collection name string |
| `REDDIT_CLIENT_ID` | Yes | — | Reddit app client ID |
| `REDDIT_CLIENT_SECRET` | Yes | — | Reddit app client secret |
| `REDDIT_USER_AGENT` | Yes | — | Reddit API user-agent string |
| `TWITTER_AUTH_TOKEN` | No | — | If absent, all Twitter sources silently return empty |
| `TWITTER_CT0` | No | — | Must be set alongside `TWITTER_AUTH_TOKEN` |
| `SENTIMENT_NEWSROOM_LLM_ENABLED` | No | disabled | Set to `1` to enable LLM newsroom extraction |
| `OPENAI_API_KEY` | No | — | Required when `SENTIMENT_NEWSROOM_LLM_ENABLED=1` |
| `SENTIMENT_NEWSROOM_LLM_MODEL` | No | `gpt-4o-mini` | Override LLM model for newsroom extraction |

**Do not call `dotenvy::dotenv()` in this crate.** Only binary entrypoints load `.env`.

## Dry-Run Mode

Dry-run is handled entirely in `scbdb-cli`, not in this library. When
`--dry-run` is passed to `scbdb-cli sentiment collect`, the CLI prints the
brand list and returns before calling `run_brand_sentiment`. This crate has no
dry-run concept internally.

## Failure Behavior

| Failure | Behavior |
|--------|---------|
| Individual source HTTP/parse error | `warn!` log, source skipped, continue |
| All sources return empty | Score `0.0`, `signal_count: 0`, return `Ok` |
| Qdrant upsert failure | `warn!` log, signal still scored for snapshot |
| Qdrant existence-check failure | `warn!` log, upsert skipped, signal still scored |
| TEI request failure | `Err(SentimentError::Tei)` — brand run fails |
| Reddit OAuth failure | `warn!` log, Reddit source skipped |
| `bird` CLI not found | `Err(SentimentError::Twitter)` on spawn — logged as warn by `collect_signals` |

## Testing

Run unit tests in isolation (no external services needed):

```bash
cargo test -p scbdb-sentiment
```

Test coverage:
- `scorer.rs` — lexicon scoring edge cases (clamping, punctuation, empty input).
- `sources/rss.rs` — RSS XML parsing (valid, empty, malformed).
- `sources/brand_newsroom/` — URL canonicalization, sitemap parsing, article URL filtering, extraction priority chain.
- `vector_store.rs` — `url_to_point_id` stability.
- `sources/twitter.rs` — `BirdTweet` deserialization; no-creds fast-return.

Integration tests (require live services, marked `#[ignore]`):

```bash
# Twitter live (needs TWITTER_AUTH_TOKEN + TWITTER_CT0)
cargo test -p scbdb-sentiment twitter_live -- --ignored --nocapture
cargo test -p scbdb-sentiment brand_timeline_live -- --ignored --nocapture
```

There are no integration tests for TEI or Qdrant; those paths are covered by
manual `just serve` + `scbdb-cli sentiment collect --brand <slug>` runs.

## Module Map

```
src/
├── lib.rs                          # Public re-exports, run_brand_sentiment entry point
├── config.rs                       # SentimentConfig, from_env()
├── error.rs                        # SentimentError enum
├── pipeline.rs                     # Pipeline orchestration: collect → dedup → embed → score → aggregate
├── scorer.rs                       # lexicon_score(), LEXICON static slice
├── tei.rs                          # TeiClient — POST /embed, batch size 64
├── vector_store.rs                 # QdrantClient — ensure_collection, point exists check, upsert, url_to_point_id
├── types.rs                        # SentimentSignal, BrandSentimentResult, SignalEvidence
└── sources/
    ├── mod.rs                      # collect_signals() — fan-out to all sources, fail-open
    ├── rss.rs                      # google_news — Google News RSS search
    ├── bing_rss.rs                 # bing_news — Bing News RSS search
    ├── yahoo_rss.rs                # yahoo_news — Yahoo News RSS search
    ├── reddit.rs                   # reddit_post / reddit_comment — OAuth client-credentials
    ├── twitter.rs                  # twitter / twitter_brand / twitter_replies — `bird` CLI subprocess
    ├── gdelt.rs                    # gdelt_news — GDELT Doc API (no auth)
    └── brand_newsroom/
        ├── mod.rs                  # Orchestrates sitemap → index → article crawl
        ├── crawl.rs                # HTTP fetch helpers, article text extraction
        ├── extract.rs              # JSON-LD / og:title / h1 extraction priority chain
        └── filter.rs               # URL filters (e-commerce path exclusion)
```

## Code Conventions

- All `pub(crate)` modules under `sources/` — sources are internal implementation details; only `collect_signals` and individual fetch fns are `pub(crate)`.
- All public functions have `/// # Errors` docstrings.
- `#[must_use]` on pure functions (`lexicon_score`, `url_to_point_id`, constructors).
- `tracing::{debug, info, warn, error}` with structured fields (`brand = brand_slug`, `source = "..."`, `error = %e`).
- Source tags (the `source` field on `SentimentSignal`) are static string literals defined at their source of origin, not a shared enum. Add new sources there and they propagate automatically through `source_counts` and the dashboard.

## Adding a New Signal Source

1. Create `src/sources/<name>.rs` with a `pub(crate) async fn fetch_<name>_signals(...) -> Result<Vec<SentimentSignal>, SentimentError>`.
2. Register it in `src/sources/mod.rs` — add `mod <name>` and call the fetch function inside `collect_signals`. Follow the existing fail-open pattern (match + warn on error).
3. Use a unique lowercase string literal for `signal.source` (this becomes the key in `source_counts`).
4. Add the source tag to `docs/SENTIMENT_PIPELINE.md` Sources section and `docs/SENTIMENT_DASHBOARD.md` Source Transparency Expectations.
5. Add the source to the dashboard fixture in `web/src/components/dashboard-page.test.tsx` if it should always appear in transparency output.
