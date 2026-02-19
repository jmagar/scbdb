DROP INDEX IF EXISTS idx_product_variants_pending_extraction;
DROP INDEX IF EXISTS idx_products_handle;

ALTER TABLE product_variants
    DROP COLUMN IF EXISTS extracted_at,
    DROP COLUMN IF EXISTS extraction_status;

ALTER TABLE products
    DROP COLUMN IF EXISTS metadata,
    DROP COLUMN IF EXISTS handle;
