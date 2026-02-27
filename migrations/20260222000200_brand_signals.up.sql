-- migrations/20260222000200_brand_signals.up.sql

CREATE TYPE brand_signal_type AS ENUM (
  'article',
  'blog_post',
  'tweet',
  'youtube_video',
  'reddit_post',
  'newsletter',
  'press_release',
  'podcast_episode',
  'event',
  'award',
  'partnership',
  'launch'
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
