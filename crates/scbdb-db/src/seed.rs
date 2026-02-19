use scbdb_core::brands::BrandConfig;
use sqlx::PgPool;

use crate::DbError;

/// Upsert brands from config into the database.
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

        sqlx::query(
            "INSERT INTO brands (name, slug, relationship, tier, domain, shop_url, notes, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, true) \
             ON CONFLICT (slug) DO UPDATE SET \
                 name = EXCLUDED.name, \
                 relationship = EXCLUDED.relationship, \
                 tier = EXCLUDED.tier, \
                 domain = EXCLUDED.domain, \
                 shop_url = EXCLUDED.shop_url, \
                 notes = EXCLUDED.notes, \
                 updated_at = NOW()",
        )
        .bind(&brand.name)
        .bind(&slug)
        .bind(&relationship)
        .bind(tier)
        .bind(&brand.domain)
        .bind(&brand.shop_url)
        .bind(&brand.notes)
        .execute(&mut *tx)
        .await?;

        count += 1;
    }

    tx.commit().await?;
    Ok(count)
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
