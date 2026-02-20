CREATE TABLE sentiment_snapshots (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    brand_id BIGINT NOT NULL REFERENCES brands(id),
    captured_at TIMESTAMPTZ NOT NULL,
    score NUMERIC(6,3) NOT NULL,
    signal_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sentiment_snapshots_brand_captured
    ON sentiment_snapshots (brand_id, captured_at DESC);
