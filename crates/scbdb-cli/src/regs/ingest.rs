use scbdb_legiscan::{normalize_bill, normalize_bill_events, LegiscanClient};

/// Ingest bills from the `LegiScan` API for a given state.
///
/// Searches for bills matching the provided keyword (defaulting to `"hemp"`),
/// fetches full details for each result, and upserts bills and events into
/// the database. A collection run is created to track progress.
///
/// When `dry_run` is `true` the function prints what would be ingested and
/// returns without touching the database.
///
/// # Errors
///
/// Returns an error if the API key is missing, the client cannot be built,
/// or the collection run cannot be created. Individual bill fetch failures
/// are logged and skipped, not propagated.
pub(crate) async fn run_regs_ingest(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    state: &str,
    keyword: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let api_key = config
        .legiscan_api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("LEGISCAN_API_KEY is not set; cannot run regs ingest"))?;

    let client = LegiscanClient::new(api_key, config.scraper_request_timeout_secs)
        .map_err(|e| anyhow::anyhow!("failed to build LegiScan client: {e}"))?;

    let keyword = keyword.unwrap_or("hemp");

    if dry_run {
        println!("dry-run: would ingest bills for state {state} (keyword: {keyword:?})");
        return Ok(());
    }

    let run = scbdb_db::create_collection_run(pool, "regs", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        crate::fail_run_best_effort(pool, run.id, "regs", format!("{e:#}")).await;
        return Err(e.into());
    }

    let mut total_bills: i32 = 0;
    let mut total_events: i32 = 0;

    let result: anyhow::Result<()> = async {
        let search_results = client.search_bills(keyword, Some(state)).await?;

        for item in &search_results {
            let detail = match client.get_bill(item.bill_id).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(
                        bill_id = item.bill_id,
                        error = %e,
                        "skipping bill \u{2014} failed to fetch detail"
                    );
                    continue;
                }
            };

            let normalized = normalize_bill(&detail);
            let events = normalize_bill_events(&detail);

            let bill_id = scbdb_db::upsert_bill(
                pool,
                &normalized.jurisdiction,
                &normalized.bill_number,
                &normalized.title,
                normalized.summary.as_deref(),
                &normalized.status,
                normalized.status_date,
                normalized.introduced_date,
                normalized.last_action_date,
                normalized.session.as_deref(),
                normalized.source_url.as_deref(),
            )
            .await?;

            // DB write failures are treated as fatal — `?` aborts the entire
            // ingestion run and marks it failed. API fetch failures for
            // individual bills (above) are non-fatal and logged as warnings.
            for event in &events {
                scbdb_db::upsert_bill_event(
                    pool,
                    bill_id,
                    event.event_date,
                    event.event_type.as_deref(),
                    event.chamber.as_deref(),
                    &event.description,
                    event.source_url.as_deref(),
                )
                .await?;
            }

            total_bills = total_bills.saturating_add(1);
            total_events =
                total_events.saturating_add(i32::try_from(events.len()).unwrap_or(i32::MAX));
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_bills).await {
                let message = format!("{err:#}");
                crate::fail_run_best_effort(pool, run.id, "regs", message).await;
                return Err(err.into());
            }
            println!("ingested {total_bills} bills, {total_events} events for state {state}");
            Ok(())
        }
        Err(err) => {
            crate::fail_run_best_effort(pool, run.id, "regs", format!("{err:#}")).await;
            Err(err)
        }
    }
}
