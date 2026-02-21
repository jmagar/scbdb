# Database Schema

## Document Metadata

- Version: 1.3
- Status: Active
- Last Updated (EST): 00:00:00 | 02/20/2026 EST

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

These tables are added in Phase 6 to support the `scbdb-profiler` crate and brand profile dashboard. Full DDL lives in the `migrations/` directory.

### `brand_profiles`

Static profile data for each brand — extended metadata beyond what lives in the `brands` table.

Key columns: `brand_id` (FK → `brands`), `tagline`, `description`, `hq_city`, `hq_state`, `hq_country`, `founded_year`, `funding_stage`, `employee_count_estimate`, `primary_channel` (`dtc`, `retail`, `hybrid`), `metadata` (JSONB), timestamps.

### `brand_social_handles`

Platform → handle mapping per brand. Supports multi-platform social presence tracking.

Key columns: `brand_id` (FK → `brands`), `platform` (`twitter`, `instagram`, `youtube`, `tiktok`, `linkedin`, etc.), `handle`, `profile_url`, `is_active`, timestamps.

Unique constraint: `(brand_id, platform)`.

### `brand_domains`

Domain registrations per brand. A brand may own multiple domains (redirect domains, regional TLDs, sub-brands).

Key columns: `brand_id` (FK → `brands`), `domain`, `is_primary`, `is_active`, timestamps.

Unique constraint: `(brand_id, domain)`.

### `brand_signals`

Unified signal stream for all brand intelligence content types — social posts, news articles, press releases, YouTube videos, RSS items, etc. Backed by Qdrant for embedding dedup.

Key columns: `brand_id` (FK → `brands`), `signal_type` (`tweet`, `instagram_post`, `youtube_video`, `rss_item`, `press_release`, `news_article`, `newsletter`), `source_url`, `source_id` (platform-native ID), `title`, `body`, `published_at`, `author`, `metadata` (JSONB), `qdrant_id` (UUID — reference to Qdrant point), `embed_status` (`pending`, `embedded`, `skipped`, `failed`), timestamps.

Unique constraint: `(brand_id, signal_type, source_id)`.

### `brand_signal_embeds`

Qdrant vector reference table. Records the relationship between a `brand_signals` row and its Qdrant point ID.

Key columns: `signal_id` (FK → `brand_signals`), `qdrant_collection`, `qdrant_id` (UUID), `model`, `vector_dims`, `embedded_at`.

### `brand_funding_events`

Investment rounds, exits, and other financing events per brand.

Key columns: `brand_id` (FK → `brands`), `event_type` (`seed`, `series_a`, `series_b`, `growth`, `acquisition`, `ipo`, `other`), `amount_usd`, `announced_date`, `investors` (TEXT[]), `source_url`, `notes`.

### `brand_lab_tests`

CoA (Certificate of Analysis) and lab test results per product variant.

Key columns: `brand_id` (FK → `brands`), `variant_id` (FK → `product_variants`, nullable), `lab_name`, `test_date`, `thc_mg_per_serving`, `cbd_mg_per_serving`, `batch_id`, `coa_url`, `metadata` (JSONB), timestamps.

### `brand_legal_proceedings`

Lawsuits, regulatory actions, and enforcement events involving a brand.

Key columns: `brand_id` (FK → `brands`), `proceeding_type` (`lawsuit`, `fda_warning`, `state_enforcement`, `class_action`, `settlement`, `other`), `title`, `filed_date`, `resolved_date`, `jurisdiction`, `outcome`, `source_url`, `notes`.

### `brand_sponsorships`

Sports, events, and influencer deal tracking per brand.

Key columns: `brand_id` (FK → `brands`), `sponsorship_type` (`sports_team`, `event`, `athlete`, `influencer`, `venue`, `podcast`, `other`), `partner_name`, `deal_start`, `deal_end`, `territory`, `source_url`, `notes`.

### `brand_distributors`

Distribution relationships per brand per territory.

Key columns: `brand_id` (FK → `brands`), `distributor_name`, `territory_state` (CHAR(2)), `territory_region`, `channel` (`on_premise`, `off_premise`, `dtc`, `dispensary`), `status` (`active`, `inactive`, `rumored`), `source_url`, `notes`.

### `brand_competitor_relationships`

Direct competitor mapping — records directional or symmetric relationships between brands.

Key columns: `brand_id` (FK → `brands`), `competitor_brand_id` (FK → `brands`), `relationship_type` (`direct`, `adjacent`, `acqui_hire`, `distribution_overlap`), `notes`.

Unique constraint: `(brand_id, competitor_brand_id)`.

### `brand_newsletters`

Newsletter subscription tracking — records newsletter sources associated with each brand.

Key columns: `brand_id` (FK → `brands`), `list_name`, `subscribe_url`, `last_ingested_at`, `is_active`, timestamps.

### `brand_media_appearances`

Podcast, press, and video appearances by brand principals or featuring the brand.

Key columns: `brand_id` (FK → `brands`), `media_type` (`podcast`, `press_article`, `video`, `interview`, `review`, `other`), `outlet_name`, `title`, `url`, `published_at`, `author`, `notes`.

### `brand_profile_runs`

Profiler job run log — records each execution of the `scbdb-profiler` pipeline per brand.

Key columns: `brand_id` (FK → `brands`), `run_type` (e.g., `youtube`, `rss`, `newsroom`, `full`), `status` (`queued`, `running`, `succeeded`, `failed`), `signals_upserted`, `signals_embedded`, `started_at`, `completed_at`, `error_message`, `metadata` (JSONB).

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
- `brand_signal_embeds`
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

- Qdrant and TEI are integrated in Phase 4 for sentiment signal dedup and embedding storage. Future semantic search capabilities beyond dedup may add additional schema as needed.
- Spider integration for non-Shopify crawling will add schema later as needed.
- Phase 6 `brand_signals` rows reference Qdrant points via `qdrant_id`; the `brand_signal_embeds` table is the authoritative join between PostgreSQL rows and the Qdrant `brand_signals` collection.
