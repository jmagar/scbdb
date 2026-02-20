-- Add a unique index on bill_events(bill_id, description, event_date) with
-- NULLS NOT DISTINCT so that rows with a NULL event_date are treated as equal
-- (not distinct from each other). This makes ON CONFLICT DO NOTHING atomic and
-- eliminates the TOCTOU race in the previous WHERE NOT EXISTS implementation.
CREATE UNIQUE INDEX idx_bill_events_dedup
    ON bill_events (bill_id, description, event_date)
    NULLS NOT DISTINCT;
