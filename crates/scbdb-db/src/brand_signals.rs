// crates/scbdb-db/src/brand_signals.rs
use crate::DbError;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrandSignalRow {
    pub id: i64,
    pub public_id: Uuid,
    pub brand_id: i64,
    pub signal_type: String,
    pub source_platform: Option<String>,
    pub source_url: Option<String>,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub view_count: Option<i32>,
    pub like_count: Option<i32>,
    pub comment_count: Option<i32>,
    pub share_count: Option<i32>,
    pub qdrant_point_id: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub collected_at: DateTime<Utc>,
}

pub struct NewBrandSignal<'a> {
    pub brand_id: i64,
    pub signal_type: &'a str,
    pub source_platform: Option<&'a str>,
    pub source_url: Option<&'a str>,
    pub external_id: Option<&'a str>,
    pub title: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub content: Option<&'a str>,
    pub image_url: Option<&'a str>,
    pub qdrant_point_id: Option<&'a str>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Upsert a brand signal. Returns the internal ID.
/// Dedup key: (`brand_id`, `signal_type`, `external_id`).
/// If `external_id` is None, the row is always inserted (no dedup).
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn upsert_brand_signal(
    pool: &PgPool,
    signal: &NewBrandSignal<'_>,
) -> Result<i64, DbError> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO brand_signals \
           (brand_id, signal_type, source_platform, source_url, external_id, \
            title, summary, content, image_url, qdrant_point_id, published_at) \
         VALUES ($1, $2::brand_signal_type, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (brand_id, signal_type, external_id) DO UPDATE SET \
           title = COALESCE(EXCLUDED.title, brand_signals.title), \
           summary = COALESCE(EXCLUDED.summary, brand_signals.summary), \
           qdrant_point_id = COALESCE(EXCLUDED.qdrant_point_id, brand_signals.qdrant_point_id), \
           updated_at = NOW() \
         RETURNING id",
    )
    .bind(signal.brand_id)
    .bind(signal.signal_type)
    .bind(signal.source_platform)
    .bind(signal.source_url)
    .bind(signal.external_id)
    .bind(signal.title)
    .bind(signal.summary)
    .bind(signal.content)
    .bind(signal.image_url)
    .bind(signal.qdrant_point_id)
    .bind(signal.published_at)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Returns IDs of active brands whose most recent signal is older than the given
/// `stale_hours` threshold, or that have no signals at all.
///
/// Used by the scheduler to determine which brands need signal collection.
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn list_brands_needing_signal_refresh(
    pool: &PgPool,
    stale_hours: i32,
) -> Result<Vec<i64>, DbError> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT b.id FROM brands b \
         LEFT JOIN LATERAL ( \
             SELECT MAX(bs.collected_at) AS latest \
             FROM brand_signals bs WHERE bs.brand_id = b.id \
         ) s ON TRUE \
         WHERE b.is_active = true AND b.deleted_at IS NULL \
           AND (s.latest IS NULL OR s.latest < NOW() - make_interval(hours => $1)) \
         ORDER BY b.id",
    )
    .bind(stale_hours)
    .fetch_all(pool)
    .await?)
}

/// Returns all active domain URLs for a given brand.
///
/// Used by the scheduler to construct feed URLs for the brand intake pipeline.
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn list_brand_feed_urls(pool: &PgPool, brand_id: i64) -> Result<Vec<String>, DbError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT domain FROM brand_domains \
         WHERE brand_id = $1 AND is_active = true \
         ORDER BY domain",
    )
    .bind(brand_id)
    .fetch_all(pool)
    .await?)
}

/// Returns IDs of active brands whose social handles have not been checked
/// within the given `stale_days` threshold.
///
/// Used by the scheduler to trigger periodic handle verification.
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn list_brands_with_stale_handles(
    pool: &PgPool,
    stale_days: i32,
) -> Result<Vec<i64>, DbError> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT bsh.brand_id FROM brand_social_handles bsh \
         JOIN brands b ON b.id = bsh.brand_id \
         WHERE b.is_active = true AND b.deleted_at IS NULL \
           AND bsh.is_active = true \
           AND (bsh.last_checked_at IS NULL \
                OR bsh.last_checked_at < NOW() - make_interval(days => $1)) \
         ORDER BY bsh.brand_id",
    )
    .bind(stale_days)
    .fetch_all(pool)
    .await?)
}

/// Cursor-paginated signal feed for a brand.
/// `cursor` is the `id` of the last seen row (exclusive, for next-page queries).
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn list_brand_signals(
    pool: &PgPool,
    brand_id: i64,
    signal_type_filter: Option<&str>,
    limit: i64,
    cursor: Option<i64>,
) -> Result<Vec<BrandSignalRow>, DbError> {
    let rows = sqlx::query_as::<_, BrandSignalRow>(
        "SELECT id, public_id, brand_id, signal_type::TEXT, source_platform, source_url, \
                external_id, title, summary, image_url, view_count, like_count, \
                comment_count, share_count, qdrant_point_id, published_at, collected_at \
         FROM brand_signals \
         WHERE brand_id = $1 \
           AND ($2::TEXT IS NULL OR signal_type = $2::brand_signal_type) \
           AND ($3::BIGINT IS NULL OR id < $3) \
         ORDER BY id DESC LIMIT $4",
    )
    .bind(brand_id)
    .bind(signal_type_filter)
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
