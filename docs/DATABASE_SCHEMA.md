# Database Schema

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

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
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deleted_at TIMESTAMPTZ,
  UNIQUE (brand_id, source_platform, source_product_id)
);
CREATE INDEX idx_products_brand_id ON products (brand_id);
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
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (product_id, source_variant_id)
);
CREATE INDEX idx_product_variants_product_id ON product_variants (product_id);
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
  collection_run_id BIGINT,
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

### `sentiment_snapshots` (Post-MVP)

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

### Planned Post-MVP / Future Work

- `sentiment_snapshots`
- Any Spider/Qdrant/TEI-related schema extensions

## Notes

- `sentiment_snapshots` is planned but not required for MVP delivery.
- Spider/Qdrant/TEI integration will add additional schema later as needed.
