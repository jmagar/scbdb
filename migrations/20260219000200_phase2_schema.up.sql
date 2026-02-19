-- Phase 2: Add fields needed for Shopify product collection
--
-- Products: add handle (Shopify URL slug) and metadata escape hatch
ALTER TABLE products
    ADD COLUMN IF NOT EXISTS handle TEXT,
    ADD COLUMN IF NOT EXISTS metadata JSONB;

-- Index on handle for URL-based lookups and dedup cross-check
CREATE INDEX IF NOT EXISTS idx_products_handle
    ON products (handle)
    WHERE handle IS NOT NULL;

-- Product variants: add extraction tracking for Phase 4 LLM pipeline
-- extraction_status tracks whether dosage/size values have been extracted
ALTER TABLE product_variants
    ADD COLUMN IF NOT EXISTS extraction_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (extraction_status IN ('pending', 'extracted', 'skipped', 'failed')),
    ADD COLUMN IF NOT EXISTS extracted_at TIMESTAMPTZ;

-- Partial index to efficiently query variants that still need extraction
CREATE INDEX IF NOT EXISTS idx_product_variants_pending_extraction
    ON product_variants (id)
    WHERE extraction_status = 'pending';
