//! Weighted completeness score for a brand's intelligence profile.
//!
//! Scores range from 0 to 100 and reflect how much data has been collected
//! for a brand across all intelligence tables.

use sqlx::PgPool;

use crate::DbError;

// ---------------------------------------------------------------------------
// Weight constants (must sum to exactly 100)
// ---------------------------------------------------------------------------

/// Has a `brand_profiles` row at all.
pub const W_PROFILE: i32 = 10;
/// `description` field is non-null.
pub const W_DESCRIPTION: i32 = 15;
/// `tagline` field is non-null.
pub const W_TAGLINE: i32 = 5;
/// `founded_year` field is non-null.
pub const W_FOUNDED_YEAR: i32 = 5;
/// `hq_city` AND `hq_state` both non-null.
pub const W_LOCATION: i32 = 5;
/// At least one `brand_social_handles` row exists.
pub const W_SOCIAL: i32 = 10;
/// At least one `brand_domains` row exists.
pub const W_DOMAINS: i32 = 5;
/// At least one `brand_signals` row exists.
pub const W_SIGNALS: i32 = 10;
/// At least one `brand_funding_events` row exists.
pub const W_FUNDING: i32 = 5;
/// At least one `brand_lab_tests` row exists.
pub const W_LAB_TESTS: i32 = 5;
/// At least one `brand_legal_proceedings` row exists.
pub const W_LEGAL: i32 = 5;
/// At least one `brand_sponsorships` row exists.
pub const W_SPONSORSHIPS: i32 = 5;
/// At least one `brand_distributors` row exists.
pub const W_DISTRIBUTORS: i32 = 10;
/// At least one `brand_media_appearances` row exists.
pub const W_MEDIA: i32 = 5;

// Compile-time assertion that weights sum to 100.
const _: () = assert!(
    W_PROFILE
        + W_DESCRIPTION
        + W_TAGLINE
        + W_FOUNDED_YEAR
        + W_LOCATION
        + W_SOCIAL
        + W_DOMAINS
        + W_SIGNALS
        + W_FUNDING
        + W_LAB_TESTS
        + W_LEGAL
        + W_SPONSORSHIPS
        + W_DISTRIBUTORS
        + W_MEDIA
        == 100,
    "completeness weights must sum to exactly 100"
);

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

/// Weighted completeness score for a single brand.
///
/// Each `has_*` flag maps to a specific intelligence dimension. The high
/// boolean count is inherent to the domain -- each flag directly corresponds
/// to a column in the SQL result and cannot be meaningfully collapsed.
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(clippy::struct_excessive_bools)]
pub struct BrandCompletenessScore {
    pub brand_id: i64,
    /// 0-100 weighted score.
    pub score: i32,
    pub has_profile: bool,
    pub has_description: bool,
    pub has_tagline: bool,
    pub has_founded_year: bool,
    /// `hq_city` AND `hq_state` both present.
    pub has_location: bool,
    pub has_social_handles: bool,
    pub has_domains: bool,
    pub has_signals: bool,
    pub has_funding: bool,
    pub has_lab_tests: bool,
    pub has_legal: bool,
    pub has_sponsorships: bool,
    pub has_distributors: bool,
    pub has_media: bool,
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// Compute the weighted profile completeness score for a brand.
///
/// Returns `None` if the brand does not exist in the `brands` table.
///
/// # Errors
///
/// Returns [`DbError`] on database query failure.
pub async fn get_brand_completeness(
    pool: &PgPool,
    brand_id: i64,
) -> Result<Option<BrandCompletenessScore>, DbError> {
    let row = sqlx::query_as::<_, BrandCompletenessScore>(
        "WITH presence AS ( \
            SELECT \
                b.id AS brand_id, \
                (bp.id IS NOT NULL) AS has_profile, \
                (bp.id IS NOT NULL AND bp.description IS NOT NULL) AS has_description, \
                (bp.id IS NOT NULL AND bp.tagline IS NOT NULL) AS has_tagline, \
                (bp.id IS NOT NULL AND bp.founded_year IS NOT NULL) AS has_founded_year, \
                (bp.id IS NOT NULL AND bp.hq_city IS NOT NULL AND bp.hq_state IS NOT NULL) AS has_location, \
                EXISTS(SELECT 1 FROM brand_social_handles bsh WHERE bsh.brand_id = b.id) AS has_social_handles, \
                EXISTS(SELECT 1 FROM brand_domains bd WHERE bd.brand_id = b.id) AS has_domains, \
                EXISTS(SELECT 1 FROM brand_signals bs WHERE bs.brand_id = b.id) AS has_signals, \
                EXISTS(SELECT 1 FROM brand_funding_events bfe WHERE bfe.brand_id = b.id) AS has_funding, \
                EXISTS(SELECT 1 FROM brand_lab_tests blt WHERE blt.brand_id = b.id) AS has_lab_tests, \
                EXISTS(SELECT 1 FROM brand_legal_proceedings blp WHERE blp.brand_id = b.id) AS has_legal, \
                EXISTS(SELECT 1 FROM brand_sponsorships bsp WHERE bsp.brand_id = b.id) AS has_sponsorships, \
                EXISTS(SELECT 1 FROM brand_distributors bdist WHERE bdist.brand_id = b.id) AS has_distributors, \
                EXISTS(SELECT 1 FROM brand_media_appearances bma WHERE bma.brand_id = b.id) AS has_media \
            FROM brands b \
            LEFT JOIN brand_profiles bp ON bp.brand_id = b.id \
            WHERE b.id = $1 \
        ) \
        SELECT \
            brand_id, \
            ( \
                CASE WHEN has_profile THEN $2 ELSE 0 END + \
                CASE WHEN has_description THEN $3 ELSE 0 END + \
                CASE WHEN has_tagline THEN $4 ELSE 0 END + \
                CASE WHEN has_founded_year THEN $5 ELSE 0 END + \
                CASE WHEN has_location THEN $6 ELSE 0 END + \
                CASE WHEN has_social_handles THEN $7 ELSE 0 END + \
                CASE WHEN has_domains THEN $8 ELSE 0 END + \
                CASE WHEN has_signals THEN $9 ELSE 0 END + \
                CASE WHEN has_funding THEN $10 ELSE 0 END + \
                CASE WHEN has_lab_tests THEN $11 ELSE 0 END + \
                CASE WHEN has_legal THEN $12 ELSE 0 END + \
                CASE WHEN has_sponsorships THEN $13 ELSE 0 END + \
                CASE WHEN has_distributors THEN $14 ELSE 0 END + \
                CASE WHEN has_media THEN $15 ELSE 0 END \
            )::INT AS score, \
            has_profile, \
            has_description, \
            has_tagline, \
            has_founded_year, \
            has_location, \
            has_social_handles, \
            has_domains, \
            has_signals, \
            has_funding, \
            has_lab_tests, \
            has_legal, \
            has_sponsorships, \
            has_distributors, \
            has_media \
        FROM presence",
    )
    .bind(brand_id)
    .bind(W_PROFILE)
    .bind(W_DESCRIPTION)
    .bind(W_TAGLINE)
    .bind(W_FOUNDED_YEAR)
    .bind(W_LOCATION)
    .bind(W_SOCIAL)
    .bind(W_DOMAINS)
    .bind(W_SIGNALS)
    .bind(W_FUNDING)
    .bind(W_LAB_TESTS)
    .bind(W_LEGAL)
    .bind(W_SPONSORSHIPS)
    .bind(W_DISTRIBUTORS)
    .bind(W_MEDIA)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_100() {
        let sum = W_PROFILE
            + W_DESCRIPTION
            + W_TAGLINE
            + W_FOUNDED_YEAR
            + W_LOCATION
            + W_SOCIAL
            + W_DOMAINS
            + W_SIGNALS
            + W_FUNDING
            + W_LAB_TESTS
            + W_LEGAL
            + W_SPONSORSHIPS
            + W_DISTRIBUTORS
            + W_MEDIA;
        assert_eq!(sum, 100, "weights must sum to 100, got {sum}");
    }
}
