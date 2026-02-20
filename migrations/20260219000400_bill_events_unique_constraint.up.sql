-- Remove any duplicate bill_events rows that would block the unique index.
-- Keeps the row with the lowest id for each (bill_id, description, event_date)
-- tuple (treating NULLs as equal for deduplication purposes).
DELETE FROM bill_events
WHERE id NOT IN (
    SELECT MIN(id)
    FROM bill_events
    GROUP BY bill_id, description, COALESCE(event_date::text, '')
);

-- Add a unique index on bill_events(bill_id, description, event_date) with
-- NULLS NOT DISTINCT so that rows with a NULL event_date are treated as equal
-- (not distinct from each other). This makes ON CONFLICT DO NOTHING atomic and
-- eliminates the TOCTOU race in the previous WHERE NOT EXISTS implementation.
CREATE UNIQUE INDEX idx_bill_events_dedup
    ON bill_events (bill_id, description, event_date)
    NULLS NOT DISTINCT;
