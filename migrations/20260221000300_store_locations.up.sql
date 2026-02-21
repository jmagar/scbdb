CREATE TABLE store_locations (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id        UUID NOT NULL DEFAULT gen_random_uuid(),
  brand_id         BIGINT NOT NULL REFERENCES brands(id),

  -- Stable dedup key: SHA-256(brand_id || name.lower() || city.lower() || state.upper() || zip)
  -- Computed in Rust before insert so upsert is safe across runs
  location_key     TEXT NOT NULL,

  name             TEXT NOT NULL,
  address_line1    TEXT,
  city             TEXT,
  state            TEXT,        -- 2-letter US state code or province
  zip              TEXT,
  country          TEXT NOT NULL DEFAULT 'US',
  latitude         NUMERIC(9,6),
  longitude        NUMERIC(9,6),
  phone            TEXT,

  external_id      TEXT,        -- Locally.com or Storemapper ID
  locator_source   TEXT,        -- 'locally', 'storemapper', 'jsonld', 'json_embed'
  raw_data         JSONB,       -- Full object from source (for future enrichment)

  first_seen_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  is_active        BOOLEAN NOT NULL DEFAULT TRUE,

  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CONSTRAINT uq_store_location_key UNIQUE (brand_id, location_key)
);

-- Note: No standalone brand_id index needed — the UNIQUE (brand_id, location_key) constraint
-- already creates a composite index with brand_id as leading column.
CREATE INDEX idx_store_locations_state      ON store_locations (state);
CREATE INDEX idx_store_locations_first_seen ON store_locations (first_seen_at DESC);
CREATE INDEX idx_store_locations_active     ON store_locations (brand_id, is_active);

-- Extend collection_runs.run_type check constraint
ALTER TABLE collection_runs DROP CONSTRAINT IF EXISTS collection_runs_run_type_check;
ALTER TABLE collection_runs ADD CONSTRAINT collection_runs_run_type_check
  CHECK (run_type IN ('products', 'pricing', 'regs', 'sentiment', 'locations'));
