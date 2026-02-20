//! Read-only sentiment query handlers.

use chrono::Utc;

/// Show recent sentiment scores for brands.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub(crate) async fn run_sentiment_status(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<()> {
    let brand_id = if let Some(slug) = brand_filter {
        let brand = scbdb_db::get_brand_by_slug(pool, slug)
            .await?
            .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
        Some(brand.id)
    } else {
        None
    };

    let snapshots = scbdb_db::list_sentiment_snapshots(pool, brand_id, 20).await?;

    if snapshots.is_empty() {
        println!(
            "no sentiment snapshots found{}; run `sentiment collect` first",
            brand_filter
                .map(|s| format!(" for brand '{s}'"))
                .unwrap_or_default()
        );
        return Ok(());
    }

    println!("{:<20}{:<12}{:<10}SIGNALS", "BRAND_ID", "CAPTURED", "SCORE");
    for snap in &snapshots {
        let captured = snap.captured_at.format("%Y-%m-%d %H:%M").to_string();
        println!(
            "{:<20}{:<12}{:<10}{}",
            snap.brand_id, captured, snap.score, snap.signal_count
        );
    }

    Ok(())
}

/// Generate a markdown sentiment report.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub(crate) async fn run_sentiment_report(
    pool: &sqlx::PgPool,
    brand_filter: Option<&str>,
) -> anyhow::Result<()> {
    let brand_id = if let Some(slug) = brand_filter {
        let brand = scbdb_db::get_brand_by_slug(pool, slug)
            .await?
            .ok_or_else(|| anyhow::anyhow!("brand '{slug}' not found"))?;
        Some(brand.id)
    } else {
        None
    };

    let snapshots = scbdb_db::list_sentiment_snapshots(pool, brand_id, 100).await?;

    if snapshots.is_empty() {
        println!("no sentiment data to report; run `sentiment collect` first");
        return Ok(());
    }

    let now = Utc::now().format("%Y-%m-%d %H:%M UTC");
    let filter_label = brand_filter.unwrap_or("All brands");

    println!("# Sentiment Report");
    println!();
    println!("**Generated**: {now}");
    println!("**Filter**: {filter_label}");
    println!("**Snapshots**: {}", snapshots.len());
    println!();
    println!("---");
    println!();
    println!("| Brand ID | Captured At | Score | Signals |");
    println!("|----------|-------------|-------|---------|");

    for snap in &snapshots {
        let captured = snap.captured_at.format("%Y-%m-%d %H:%M UTC").to_string();
        println!(
            "| {} | {} | {} | {} |",
            snap.brand_id, captured, snap.score, snap.signal_count
        );
    }

    Ok(())
}
