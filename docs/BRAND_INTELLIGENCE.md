# Brand Intelligence Layer

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
   - [Data Model](#data-model)
   - [Signal Collection Pipeline](#signal-collection-pipeline)
   - [API Endpoints](#api-endpoints)
   - [Dashboard](#dashboard)
3. [Configuration](#configuration)
4. [Completeness Scoring](#completeness-scoring)

---

## Overview

The Brand Intelligence Layer provides automated competitive intelligence collection for tracked
brands. It collects signals from multiple external sources (RSS feeds, YouTube, Twitter/X),
embeds them via TEI for semantic search, and presents them through the Brand Profile dashboard
at `/brands` and `/brands/:slug`.

The layer was introduced in Phase 6 and lives across three crates:

- **`scbdb-profiler`** -- signal collection, TEI embedding, DB upsert
- **`scbdb-db`** -- typed query modules for all 13 brand intelligence tables
- **`scbdb-server`** -- 10 REST endpoints under `/api/v1/brands/`

---

## Architecture

### Data Model

The Brand Intelligence Layer adds 13 tables to the core schema. They fall into three groups:

**Profile & Presence (3 tables)**

| Table | Purpose |
|---|---|
| `brand_profiles` | Extended static metadata: tagline, description, HQ, founding year, funding stage, CEO, employee count |
| `brand_social_handles` | Per-platform handle registry with follower counts and verified status |
| `brand_domains` | All domains owned by a brand -- primary site, redirects, regional TLDs |

**Signal Stream (1 table + enum)**

| Table | Purpose |
|---|---|
| `brand_signals` | Unified raw signal stream from all sources. One row per collected item. Deduplicated on `(brand_id, signal_type, external_id)`. Qdrant point ID stored inline. |

Signal types (`brand_signal_type` enum): `article`, `blog_post`, `tweet`, `youtube_video`,
`reddit_post`, `newsletter`, `press_release`, `podcast_episode`, `event`, `award`,
`partnership`, `launch`.

**Structured Intelligence (8 tables)**

| Table | Purpose |
|---|---|
| `brand_funding_events` | Investment rounds, acquisitions -- amount, investors, acquirer |
| `brand_lab_tests` | CoA results per variant -- THC/CBD mg actual, pass/fail, report URL |
| `brand_legal_proceedings` | Lawsuits, FDA warning letters, state enforcement actions |
| `brand_sponsorships` | Sports, event, athlete, and influencer deals |
| `brand_distributors` | Distribution relationships by state array and channel type; GIN-indexed states |
| `brand_competitor_relationships` | Ordered brand pairs with `CHECK (brand_id < competitor_brand_id)` to prevent duplicates |
| `brand_newsletters` | Newsletter subscription tracking with inbox address |
| `brand_media_appearances` | Podcast episodes, press articles, video interviews |

**Run Tracking (1 table)**

| Table | Purpose |
|---|---|
| `brand_profile_runs` | Per-brand profiler run log with task-level progress counters and `partial` status |

All brand intelligence tables use `BIGINT GENERATED ALWAYS AS IDENTITY` PKs and `TIMESTAMPTZ`
timestamps, consistent with the core schema conventions.

### Signal Collection Pipeline

The pipeline runs in `scbdb-profiler::intake::ingest_signals` and is called by the
`scbdb-server` scheduler (default: 06:00 UTC daily). It processes one brand per invocation:

```
1. RSS crawl
   scbdb-profiler::rss::crawl_feed(client, brand_id, feed_url)
   --> Vec<CollectedSignal>

2. YouTube collection  [requires YOUTUBE_API_KEY]
   scbdb-profiler::youtube::collect_channel_signals(client, brand_id, channel_id, api_key, 50)
   --> Vec<CollectedSignal>

3. Twitter collection  [optional, best-effort]
   scbdb-profiler::twitter::collect_profile_signals(brand_id, handle, 50)
   --> Vec<CollectedSignal>

4. For each signal:
   a. Build embed text from title + summary
   b. Derive deterministic Qdrant point ID from external_id / source_url / title
   c. POST to TEI /embed  [non-fatal: failure does not block DB upsert]
   d. scbdb_db::upsert_brand_signal(pool, &NewBrandSignal)
      --> ON CONFLICT (brand_id, signal_type, external_id) DO UPDATE

5. Return BrandProfileRunResult { signals_collected, signals_upserted, errors }
```

Collector failures are captured in `BrandProfileRunResult::errors` and do not abort the
pipeline. A run is marked `partial` if some collectors succeeded and others failed.

The `brand_profile_runs` table records every run with task-level counters (`tasks_total`,
`tasks_completed`, `tasks_failed`) so the dashboard can surface pipeline health per brand.

### API Endpoints

All endpoints are under `/api/v1/brands/` and require `Authorization: Bearer <api_key>`.
Path parameter `{slug}` is the brand's URL slug (not a UUID -- brands are looked up by slug for
human-readable URLs).

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/brands` | Brand list with completeness scores. Filters: `relationship`, `tier`, `is_active`, `q`. |
| `GET` | `/api/v1/brands/{slug}` | Full brand profile: brand row + profile metadata + social handles + completeness breakdown. |
| `GET` | `/api/v1/brands/{slug}/signals` | Cursor-paginated signal feed ordered by `published_at DESC`. Filters: `signal_type`, `limit`. |
| `GET` | `/api/v1/brands/{slug}/funding` | Funding events ordered by `announced_at DESC`. |
| `GET` | `/api/v1/brands/{slug}/lab-tests` | Lab test results ordered by `test_date DESC`. |
| `GET` | `/api/v1/brands/{slug}/legal` | Legal proceedings ordered by `filed_at DESC`. |
| `GET` | `/api/v1/brands/{slug}/sponsorships` | Sponsorship deals, active ones first. |
| `GET` | `/api/v1/brands/{slug}/distributors` | Distribution relationships ordered by `distributor_name`. |
| `GET` | `/api/v1/brands/{slug}/competitors` | Competitor relationships (both directions resolved). |
| `GET` | `/api/v1/brands/{slug}/media` | Media appearances ordered by `aired_at DESC`. |

All list endpoints support `limit` (default 50, max 200). `/signals` supports cursor-based
pagination via `cursor` query param and returns `next_cursor` in `meta` when more data exists.

Responses follow the standard envelope:

```json
{
  "data": [...],
  "meta": { "request_id": "req_abc", "timestamp": "2026-02-21T00:00:00Z" }
}
```

### Dashboard

The frontend brand intelligence dashboard consists of two pages:

**`/brands` -- BrandsPage**

Displays all tracked brands as a card grid. Each card shows:

- Brand name, slug, logo (with fallback placeholder)
- Tier badge (`T1`, `T2`, `T3`) and relationship tag (`portfolio` / `competitor`)
- Completeness progress bar (0-100%) sourced from `GET /api/v1/brands`

**`/brands/:slug` -- BrandProfilePage**

Full-detail brand view. Header shows logo, name, tier/relationship badges, tagline, HQ + founded
year metadata row, social platform links, and the completeness progress bar.

Three tabs below the header:

| Tab | Component | Data Sources |
|---|---|---|
| Feed | `BrandSignalFeed` | `GET /brands/{slug}/signals` -- chronological signal stream |
| Content | `BrandContentTab` | Funding events, media appearances, sponsorships, lab tests |
| Recon | `BrandReconTab` | Distributors, legal proceedings, competitor relationships, domains |

---

## Configuration

### Environment Variables

| Variable | Required | Description |
|---|---|---|
| `TEI_URL` | Yes | Text Embeddings Inference base URL (e.g., `http://localhost:8080`) |
| `YOUTUBE_API_KEY` | No | YouTube Data API v3 key. If absent, YouTube collection is skipped. |
| `BRAND_INTAKE_CRON` | No | Override default intake schedule. Default: `0 6 * * *` (06:00 UTC daily). |

`TEI_URL` is passed into `IntakeConfig` and used by `scbdb-profiler::embedder::embed_text`.
If `TEI_URL` is set to an empty string, embedding is skipped (signals are still upserted to
the database without a `qdrant_point_id`).

`YOUTUBE_API_KEY` is optional -- the collector skips YouTube if no key is present, which means
the profiler degrades gracefully to RSS-only collection without any configuration change.

---

## Completeness Scoring

The completeness score is a 0-100 integer computed by `scbdb-db::brand_completeness` for each
brand. It is used as a progress indicator on both the brands list and the brand profile page.

### Flags and Weights

Fourteen boolean presence checks, each mapped to a weight. Weights sum to exactly 100 (enforced
by a compile-time `const` assertion in `brand_completeness.rs`).

| Flag | Check | Weight |
|---|---|---|
| `has_profile` | `brand_profiles` row exists | 10 |
| `has_description` | `brand_profiles.description` is non-null | 15 |
| `has_tagline` | `brand_profiles.tagline` is non-null | 5 |
| `has_founded_year` | `brand_profiles.founded_year` is non-null | 5 |
| `has_location` | `brand_profiles.hq_city` AND `hq_state` both non-null | 5 |
| `has_social_handles` | At least one active `brand_social_handles` row | 10 |
| `has_domains` | At least one active `brand_domains` row | 5 |
| `has_signals` | At least one `brand_signals` row | 10 |
| `has_funding` | At least one `brand_funding_events` row | 5 |
| `has_lab_tests` | At least one `brand_lab_tests` row | 5 |
| `has_legal` | At least one `brand_legal_proceedings` row | 5 |
| `has_sponsorships` | At least one `brand_sponsorships` row | 5 |
| `has_distributors` | At least one `brand_distributors` row | 10 |
| `has_media` | At least one `brand_media_appearances` row | 5 |

### Computation

The score is computed in a single SQL round-trip using a CTE (`WITH presence AS (...)`) that
evaluates all 14 presence checks against the brand's data, then sums the weights for every
`true` flag. Two query variants exist:

- `get_brand_completeness(pool, brand_id)` -- returns the full `BrandCompletenessScore` struct
  with all 14 boolean flags plus the score. Used by the brand detail endpoint.
- `get_all_brands_completeness(pool)` -- returns a `HashMap<i64, i32>` of brand_id -> score
  for all active brands in one round-trip. Used by the brand list endpoint to avoid N+1 queries.

### Example

A brand with a `brand_profiles` row containing `description`, `tagline`, and `hq_city`/
`hq_state`, plus active social handles and at least one signal, would score:

```
has_profile       = true  --> +10
has_description   = true  --> +15
has_tagline       = true  --> +5
has_founded_year  = false --> +0
has_location      = true  --> +5
has_social_handles= true  --> +10
has_domains       = false --> +0
has_signals       = true  --> +10
has_funding       = false --> +0
has_lab_tests     = false --> +0
has_legal         = false --> +0
has_sponsorships  = false --> +0
has_distributors  = false --> +0
has_media         = false --> +0
                          ------
Total                        55
```
