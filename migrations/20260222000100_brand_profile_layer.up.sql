-- migrations/20260222000100_brand_profile_layer.up.sql

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
