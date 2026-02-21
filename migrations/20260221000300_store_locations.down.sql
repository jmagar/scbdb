DROP TABLE IF EXISTS store_locations;

-- Remove rows with run_type = 'locations' before re-adding the constraint
-- without 'locations' in the allowed set, otherwise ADD CONSTRAINT will fail.
DELETE FROM collection_runs WHERE run_type = 'locations';

ALTER TABLE collection_runs DROP CONSTRAINT IF EXISTS collection_runs_run_type_check;
ALTER TABLE collection_runs ADD CONSTRAINT collection_runs_run_type_check
  CHECK (run_type IN ('products', 'pricing', 'regs', 'sentiment'));
