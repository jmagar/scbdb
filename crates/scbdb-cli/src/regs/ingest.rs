use std::collections::HashSet;

use scbdb_legiscan::{normalize_bill, normalize_bill_events, LegiscanClient, LegiscanError};

/// Ingest bills from the `LegiScan` API for one or more states and keywords.
///
/// Searches every (state × keyword) combination, deduplicates results by
/// `bill_id` across all searches, then fetches full detail for each unique
/// bill and upserts it into the database.
///
/// **Request budget:** every page fetch and every `getBill` call counts against
/// `max_requests`. When the budget is reached the search or detail phase stops
/// early and the run is marked succeeded with whatever was collected. A
/// [`LegiscanError::QuotaExceeded`] (API-level quota exhausted) aborts the run
/// immediately and is treated as an error.
///
/// When `dry_run` is `true` the function prints what would be ingested and
/// returns without touching the database.
///
/// # Errors
///
/// Returns an error if the API key is missing, the client cannot be built,
/// or the collection run cannot be created. Individual bill fetch failures
/// are logged and skipped. Quota exhaustion is propagated as an error.
#[allow(clippy::too_many_lines)] // Orchestration: search phase, dedup, detail-fetch phase, collection-run lifecycle
pub(crate) async fn run_regs_ingest(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    states: &[String],
    keywords: &[String],
    max_pages: u32,
    max_requests: u32,
    dry_run: bool,
) -> anyhow::Result<()> {
    let api_key = config
        .legiscan_api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("LEGISCAN_API_KEY is not set; cannot run regs ingest"))?;

    // Default keyword when none are supplied.
    let default_kw = vec!["hemp".to_string()];
    let keywords = if keywords.is_empty() {
        &default_kw
    } else {
        keywords
    };

    if dry_run {
        println!(
            "dry-run: would ingest bills for states [{}] keywords [{}] (max_pages={max_pages}, max_requests={max_requests})",
            states.join(", "),
            keywords.join(", "),
        );
        return Ok(());
    }

    let client = LegiscanClient::new(api_key, config.legiscan_request_timeout_secs, max_requests)
        .map_err(|e| anyhow::anyhow!("failed to build LegiScan client: {e}"))?;

    let run = scbdb_db::create_collection_run(pool, "regs", "cli").await?;
    if let Err(e) = scbdb_db::start_collection_run(pool, run.id).await {
        crate::fail_run_best_effort(pool, run.id, "regs", format!("{e:#}")).await;
        return Err(e.into());
    }

    let result: anyhow::Result<(i32, i32)> = async {
        // ── Search phase ──────────────────────────────────────────────────────
        // Collect unique bill_ids across all (state × keyword) combinations to
        // avoid redundant getBill calls when keywords overlap.
        let mut seen: HashSet<i64> = HashSet::new();
        let mut search_items = Vec::new();
        let mut budget_hit = false;

        'search: for state in states {
            for keyword in keywords {
                tracing::info!(state, keyword, "searching LegiScan");
                match client.search_bills(keyword, Some(state.as_str()), max_pages).await {
                    Ok(items) => {
                        for item in items {
                            if seen.insert(item.bill_id) {
                                search_items.push(item);
                            }
                        }
                    }
                    Err(LegiscanError::BudgetExceeded { used, limit }) => {
                        tracing::warn!(
                            used,
                            limit,
                            state,
                            keyword,
                            "request budget reached — stopping search phase early"
                        );
                        budget_hit = true;
                        break 'search;
                    }
                    Err(LegiscanError::QuotaExceeded(ref msg)) => {
                        return Err(anyhow::anyhow!(
                            "LegiScan quota exhausted during search (state={state}, keyword={keyword}): {msg}"
                        ));
                    }
                    Err(e) => {
                        tracing::warn!(
                            state,
                            keyword,
                            error = %e,
                            "search failed — skipping keyword"
                        );
                    }
                }
            }
        }

        tracing::info!(
            unique_bills = search_items.len(),
            requests_used = client.requests_used(),
            budget_hit,
            "search phase complete"
        );

        // ── Detail fetch + upsert phase ───────────────────────────────────────
        let mut total_bills: i32 = 0;
        let mut total_events: i32 = 0;

        for item in &search_items {
            let detail = match client.get_bill(item.bill_id).await {
                Ok(d) => d,
                Err(LegiscanError::BudgetExceeded { used, limit }) => {
                    tracing::warn!(
                        used,
                        limit,
                        bill_id = item.bill_id,
                        "request budget reached — stopping detail fetch early"
                    );
                    break;
                }
                Err(LegiscanError::QuotaExceeded(ref msg)) => {
                    return Err(anyhow::anyhow!(
                        "LegiScan quota exhausted during getBill(id={}): {msg}",
                        item.bill_id
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        bill_id = item.bill_id,
                        error = %e,
                        "skipping bill — failed to fetch detail"
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

        Ok((total_bills, total_events))
    }
    .await;

    match result {
        Ok((total_bills, total_events)) => {
            if let Err(err) = scbdb_db::complete_collection_run(pool, run.id, total_bills).await {
                let message = format!("{err:#}");
                crate::fail_run_best_effort(pool, run.id, "regs", message).await;
                return Err(err.into());
            }
            println!(
                "ingested {total_bills} bills, {total_events} events \
                 ({} API requests used of {max_requests} allowed)",
                client.requests_used()
            );
            Ok(())
        }
        Err(err) => {
            crate::fail_run_best_effort(pool, run.id, "regs", format!("{err:#}")).await;
            Err(err)
        }
    }
}
