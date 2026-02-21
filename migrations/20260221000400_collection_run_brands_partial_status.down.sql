ALTER TABLE collection_run_brands
    DROP CONSTRAINT IF EXISTS collection_run_brands_status_check;

ALTER TABLE collection_run_brands
    ADD CONSTRAINT collection_run_brands_status_check
    CHECK (status IN ('skipped', 'succeeded', 'failed'));
