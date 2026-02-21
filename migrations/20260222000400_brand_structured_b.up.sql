-- migrations/20260222000400_brand_structured_b.up.sql

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
