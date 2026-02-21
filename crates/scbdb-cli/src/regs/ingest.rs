use std::collections::HashMap;

use scbdb_legiscan::{
    normalize_bill, normalize_bill_events, normalize_bill_texts, LegiscanClient, LegiscanError,
};

/// Ingest bills from the `LegiScan` API for one or more states and keywords.
///
/// Uses `getMasterList` (1 request per state) to discover bills, then filters
/// locally by keyword match on title. Cross-references incoming `change_hash`
/// values against stored hashes in a single batch DB query, and only calls
/// `getBill` for new or changed bills. This reduces steady-state API spend
/// from `N_pages + N_bills` to `N_states + N_changed_bills`.
///
/// **Request budget:** every `getMasterList` and every `getBill` call counts
/// against `max_requests`. When the budget is reached the fetch phase stops
/// early and the run is marked succeeded with whatever was collected. A
/// [`LegiscanError::QuotaExceeded`] (API-level quota exhausted) aborts the
/// run immediately and is treated as an error.
///
/// When `dry_run` is `true` the function prints what would be ingested and
/// returns without touching the database.
///
/// # Errors
///
/// Returns an error if the API key is missing, the client cannot be built,
/// or the collection run cannot be created. Individual bill fetch failures
/// are logged and skipped. Quota exhaustion is propagated as an error.
#[allow(clippy::too_many_lines)] // Orchestration: discovery, hash-check, fetch, and collection-run lifecycle
pub(crate) async fn run_regs_ingest(
    pool: &sqlx::PgPool,
    config: &scbdb_core::AppConfig,
    states: &[String],
    keywords: &[String],
    _max_pages: u32,
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
            "dry-run: would ingest bills for states [{}] keywords [{}] (max_requests={max_requests})",
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
        // ── Phase 1: Discover bills via getMasterList ─────────────────────────
        // One request per state; filter locally by keyword — free and fast.
        // Keyed by legiscan_bill_id → (change_hash, MasterListEntry).
        let mut candidates: HashMap<i64, (String, scbdb_legiscan::types::MasterListEntry)> =
            HashMap::new();
        let mut budget_hit = false;

        'discovery: for state in states {
            tracing::info!(state, "getMasterList");
            let entries = match client.get_master_list(state).await {
                Ok((_session, entries)) => entries,
                Err(LegiscanError::BudgetExceeded { used, limit }) => {
                    tracing::warn!(
                        used,
                        limit,
                        state,
                        "request budget reached — stopping discovery phase early"
                    );
                    budget_hit = true;
                    break 'discovery;
                }
                Err(LegiscanError::QuotaExceeded(ref msg)) => {
                    return Err(anyhow::anyhow!(
                        "LegiScan quota exhausted during getMasterList(state={state}): {msg}"
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        state,
                        error = %e,
                        "getMasterList failed — skipping state"
                    );
                    continue;
                }
            };

            // Local keyword filter — no API requests consumed.
            for entry in entries {
                let title_lower = entry.title.to_lowercase();
                let matches = keywords
                    .iter()
                    .any(|kw| title_lower.contains(kw.to_lowercase().as_str()));
                if matches {
                    candidates
                        .entry(entry.bill_id)
                        .or_insert_with(|| (entry.change_hash.clone(), entry));
                }
            }
        }

        tracing::info!(
            candidates = candidates.len(),
            requests_used = client.requests_used(),
            budget_hit,
            "discovery phase complete"
        );

        // ── Phase 2: Hash check — skip unchanged bills ────────────────────────
        let all_ids: Vec<i64> = candidates.keys().copied().collect();
        let stored_hashes = scbdb_db::get_bills_stored_hashes(pool, &all_ids).await?;

        let to_fetch: Vec<i64> = candidates
            .iter()
            .filter(|(id, (incoming_hash, _))| {
                stored_hashes
                    .get(*id)
                    .is_none_or(|stored| stored != incoming_hash)
            })
            .map(|(id, _)| *id)
            .collect();

        let skipped = candidates.len().saturating_sub(to_fetch.len());
        tracing::info!(
            to_fetch = to_fetch.len(),
            skipped_unchanged = skipped,
            "hash-check phase complete"
        );

        // ── Phase 3: Fetch + upsert changed/new bills ─────────────────────────
        let mut total_bills: i32 = 0;
        let mut total_events: i32 = 0;

        for bill_id in &to_fetch {
            let (incoming_hash, _entry) = candidates
                .get(bill_id)
                .expect("bill_id must be in candidates");

            let detail = match client.get_bill(*bill_id).await {
                Ok(d) => d,
                Err(LegiscanError::BudgetExceeded { used, limit }) => {
                    tracing::warn!(
                        used,
                        limit,
                        bill_id,
                        "request budget reached — stopping detail fetch early"
                    );
                    break;
                }
                Err(LegiscanError::QuotaExceeded(ref msg)) => {
                    return Err(anyhow::anyhow!(
                        "LegiScan quota exhausted during getBill(id={bill_id}): {msg}"
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        bill_id,
                        error = %e,
                        "skipping bill — failed to fetch detail"
                    );
                    continue;
                }
            };

            let normalized = normalize_bill(&detail);
            let events = normalize_bill_events(&detail);
            let texts = normalize_bill_texts(&detail);

            let db_bill_id = scbdb_db::upsert_bill(
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
                Some(*bill_id),
                Some(incoming_hash.as_str()),
            )
            .await?;

            for event in &events {
                scbdb_db::upsert_bill_event(
                    pool,
                    db_bill_id,
                    event.event_date,
                    event.event_type.as_deref(),
                    event.chamber.as_deref(),
                    &event.description,
                    event.source_url.as_deref(),
                )
                .await?;
            }

            for text in &texts {
                scbdb_db::upsert_bill_text(
                    pool,
                    db_bill_id,
                    text.legiscan_text_id,
                    text.text_date,
                    &text.text_type,
                    &text.mime,
                    text.legiscan_url.as_deref(),
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
