-- migrations/20260222000500_brand_profile_runs.up.sql
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
