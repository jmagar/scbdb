-- migrations/20260222000300_brand_structured_a.up.sql

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
