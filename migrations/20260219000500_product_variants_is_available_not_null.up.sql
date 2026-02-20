-- Backfill any NULL is_available values before adding the NOT NULL constraint.
-- Treat missing availability data as available — this is the correct display
-- default for a competitive intelligence platform where scrape failures should
-- not hide a product. If a NULL was caused by a genuinely indeterminate scrape,
-- the next collection run will overwrite it with the live value.
UPDATE product_variants SET is_available = TRUE WHERE is_available IS NULL;

ALTER TABLE product_variants
    ALTER COLUMN is_available SET NOT NULL,
    ALTER COLUMN is_available SET DEFAULT TRUE;
