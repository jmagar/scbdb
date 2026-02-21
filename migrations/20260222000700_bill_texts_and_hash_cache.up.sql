-- Add LegiScan tracking columns to bills for change-hash caching.
-- legiscan_bill_id: stable numeric ID for cross-referencing getMasterList.
-- legiscan_change_hash: opaque hash from getMasterList; skip getBill when unchanged.
ALTER TABLE bills
    ADD COLUMN legiscan_bill_id   BIGINT UNIQUE,
    ADD COLUMN legiscan_change_hash TEXT;

CREATE INDEX idx_bills_legiscan_bill_id ON bills (legiscan_bill_id)
    WHERE legiscan_bill_id IS NOT NULL;

-- Stores versioned text links for each bill (Introduced, Engrossed, etc.).
-- legiscan_text_id is stable; ON CONFLICT DO NOTHING is the cache strategy.
CREATE TABLE bill_texts (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    bill_id           BIGINT NOT NULL REFERENCES bills(id) ON DELETE CASCADE,
    legiscan_text_id  BIGINT NOT NULL UNIQUE,
    text_date         DATE,
    text_type         TEXT NOT NULL,
    mime              TEXT NOT NULL,
    legiscan_url      TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bill_texts_bill_id ON bill_texts (bill_id);
