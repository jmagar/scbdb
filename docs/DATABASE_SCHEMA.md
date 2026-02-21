# Database Schema

## Document Metadata

- Version: 1.4
- Status: Active
- Last Updated (EST): 00:00:00 | 02/21/2026 EST

## Purpose

Authoritative relational schema for SCBDB MVP.

## Conventions

- Table names: `snake_case`, plural where meaningful.
- Primary key: `BIGINT GENERATED ALWAYS AS IDENTITY`.
- External IDs: `UUID` for API-safe public references.
- Timestamps: `TIMESTAMPTZ` only.
- Soft delete: `deleted_at TIMESTAMPTZ` where needed.

## Extensions

```sql
CREATE EXTENSION IF NOT EXISTS pgcrypto;
```

## Tables

### `brands`

```sql
CREATE TABLE brands (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  name TEXT NOT NULL,
  slug TEXT NOT NULL UNIQUE,
  relationship TEXT NOT NULL CHECK (relationship IN ('portfolio', 'competitor')),
  tier SMALLINT NOT NULL CHECK (tier IN (1, 2, 3)),
  domain TEXT,
  shop_url TEXT,
  notes TEXT,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deleted_at TIMESTAMPTZ
);
CREATE INDEX idx_brands_relationship_tier ON brands (relationship, tier);
```

### `products`

```sql
CREATE TABLE products (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  brand_id BIGINT NOT NULL REFERENCES brands(id),
  source_platform TEXT NOT NULL DEFAULT 'shopify',
  source_product_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT,
  product_type TEXT,
  tags TEXT[],
  created_at_source TIMESTAMPTZ,
  updated_at_source TIMESTAMPTZ,
  handle TEXT,
  metadata JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deleted_at TIMESTAMPTZ,
  UNIQUE (brand_id, source_platform, source_product_id)
);
CREATE INDEX idx_products_brand_id ON products (brand_id);
CREATE INDEX idx_products_handle ON products (handle) WHERE handle IS NOT NULL;
```

### `product_variants`

```sql
CREATE TABLE product_variants (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  product_id BIGINT NOT NULL REFERENCES products(id),
  source_variant_id TEXT NOT NULL,
  sku TEXT,
  title TEXT,
  dosage_mg NUMERIC(8,2),
  cbd_mg NUMERIC(8,2),
  size_value NUMERIC(10,2),
  size_unit TEXT,
  is_default BOOLEAN NOT NULL DEFAULT FALSE,
  is_available BOOLEAN,
  extraction_status TEXT NOT NULL DEFAULT 'pending'
    CHECK (extraction_status IN ('pending', 'extracted', 'skipped', 'failed')),
  extracted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (product_id, source_variant_id)
);
CREATE INDEX idx_product_variants_product_id ON product_variants (product_id);
CREATE INDEX idx_product_variants_pending_extraction ON product_variants (id) WHERE extraction_status = 'pending';
```

### `price_snapshots`

```sql
CREATE TABLE price_snapshots (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  variant_id BIGINT NOT NULL REFERENCES product_variants(id),
  captured_at TIMESTAMPTZ NOT NULL,
  currency_code CHAR(3) NOT NULL DEFAULT 'USD',
  price NUMERIC(10,2) NOT NULL,
  compare_at_price NUMERIC(10,2),
  source_url TEXT,
  collection_run_id BIGINT REFERENCES collection_runs(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_price_snapshots_variant_captured ON price_snapshots (variant_id, captured_at DESC);
```

### `collection_runs`

```sql
CREATE TABLE collection_runs (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  run_type TEXT NOT NULL CHECK (run_type IN ('products', 'pricing', 'regs', 'sentiment')),
  trigger_source TEXT NOT NULL CHECK (trigger_source IN ('cli', 'api', 'scheduler')),
  status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  records_processed INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_collection_runs_type_status_created ON collection_runs (run_type, status, created_at DESC);
```

### `collection_run_brands`

```sql
CREATE TABLE collection_run_brands (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  collection_run_id BIGINT NOT NULL REFERENCES collection_runs(id) ON DELETE CASCADE,
  brand_id BIGINT NOT NULL REFERENCES brands(id),
  status TEXT NOT NULL CHECK (status IN ('skipped', 'succeeded', 'failed')),
  records_processed INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (collection_run_id, brand_id)
);
```

### `bills`

```sql
CREATE TABLE bills (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  jurisdiction TEXT NOT NULL,
  session TEXT,
  bill_number TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT,
  status TEXT NOT NULL,
  status_date DATE,
  introduced_date DATE,
  last_action_date DATE,
  source_url TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deleted_at TIMESTAMPTZ,
  UNIQUE (jurisdiction, bill_number)
);
CREATE INDEX idx_bills_jurisdiction_status ON bills (jurisdiction, status);
```

### `bill_events`

```sql
CREATE TABLE bill_events (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  bill_id BIGINT NOT NULL REFERENCES bills(id) ON DELETE CASCADE,
  event_date DATE,
  event_type TEXT,
  chamber TEXT,
  description TEXT NOT NULL,
  source_url TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_bill_events_bill_id_event_date ON bill_events (bill_id, event_date DESC);
```

### `sentiment_snapshots`

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

## Views

### `view_products_dashboard`

Read-model view aggregating product details, brand context, and latest pricing for the dashboard.

```sql
CREATE VIEW view_products_dashboard AS SELECT ...
```

### `view_pricing_summary`

Read-model view aggregating pricing metrics (min/max/avg) per brand.

```sql
CREATE VIEW view_pricing_summary AS SELECT ...
```

### `view_sentiment_summary`

Read-model view providing the most recent sentiment snapshot per brand.

```sql
CREATE VIEW view_sentiment_summary AS SELECT ...
```

## Phase 6: Brand Intelligence Layer

These tables are added in Phase 6 to support the `scbdb-profiler` crate and the brand profile dashboard. Migrations live in `migrations/20260222000100` through `migrations/20260222000500`.

### `brand_profiles`

Extended static profile metadata per brand -- one row per brand maximum.

Notable constraint: `UNIQUE (brand_id)` -- enforced as a column-level unique constraint.

```sql
CREATE TABLE brand_profiles (
  id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id              BIGINT NOT NULL REFERENCES brands(id) UNIQUE,
  tagline               TEXT,
  description           TEXT,
  founded_year          SMALLINT,
  hq_city               TEXT,
  hq_state              TEXT,
  hq_country            TEXT NOT NULL DEFAULT 'US',
  parent_company        TEXT,
  parent_domain         TEXT,
  ceo_name              TEXT,
  employee_count_approx INTEGER,
  total_funding_usd     BIGINT,
  latest_valuation_usd  BIGINT,
  funding_stage         TEXT,
  stock_ticker          TEXT,
  stock_exchange        TEXT,
  hero_image_url        TEXT,
  profile_completed_at  TIMESTAMPTZ,
  last_enriched_at      TIMESTAMPTZ,
  created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### `brand_social_handles`

Platform-to-handle mapping for each brand. Supports multi-platform social presence tracking
with follower counts and verified status.

Notable constraint: `UNIQUE (brand_id, platform, handle)` -- a brand may have multiple handles
per platform, but the same handle cannot be registered twice for the same brand and platform.

```sql
CREATE TABLE brand_social_handles (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id        BIGINT NOT NULL REFERENCES brands(id),
  platform        TEXT NOT NULL,
  handle          TEXT NOT NULL,
  profile_url     TEXT,
  follower_count  INTEGER,
  is_verified     BOOLEAN,
  is_active       BOOLEAN NOT NULL DEFAULT TRUE,
  last_checked_at TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (brand_id, platform, handle)
);
CREATE INDEX idx_brand_social_handles_platform ON brand_social_handles (platform);
```

### `brand_domains`

Domain registrations per brand. A brand may own multiple domains -- redirect domains,
regional TLDs, sub-brands.

Notable constraint: `UNIQUE (brand_id, domain)`.

```sql
CREATE TABLE brand_domains (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id      BIGINT NOT NULL REFERENCES brands(id),
  domain        TEXT NOT NULL,
  domain_type   TEXT NOT NULL,
  is_active     BOOLEAN NOT NULL DEFAULT TRUE,
  registrar     TEXT,
  registered_at DATE,
  expires_at    DATE,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (brand_id, domain)
);
CREATE INDEX idx_brand_domains_domain ON brand_domains (domain);
```

### `brand_signals`

Unified signal stream for all brand intelligence content types. The `qdrant_point_id` column
stores the deterministic Qdrant point ID computed during embedding. Signals are deduplicated
on `(brand_id, signal_type, external_id)`.

Signal types (`brand_signal_type` enum): `article`, `blog_post`, `tweet`, `youtube_video`,
`reddit_post`, `newsletter`, `press_release`, `podcast_episode`, `event`, `award`,
`partnership`, `launch`.

Notable constraint: `UNIQUE (brand_id, signal_type, external_id)`.

```sql
CREATE TYPE brand_signal_type AS ENUM (
  'article', 'blog_post', 'tweet', 'youtube_video', 'reddit_post',
  'newsletter', 'press_release', 'podcast_episode', 'event', 'award',
  'partnership', 'launch'
);

CREATE TABLE brand_signals (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id       UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  brand_id        BIGINT NOT NULL REFERENCES brands(id),
  signal_type     brand_signal_type NOT NULL,
  source_platform TEXT,
  source_url      TEXT,
  external_id     TEXT,
  title           TEXT,
  summary         TEXT,
  content         TEXT,
  image_url       TEXT,
  view_count      INTEGER,
  like_count      INTEGER,
  comment_count   INTEGER,
  share_count     INTEGER,
  qdrant_point_id TEXT,
  published_at    TIMESTAMPTZ,
  collected_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (brand_id, signal_type, external_id)
);
CREATE INDEX idx_brand_signals_brand_published ON brand_signals (brand_id, published_at DESC);
CREATE INDEX idx_brand_signals_type            ON brand_signals (signal_type);
CREATE INDEX idx_brand_signals_source_platform ON brand_signals (source_platform);
CREATE INDEX idx_brand_signals_collected       ON brand_signals (collected_at DESC);
```

### `brand_funding_events`

Investment rounds, acquisitions, and other financing events per brand. `investors` is a TEXT
array for multiple investor names per round. `acquirer` captures the acquiring entity name for
acquisition events.

```sql
CREATE TABLE brand_funding_events (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id     BIGINT NOT NULL REFERENCES brands(id),
  event_type   TEXT NOT NULL,
  amount_usd   BIGINT,
  announced_at DATE,
  investors    TEXT[],
  acquirer     TEXT,
  source_url   TEXT,
  notes        TEXT,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_funding_events_brand_id  ON brand_funding_events (brand_id);
CREATE INDEX idx_brand_funding_events_announced ON brand_funding_events (announced_at DESC);
```

### `brand_lab_tests`

CoA (Certificate of Analysis) and lab test results. Can be linked to a specific product and/or
variant; both FKs are nullable to allow brand-level test records not yet matched to a SKU.

```sql
CREATE TABLE brand_lab_tests (
  id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id              BIGINT NOT NULL REFERENCES brands(id),
  product_id            BIGINT REFERENCES products(id),
  variant_id            BIGINT REFERENCES product_variants(id),
  lab_name              TEXT,
  test_date             DATE,
  report_url            TEXT,
  thc_mg_actual         NUMERIC(8,3),
  cbd_mg_actual         NUMERIC(8,3),
  total_cannabinoids_mg NUMERIC(8,3),
  passed                BOOLEAN,
  raw_data              JSONB,
  created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_lab_tests_brand_id   ON brand_lab_tests (brand_id);
CREATE INDEX idx_brand_lab_tests_variant_id ON brand_lab_tests (variant_id);
```

### `brand_legal_proceedings`

Lawsuits, regulatory enforcement actions, and FDA warning letters involving a brand.

```sql
CREATE TABLE brand_legal_proceedings (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id        BIGINT NOT NULL REFERENCES brands(id),
  proceeding_type TEXT NOT NULL,
  jurisdiction    TEXT,
  case_number     TEXT,
  title           TEXT NOT NULL,
  summary         TEXT,
  status          TEXT NOT NULL,
  filed_at        DATE,
  resolved_at     DATE,
  source_url      TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_legal_proceedings_brand_id ON brand_legal_proceedings (brand_id);
CREATE INDEX idx_brand_legal_proceedings_status   ON brand_legal_proceedings (status);
```

### `brand_sponsorships`

Sports team, event, athlete, and influencer deal tracking per brand. `entity_type` and
`deal_type` are free-text fields (e.g., `sports_team`, `title_sponsor`).

```sql
CREATE TABLE brand_sponsorships (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id     BIGINT NOT NULL REFERENCES brands(id),
  entity_name  TEXT NOT NULL,
  entity_type  TEXT NOT NULL,
  deal_type    TEXT NOT NULL,
  announced_at DATE,
  ends_at      DATE,
  source_url   TEXT,
  notes        TEXT,
  is_active    BOOLEAN NOT NULL DEFAULT TRUE,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_sponsorships_brand_id ON brand_sponsorships (brand_id);
CREATE INDEX idx_brand_sponsorships_active   ON brand_sponsorships (brand_id, is_active);
```

### `brand_distributors`

Distribution relationships per brand. `states` is a TEXT array (two-letter state codes) indexed
with GIN for efficient containment queries. `territory_type` and `channel_type` are free-text
fields (e.g., `national`, `regional`, `on_premise`, `off_premise`).

```sql
CREATE TABLE brand_distributors (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id          BIGINT NOT NULL REFERENCES brands(id),
  distributor_name  TEXT NOT NULL,
  distributor_slug  TEXT NOT NULL,
  states            TEXT[],
  territory_type    TEXT NOT NULL,
  channel_type      TEXT NOT NULL,
  started_at        DATE,
  ended_at          DATE,
  is_active         BOOLEAN NOT NULL DEFAULT TRUE,
  notes             TEXT,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_distributors_brand_id         ON brand_distributors (brand_id);
CREATE INDEX idx_brand_distributors_distributor_slug ON brand_distributors (distributor_slug);
CREATE INDEX idx_brand_distributors_states           ON brand_distributors USING GIN (states);
```

### `brand_competitor_relationships`

Directional or symmetric competitor mapping between brands. The
`CHECK (brand_id < competitor_brand_id)` constraint enforces canonical ordering so the pair
`(A, B)` and `(B, A)` cannot both exist -- all relationships are stored with the lower internal
ID first.

Notable constraints: `CHECK (brand_id < competitor_brand_id)`,
`UNIQUE (brand_id, competitor_brand_id, relationship_type, distributor_name)`.

```sql
CREATE TABLE brand_competitor_relationships (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id            BIGINT NOT NULL REFERENCES brands(id),
  competitor_brand_id BIGINT NOT NULL REFERENCES brands(id),
  relationship_type   TEXT NOT NULL,
  distributor_name    TEXT,
  states              TEXT[],
  notes               TEXT,
  first_observed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  is_active           BOOLEAN NOT NULL DEFAULT TRUE,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT chk_brand_ordering CHECK (brand_id < competitor_brand_id),
  UNIQUE (brand_id, competitor_brand_id, relationship_type, distributor_name)
);
CREATE INDEX idx_brand_competitor_rel_brand_id      ON brand_competitor_relationships (brand_id);
CREATE INDEX idx_brand_competitor_rel_competitor_id ON brand_competitor_relationships (competitor_brand_id);
```

### `brand_newsletters`

Newsletter subscription tracking. `inbox_address` is the monitoring inbox used to receive
issues. The `UNIQUE (brand_id, inbox_address)` constraint prevents duplicate subscriptions to
the same inbox for the same brand.

```sql
CREATE TABLE brand_newsletters (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id         BIGINT NOT NULL REFERENCES brands(id),
  list_name        TEXT NOT NULL,
  subscribe_url    TEXT,
  unsubscribe_url  TEXT,
  inbox_address    TEXT,
  subscribed_at    TIMESTAMPTZ,
  last_received_at TIMESTAMPTZ,
  is_active        BOOLEAN NOT NULL DEFAULT TRUE,
  notes            TEXT,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (brand_id, inbox_address)
);
CREATE INDEX idx_brand_newsletters_brand_id ON brand_newsletters (brand_id);
```

### `brand_media_appearances`

Podcast episodes, press articles, video interviews, and reviews featuring the brand.
`brand_signal_id` is an optional FK to `brand_signals` -- used when the appearance was first
ingested as a raw signal before being promoted to a structured record.

```sql
CREATE TABLE brand_media_appearances (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id         BIGINT NOT NULL REFERENCES brands(id),
  brand_signal_id  BIGINT REFERENCES brand_signals(id),
  appearance_type  TEXT NOT NULL,
  outlet_name      TEXT NOT NULL,
  title            TEXT,
  host_or_author   TEXT,
  aired_at         DATE,
  duration_seconds INTEGER,
  source_url       TEXT,
  notes            TEXT,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_media_appearances_brand_id ON brand_media_appearances (brand_id);
CREATE INDEX idx_brand_media_appearances_aired_at ON brand_media_appearances (aired_at DESC);
```

### `brand_profile_runs`

Profiler job run log -- records each execution of the `scbdb-profiler` pipeline per brand.
`tasks_total`, `tasks_completed`, and `tasks_failed` track sub-task progress within a run
(one task per collector source). `trigger_source` is constrained to `scheduler` or `api`.
The `partial` status applies when some tasks completed and others failed.

Notable constraint: `CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'partial'))`.

```sql
CREATE TABLE brand_profile_runs (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  brand_id         BIGINT NOT NULL REFERENCES brands(id),
  status           TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'partial')),
  trigger_source   TEXT NOT NULL CHECK (trigger_source IN ('scheduler', 'api')),
  tasks_total      INTEGER NOT NULL DEFAULT 0,
  tasks_completed  INTEGER NOT NULL DEFAULT 0,
  tasks_failed     INTEGER NOT NULL DEFAULT 0,
  started_at       TIMESTAMPTZ,
  completed_at     TIMESTAMPTZ,
  error_message    TEXT,
  metadata         JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_brand_profile_runs_brand_id ON brand_profile_runs (brand_id);
CREATE INDEX idx_brand_profile_runs_status   ON brand_profile_runs (brand_id, status, created_at DESC);
```

---

## Scope Status

### Defined for MVP Scope

- `brands`
- `products`
- `product_variants`
- `price_snapshots`
- `collection_runs`
- `collection_run_brands`
- `bills`
- `bill_events`
- `sentiment_snapshots`

### Defined for Phase 6 (Brand Intelligence Layer)

- `brand_profiles`
- `brand_social_handles`
- `brand_domains`
- `brand_signals`
- `brand_funding_events`
- `brand_lab_tests`
- `brand_legal_proceedings`
- `brand_sponsorships`
- `brand_distributors`
- `brand_competitor_relationships`
- `brand_newsletters`
- `brand_media_appearances`
- `brand_profile_runs`

### Planned Post-MVP / Future Work

- Any Spider-related schema extensions
- Qdrant/TEI schema extensions beyond the current vector dedup in Phase 4

## Notes

- Qdrant and TEI are integrated in Phase 4 for sentiment signal dedup and embedding storage.
  Future semantic search capabilities beyond dedup may add additional schema as needed.
- Spider integration for non-Shopify crawling will add schema later as needed.
- Phase 6 `brand_signals` rows store a `qdrant_point_id` TEXT column -- a deterministic UUID
  string computed from the signal's `external_id`, `source_url`, or `title` during embedding.
  This is the pointer from PostgreSQL into the Qdrant `brand_signals` collection.
