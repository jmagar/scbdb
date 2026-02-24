# CLAUDE.md — scbdb-profiler

Brand intelligence profiler crate. Collects signals from RSS feeds, YouTube, and Twitter (X), embeds them via TEI, and upserts into PostgreSQL. The embedding→Qdrant write path is partially wired — vectors are fetched from TEI but Qdrant upsert is not yet implemented (TODO in `intake.rs`).

## Module Map

| Module | Responsibility |
|--------|---------------|
| `types` | `CollectedSignal`, `BrandProfileRunResult` — the two core data shapes |
| `intake` | Pipeline orchestrator: collect → embed → upsert. Entry point for callers. |
| `rss` | RSS/Atom feed crawler via `feed-rs` (format-agnostic) |
| `youtube` | YouTube Data API v3 client (search endpoint, `part=snippet`) |
| `twitter` | X/Twitter via `t` ruby-gem CLI — synchronous subprocess, graceful-degradation |
| `embedder` | TEI `/embed` HTTP client + deterministic Qdrant point-ID derivation |
| `error` | `ProfilerError` enum: `Http`, `Db`, `Json`, `Other` |

## Public API

```rust
// Primary pipeline entry point
scbdb_profiler::intake::ingest_signals(
    pool: &PgPool,
    config: &IntakeConfig,
    brand_id: i64,
    feed_urls: &[String],          // RSS/Atom feed URLs from brand_domains
    youtube_channel_id: Option<&str>,
    twitter_handle: Option<&str>,   // WITHOUT the @ sign
) -> Result<BrandProfileRunResult, ProfilerError>

// Config (constructed by caller — server scheduler or future CLI command)
IntakeConfig {
    client: reqwest::Client,    // share across requests
    tei_url: String,            // e.g. "http://localhost:52000"
    youtube_api_key: Option<String>,
}
```

Re-exported at crate root: `ProfilerError`, `IntakeConfig`, `BrandProfileRunResult`, `CollectedSignal`.

**`IntakeConfig` note:** the `client` field must be a caller-owned `reqwest::Client`. Build one with `reqwest::Client::new()` or a custom builder and share it across brands. Do not construct a new client per `ingest_signals` call.

**`ProfilerError` variants:**

```rust
pub enum ProfilerError {
    Http(reqwest::Error),      // HTTP request failure (RSS, YouTube, TEI)
    Db(scbdb_db::DbError),     // Database upsert failure
    Json(serde_json::Error),   // JSON deserialization failure (YouTube API response)
    Other(String),             // subprocess errors (twitter t gem), misc
}
```

## Pipeline Flow (intake.rs)

```
1. RSS:     for each feed_url → rss::crawl_feed()       → Vec<CollectedSignal>
2. YouTube: if channel_id + api_key → youtube::collect_channel_signals()
3. Twitter: if handle → twitter::collect_profile_signals() (best-effort, never fatal)
4. For each signal:
   a. Build embed text: "title\n\nsummary" (or whichever is present)
   b. Derive content_key: external_id ?? source_url ?? title ?? uuid::Uuid::new_v4()
   c. Compute qdrant_point_id = SHA-256(content_key)[0..16] as UUID-format string
   d. Call TEI embed_text() — failure is logged at DEBUG, does NOT block DB upsert
   e. Call scbdb_db::upsert_brand_signal() — ON CONFLICT DO UPDATE
5. Return BrandProfileRunResult { signals_collected, signals_upserted, errors: Vec<String> }
```

Individual collector failures accumulate into `errors` rather than aborting the run.

## Signal Types Written

| Collector | `signal_type` | `source_platform` |
|-----------|---------------|-------------------|
| RSS/Atom  | `"article"`   | domain extracted from feed URL |
| YouTube   | `"youtube_video"` | `"youtube"` |
| Twitter   | `"tweet"`     | `"twitter"` |

These values must match the `brand_signal_type` enum in PostgreSQL. The full enum includes additional types (`blog_post`, `reddit_post`, `newsletter`, etc.) not yet collected by this crate.

## Deduplication

Two-layer strategy:

1. **Qdrant point ID** — `embedder::signal_point_id(content_key)` SHA-256's the content key and formats the first 16 bytes as a UUID string (`8-4-4-4-12` hex). Stored in `brand_signals.qdrant_point_id`. Same content always maps to the same ID.

2. **DB UNIQUE constraint** — `UNIQUE (brand_id, signal_type, external_id)` on `brand_signals`. The `upsert_brand_signal` call uses `ON CONFLICT DO UPDATE`. Important caveat: `external_id IS NULL` bypasses this constraint (PostgreSQL allows multiple NULLs in a UNIQUE index) — signals without an external ID are never deduplicated at the DB level.

Content key precedence: `external_id` > `source_url` > `title` > `uuid::Uuid::new_v4()` (last resort, produces a non-deterministic ID).

## TEI Integration

- Endpoint: `POST {TEI_URL}/embed` with body `{ "inputs": "<text>" }`
- Response: `Vec<Vec<f32>>` — crate takes `[0]`
- Failure is non-fatal: logged at `DEBUG`, DB upsert still proceeds
- Qdrant write after embedding is a **TODO** in `intake.rs` — vectors are fetched but not stored

TEI URL is distinct from `SENTIMENT_TEI_URL`. Both may point to the same service (`http://localhost:52000`) but are configured separately.

## Environment Variables

| Variable | Where Read | Default | Required |
|----------|-----------|---------|----------|
| `TEI_URL` | `scbdb-server` scheduler reads and passes via `IntakeConfig::tei_url` | `""` (embedding silently skipped) | No |
| `YOUTUBE_API_KEY` | `scbdb-server` scheduler reads and passes via `IntakeConfig::youtube_api_key` | `None` (YouTube skipped) | No |
| `BRAND_INTAKE_CRON` | `scbdb-server` scheduler | `"0 0 6 * * *"` | No |

This crate does NOT call `dotenvy::dotenv()` — it is a library. Env vars are read by `scbdb-server` (scheduler) and injected through `IntakeConfig`.

## Twitter Collector Gotchas

- Requires the `t` ruby gem (`gem install t`) to be installed on the host.
- Invoked as a subprocess: `t timeline -n <limit> <handle>`. Runs synchronously inside `tokio::task::spawn_blocking`.
- If `t` is not found or exits non-zero, the module returns `Ok(vec![])` with a `WARN` log. It is **never** a fatal error.
- CLI output format is TSV: `ID\t@handle\tYYYY-MM-DD HH:MM:SS\ttweet text`. Lines with fewer than 4 columns are silently skipped. Bad timestamps produce a signal with `published_at: None`.
- Twitter handle is passed WITHOUT the `@` sign.

## YouTube Collector Gotchas

- Uses the `search` endpoint with `part=snippet`, `type=video`, `order=date`. Does NOT call `videos` for statistics (view/like/comment counts) — those fields remain `None`.
- `nextPageToken` is deserialized but unused — pagination is not implemented.
- Items with a missing `videoId` (playlists slipping through) are silently filtered via `filter_map`.
- Thumbnail selection: `high` > `medium` > `default`.
- Datetime parsing: RFC 3339 (`parse_from_rfc3339`) → converted to UTC.

## RSS Collector Gotchas

- Uses `feed-rs` which handles RSS 0.9x, RSS 1.0, RSS 2.0, Atom 0.3/1.0, and JSON Feed.
- `Accept` header explicitly lists `application/rss+xml, application/atom+xml, text/xml, application/xml`.
- `summary` is truncated to 2000 characters (character boundary — multi-byte safe).
- `external_id` = `entry.id` (always present in `feed-rs`).
- Entries without links produce a `CollectedSignal` with `source_url: None`.
- `source_platform` is extracted from the feed URL host (e.g. `"blog.example.com"`).

## Database Interaction

Uses `scbdb_db::upsert_brand_signal(pool, &NewBrandSignal { ... })`. The `content` field is always `None` — full-text extraction is a future enhancement. The `qdrant_point_id` is stored per-signal to cross-reference into Qdrant once the write path is wired.

The `scbdb-server` scheduler calls these `scbdb-db` functions to determine which brands to target and which feeds to pass in — this crate does not call them directly:
- `list_brands_without_profiles` — initial intake targeting
- `list_brands_needing_signal_refresh` — stale signal targeting (>24h)
- `list_brand_feed_urls` — RSS URLs per brand
- `list_brand_social_handles` — platform + handle pairs per brand
- `list_brands_with_stale_handles` — weekly handle verification

## Workspace Dependencies

```
scbdb-profiler
├── scbdb-core   (domain types, shared error)
└── scbdb-db     (upsert_brand_signal, NewBrandSignal, DbError)
```

`scbdb-profiler` itself is depended upon only by `scbdb-server` (scheduler) at present.

## Testing

All tests run offline — no live HTTP calls, no DB required:

```bash
# Crate-level tests only
cargo test -p scbdb-profiler

# With output
cargo test -p scbdb-profiler -- --nocapture
```

Test coverage by module:
- `embedder`: deterministic point-ID properties + UUID format validation
- `intake`: `build_embed_text` combinations, `IntakeConfig` clone
- `rss`: `extract_domain` variants, `truncate` boundaries, RSS + Atom parse (inline XML)
- `twitter`: `parse_cli_output` edge cases (malformed lines, bad dates, empty input), `tweet_to_signal` mapping, `truncate` with multibyte chars
- `youtube`: `parse_youtube_datetime` (valid, offset, invalid), `best_thumbnail` fallback chain, `truncate`, `SearchResponse` deserialization (missing videoId, no page token)

Live integration (requires running services):
```bash
# Requires DATABASE_URL, TEI_URL, YOUTUBE_API_KEY set in .env
# No dedicated integration test yet — exercised via server scheduler or manual:
DATABASE_URL=postgres://scbdb:...@localhost:15432/scbdb \
TEI_URL=http://localhost:52000 \
YOUTUBE_API_KEY=... \
cargo run -p scbdb-server
```

## Scheduled Execution (scbdb-server)

Three cron jobs in `scbdb-server/src/scheduler/brand_intel.rs`:

| Job | Schedule | Trigger |
|-----|----------|---------|
| `brand_intake` | `0 0 6 * * *` (daily 06:00 UTC) | Brands without profiles |
| `signal_refresh` | `0 0 4 * * *` (daily 04:00 UTC) | Brands with signals older than 24h |
| `handle_refresh` | `0 0 5 * * SUN` (weekly Sunday 05:00 UTC) | Brands with stale social handles (logs only, no upsert yet) |

`BRAND_INTAKE_CRON` overrides the intake schedule; refresh and handle jobs use hardcoded crons.

## Known TODOs

- `intake.rs`: Qdrant upsert — embedding is computed but the vector is dropped (`_embedding`). Wire a Qdrant HTTP client to persist the vector.
- `youtube.rs`: Pagination via `nextPageToken` is not implemented (max 50 results per run).
- `youtube.rs`: `view_count`, `like_count`, `comment_count` are not populated — requires a separate `videos?part=statistics` call.
- `scbdb_db::brand_signals`: `content` field is always `None` — full-text extraction deferred.
- Handle verification job logs stale handles but does not HTTP-check or refresh follower counts.
