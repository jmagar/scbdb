use scbdb_core::brands::BrandConfig;
use sqlx::PgPool;

use crate::DbError;

/// Upsert brands from config into the database, including social handles and domains.
///
/// Returns the number of brands processed (inserted or updated).
/// All upserts run inside a single transaction; if any operation fails
/// the entire batch is rolled back.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if any database operation fails.
pub async fn seed_brands(pool: &PgPool, brands: &[BrandConfig]) -> Result<usize, DbError> {
    let mut tx = pool.begin().await?;
    let mut count = 0usize;

    for brand in brands {
        let slug = brand.slug();
        let relationship = brand.relationship.to_string();
        let tier = i16::from(brand.tier);

        let brand_id: i64 = sqlx::query_scalar(
            "INSERT INTO brands (name, slug, relationship, tier, domain, shop_url, store_locator_url, notes, twitter_handle, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true) \
             ON CONFLICT (slug) DO UPDATE SET \
                 name = EXCLUDED.name, \
                 relationship = EXCLUDED.relationship, \
                 tier = EXCLUDED.tier, \
                 domain = EXCLUDED.domain, \
                 shop_url = EXCLUDED.shop_url, \
                 store_locator_url = EXCLUDED.store_locator_url, \
                 notes = EXCLUDED.notes, \
                 twitter_handle = EXCLUDED.twitter_handle, \
                 updated_at = NOW() \
             RETURNING id",
        )
        .bind(&brand.name)
        .bind(&slug)
        .bind(&relationship)
        .bind(tier)
        .bind(&brand.domain)
        .bind(&brand.shop_url)
        .bind(&brand.store_locator_url)
        .bind(&brand.notes)
        .bind(&brand.twitter_handle)
        .fetch_one(&mut *tx)
        .await?;

        for (platform, handle) in &brand.social {
            sqlx::query(
                "INSERT INTO brand_social_handles (brand_id, platform, handle) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (brand_id, platform, handle) DO UPDATE \
                 SET updated_at = NOW()",
            )
            .bind(brand_id)
            .bind(platform)
            .bind(handle)
            .execute(&mut *tx)
            .await?;
        }

        for domain in &brand.domains {
            sqlx::query(
                "INSERT INTO brand_domains (brand_id, domain, domain_type) \
                 VALUES ($1, $2, 'primary') \
                 ON CONFLICT (brand_id, domain) DO UPDATE \
                 SET updated_at = NOW()",
            )
            .bind(brand_id)
            .bind(domain)
            .execute(&mut *tx)
            .await?;
        }

        count += 1;
    }

    tx.commit().await?;
    Ok(count)
}

/// Upsert social handles for a brand from config.
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn upsert_brand_social_handles<S: std::hash::BuildHasher>(
    pool: &PgPool,
    brand_id: i64,
    social: &std::collections::HashMap<String, String, S>,
) -> Result<(), DbError> {
    for (platform, handle) in social {
        sqlx::query(
            "INSERT INTO brand_social_handles (brand_id, platform, handle) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (brand_id, platform, handle) DO UPDATE \
             SET updated_at = NOW()",
        )
        .bind(brand_id)
        .bind(platform)
        .bind(handle)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Upsert domain entries for a brand from config.
///
/// Note: Also used by the server API enrichment endpoint
/// (`scbdb-server::api::brands::write_enrichment::upsert_brand_domains`),
/// not only by the seed workflow.
///
/// # Errors
///
/// Returns `DbError` on database query failure.
pub async fn upsert_brand_domains(
    pool: &PgPool,
    brand_id: i64,
    domains: &[String],
) -> Result<(), DbError> {
    for domain in domains {
        sqlx::query(
            "INSERT INTO brand_domains (brand_id, domain, domain_type) \
             VALUES ($1, $2, 'primary') \
             ON CONFLICT (brand_id, domain) DO UPDATE \
             SET updated_at = NOW()",
        )
        .bind(brand_id)
        .bind(domain)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn seed_module_is_accessible() {
        // Verify the module compiles and DbError is visible from the seed module.
        // Slug logic is tested in scbdb-core.
        let _ = std::mem::size_of::<crate::DbError>();
    }
}
