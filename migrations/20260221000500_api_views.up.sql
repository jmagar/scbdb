-- Enforce read-model complexity in database views to simplify Rust query code.

CREATE VIEW view_products_dashboard AS
SELECT
    p.id AS product_id,
    p.name AS product_name,
    p.status AS product_status,
    p.vendor,
    p.source_url,
    p.metadata->>'primary_image_url' AS primary_image_url,
    b.name AS brand_name,
    b.slug AS brand_slug,
    b.logo_url AS brand_logo_url,
    b.relationship,
    b.tier,
    COUNT(v.id)::bigint AS variant_count,
    latest.price AS latest_price,
    latest.captured_at AS latest_price_captured_at,
    p.updated_at,
    p.deleted_at,
    b.deleted_at AS brand_deleted_at
FROM products p
JOIN brands b ON b.id = p.brand_id
LEFT JOIN product_variants v ON v.product_id = p.id
LEFT JOIN LATERAL (
    SELECT ps.price, ps.captured_at
    FROM product_variants pv
    JOIN price_snapshots ps ON ps.variant_id = pv.id
    WHERE pv.product_id = p.id
    ORDER BY ps.captured_at DESC, ps.id DESC
    LIMIT 1
) latest ON TRUE
GROUP BY p.id, p.name, p.status, p.vendor, p.source_url, p.metadata, b.name, b.slug,
         b.logo_url, b.relationship, b.tier, latest.price, latest.captured_at,
         p.updated_at, p.deleted_at, b.deleted_at;

CREATE VIEW view_pricing_summary AS
WITH latest_variant_prices AS (
    SELECT DISTINCT ON (ps.variant_id)
        pv.product_id,
        ps.variant_id,
        ps.price,
        ps.captured_at
    FROM price_snapshots ps
    JOIN product_variants pv ON pv.id = ps.variant_id
    ORDER BY ps.variant_id, ps.captured_at DESC, ps.id DESC
)
SELECT
    b.name AS brand_name,
    b.slug AS brand_slug,
    b.logo_url AS brand_logo_url,
    COUNT(lvp.variant_id)::bigint AS variant_count,
    AVG(lvp.price)::numeric(10,2) AS avg_price,
    MIN(lvp.price) AS min_price,
    MAX(lvp.price) AS max_price,
    MAX(lvp.captured_at) AS latest_capture_at,
    b.deleted_at AS brand_deleted_at,
    p.deleted_at AS product_deleted_at
FROM latest_variant_prices lvp
JOIN products p ON p.id = lvp.product_id
JOIN brands b ON b.id = p.brand_id
GROUP BY b.name, b.slug, b.logo_url, b.deleted_at, p.deleted_at;

CREATE VIEW view_sentiment_summary AS
SELECT
    b.name  AS brand_name,
    b.slug  AS brand_slug,
    ss.score,
    ss.signal_count,
    ss.captured_at,
    b.deleted_at AS brand_deleted_at,
    b.is_active
FROM (
    SELECT DISTINCT ON (brand_id)
        brand_id, score, signal_count, captured_at
    FROM sentiment_snapshots
    ORDER BY brand_id, captured_at DESC, id DESC
) ss
JOIN brands b ON b.id = ss.brand_id;
