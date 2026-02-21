//! Image URL verification for product and brand logo assets.

use futures::stream::{self, StreamExt};
use reqwest::StatusCode;

#[derive(Debug, Clone, sqlx::FromRow)]
struct ProductImageCheckRow {
    brand_slug: String,
    product_name: String,
    primary_image_url: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct BrandLogoCheckRow {
    slug: String,
    logo_url: Option<String>,
}

/// Verify product/brand image URLs currently stored in the database.
///
/// Logs non-200 URLs for cleanup and prints aggregate totals.
pub(super) async fn run_collect_verify_images(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
    concurrency: usize,
) -> anyhow::Result<()> {
    let product_rows = sqlx::query_as::<_, ProductImageCheckRow>(
        "SELECT b.slug AS brand_slug, p.name AS product_name, p.metadata->>'primary_image_url' AS primary_image_url \
         FROM products p \
         JOIN brands b ON b.id = p.brand_id \
         WHERE p.deleted_at IS NULL \
           AND b.deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR b.slug = $1)",
    )
    .bind(brand_filter)
    .fetch_all(pool)
    .await?;

    let brand_rows = sqlx::query_as::<_, BrandLogoCheckRow>(
        "SELECT slug, logo_url \
         FROM brands \
         WHERE deleted_at IS NULL \
           AND ($1::TEXT IS NULL OR slug = $1)",
    )
    .bind(brand_filter)
    .fetch_all(pool)
    .await?;

    let mut targets: Vec<(String, String, String)> = Vec::new();
    targets.extend(
        product_rows
            .into_iter()
            .filter_map(|row| {
                row.primary_image_url.map(|url| {
                    (
                        "product".to_string(),
                        format!("{} / {}", row.brand_slug, row.product_name),
                        url,
                    )
                })
            })
            .collect::<Vec<_>>(),
    );
    targets.extend(
        brand_rows
            .into_iter()
            .filter_map(|row| {
                row.logo_url
                    .map(|url| ("brand".to_string(), row.slug.clone(), url))
            })
            .collect::<Vec<_>>(),
    );

    if targets.is_empty() {
        println!("no image URLs found to verify");
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("scbdb-verifier/1.0")
        .build()?;

    let checks = stream::iter(targets.into_iter().map(|(kind, label, url)| {
        let client = client.clone();
        async move {
            let result = client.head(&url).send().await;
            (kind, label, url, result)
        }
    }))
    .buffer_unordered(concurrency.max(1))
    .collect::<Vec<_>>()
    .await;

    let mut ok_count = 0usize;
    let mut bad_count = 0usize;
    for (kind, label, url, result) in checks {
        match result {
            Ok(resp) if resp.status() == StatusCode::OK => {
                ok_count += 1;
            }
            Ok(resp) => {
                bad_count += 1;
                tracing::warn!(
                    image_kind = %kind,
                    label = %label,
                    status = resp.status().as_u16(),
                    url = %url,
                    "image URL verification failed"
                );
            }
            Err(e) => {
                bad_count += 1;
                tracing::warn!(
                    image_kind = %kind,
                    label = %label,
                    error = %e,
                    url = %url,
                    "image URL verification failed"
                );
            }
        }
    }

    println!("verified image URLs: {ok_count} OK, {bad_count} bad");
    Ok(())
}
