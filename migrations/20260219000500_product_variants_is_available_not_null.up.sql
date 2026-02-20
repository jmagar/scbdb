-- Backfill any NULL is_available values before adding the NOT NULL constraint.
-- Treat missing availability data as available (safe default for display).
UPDATE product_variants SET is_available = TRUE WHERE is_available IS NULL;

ALTER TABLE product_variants
    ALTER COLUMN is_available SET NOT NULL,
    ALTER COLUMN is_available SET DEFAULT TRUE;
