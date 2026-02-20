ALTER TABLE product_variants
    ALTER COLUMN is_available DROP NOT NULL,
    ALTER COLUMN is_available DROP DEFAULT;
