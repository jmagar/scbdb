-- NOTE: This rollback restores the column's nullability and removes the DEFAULT,
-- but it does NOT recover original NULL values. Any row that was NULL before
-- the up migration ran was backfilled to TRUE and cannot be rolled back to NULL.
ALTER TABLE product_variants
    ALTER COLUMN is_available DROP NOT NULL,
    ALTER COLUMN is_available DROP DEFAULT;
