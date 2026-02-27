//! Bill discovery phase for the `regs ingest` pipeline.
//!
//! Encapsulates the `getMasterList` / `getSessionList` + `getMasterList(session_id)`
//! discovery strategies and the local keyword-filter step. Returns a candidate map
//! keyed by `legiscan_bill_id` for the hash-check and fetch phases.

use std::collections::HashMap;

use scbdb_legiscan::{types::MasterListEntry, LegiscanClient, LegiscanError};

/// Candidates map: `legiscan_bill_id → (change_hash, MasterListEntry)`.
pub(super) type CandidateMap = HashMap<i64, (String, MasterListEntry)>;

/// Discover matching bills across the given states.
///
/// - **Normal mode** (`all_sessions = false`): calls `getMasterList(state)` once per state
///   (1 request/state). Covers the current active session only.
///
/// - **Backfill mode** (`all_sessions = true`): calls `getSessionList(state)` first (1 request),
///   then `getMasterList(session_id)` for every session on record. Covers all historical sessions.
///
/// In both modes, entries are filtered locally by keyword match on bill title. No extra API
/// requests are consumed by the keyword filter.
///
/// Returns `(candidates, budget_hit)`.  `budget_hit` is `true` when the session request budget
/// was reached before all states/sessions were processed.
///
/// # Errors
///
/// Returns `Err` only on quota exhaustion ([`LegiscanError::QuotaExceeded`]).  All other
/// per-state/per-session errors are logged and skipped.
pub(super) async fn discover_candidates(
    client: &LegiscanClient,
    states: &[String],
    keywords: &[String],
    all_sessions: bool,
) -> anyhow::Result<(CandidateMap, bool)> {
    let mut candidates: CandidateMap = HashMap::new();
    let mut budget_hit = false;

    'discovery: for state in states {
        if all_sessions {
            tracing::info!(state, "getSessionList (all-sessions backfill)");
            let sessions = match client.get_session_list(state).await {
                Ok(s) => s,
                Err(LegiscanError::BudgetExceeded { used, limit }) => {
                    tracing::warn!(used, limit, state, "budget reached — stopping discovery");
                    budget_hit = true;
                    break 'discovery;
                }
                Err(LegiscanError::QuotaExceeded(ref msg)) => {
                    return Err(anyhow::anyhow!(
                        "LegiScan quota exhausted during getSessionList(state={state}): {msg}"
                    ));
                }
                Err(e) => {
                    tracing::warn!(state, error = %e, "getSessionList failed — skipping state");
                    continue;
                }
            };

            tracing::info!(
                state,
                sessions = sessions.len(),
                "fetching masterlist per session"
            );
            for session in &sessions {
                tracing::info!(
                    state,
                    session_id = session.session_id,
                    session_name = %session.session_name,
                    "getMasterList by session"
                );
                let entries = match client.get_master_list_by_session(session.session_id).await {
                    Ok((_detail, e)) => e,
                    Err(LegiscanError::BudgetExceeded { used, limit }) => {
                        tracing::warn!(used, limit, "budget reached — stopping session scan");
                        budget_hit = true;
                        break 'discovery;
                    }
                    Err(LegiscanError::QuotaExceeded(ref msg)) => {
                        return Err(anyhow::anyhow!(
                            "LegiScan quota exhausted during getMasterList(session_id={}): {msg}",
                            session.session_id
                        ));
                    }
                    Err(e) => {
                        tracing::warn!(
                            session_id = session.session_id,
                            error = %e,
                            "getMasterList by session failed — skipping"
                        );
                        continue;
                    }
                };
                collect_matching(entries, keywords, &mut candidates);
            }
        } else {
            tracing::info!(state, "getMasterList");
            let entries = match client.get_master_list(state).await {
                Ok((_session, entries)) => entries,
                Err(LegiscanError::BudgetExceeded { used, limit }) => {
                    tracing::warn!(used, limit, state, "budget reached — stopping discovery");
                    budget_hit = true;
                    break 'discovery;
                }
                Err(LegiscanError::QuotaExceeded(ref msg)) => {
                    return Err(anyhow::anyhow!(
                        "LegiScan quota exhausted during getMasterList(state={state}): {msg}"
                    ));
                }
                Err(e) => {
                    tracing::warn!(state, error = %e, "getMasterList failed — skipping state");
                    continue;
                }
            };
            collect_matching(entries, keywords, &mut candidates);
        }
    }

    Ok((candidates, budget_hit))
}

/// Inserts entries whose titles match any keyword into `candidates`.
///
/// Uses `entry()` so a bill seen in multiple sessions (backfill) keeps the
/// first-seen hash without overwriting.
fn collect_matching(
    entries: Vec<MasterListEntry>,
    keywords: &[String],
    candidates: &mut CandidateMap,
) {
    for entry in entries {
        let title_lower = entry.title.to_lowercase();
        if keywords
            .iter()
            .any(|kw| title_lower.contains(kw.to_lowercase().as_str()))
        {
            candidates
                .entry(entry.bill_id)
                .or_insert_with(|| (entry.change_hash.clone(), entry));
        }
    }
}
