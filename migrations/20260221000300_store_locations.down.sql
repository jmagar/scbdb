DROP TABLE IF EXISTS store_locations;

ALTER TABLE collection_runs DROP CONSTRAINT IF EXISTS collection_runs_run_type_check;
ALTER TABLE collection_runs ADD CONSTRAINT collection_runs_run_type_check
  CHECK (run_type IN ('products', 'pricing', 'regs', 'sentiment'));
