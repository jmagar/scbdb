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

### Planned Post-MVP / Future Work

- Any Spider-related schema extensions
- Qdrant/TEI schema extensions beyond the current vector dedup in Phase 4

## Notes

- Qdrant and TEI are integrated in Phase 4 for sentiment signal dedup and embedding storage. Future semantic search capabilities beyond dedup may add additional schema as needed.
- Spider integration for non-Shopify crawling will add schema later as needed.
