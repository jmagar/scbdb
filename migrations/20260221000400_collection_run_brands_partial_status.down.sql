-- Remap 'partial' status rows to 'succeeded' before removing 'partial' from the constraint,
-- otherwise ADD CONSTRAINT will fail on existing data.
UPDATE collection_run_brands SET status = 'succeeded' WHERE status = 'partial';

ALTER TABLE collection_run_brands
    DROP CONSTRAINT IF EXISTS collection_run_brands_status_check;

ALTER TABLE collection_run_brands
    ADD CONSTRAINT collection_run_brands_status_check
    CHECK (status IN ('skipped', 'succeeded', 'failed'));
