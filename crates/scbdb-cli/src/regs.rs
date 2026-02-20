//! Regulatory tracking command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. The `ingest` subcommand fetches bills from the `LegiScan` API
//! and persists them; the remaining subcommands are read-only queries.

use chrono::{NaiveDate, Utc};
use clap::Subcommand;
use scbdb_legiscan::{normalize_bill, normalize_bill_events, LegiscanClient};

/// Sub-commands available under `regs`.
#[derive(Debug, Subcommand)]
pub enum RegsCommands {
    /// Ingest bills from `LegiScan` API
    Ingest {
        /// State to ingest bills for (e.g., SC)
        #[arg(long, default_value = "SC")]
        state: String,
        /// Search keyword (defaults to "hemp")
        #[arg(long)]
        keyword: Option<String>,
        /// Preview what would be ingested without writing to the database
        #[arg(long)]
        dry_run: bool,
    },
    /// Show current status of tracked bills
    Status {
        /// Filter by state (e.g., SC)
        #[arg(long)]
        state: Option<String>,
        /// Maximum number of bills to show
        #[arg(long, default_value = "20")]
        limit: i64,
    },
    /// Show event timeline for a specific bill
    Timeline {
        /// State abbreviation (e.g., SC)
        #[arg(long)]
        state: String,
        /// Bill number (e.g., HB1234)
        #[arg(long)]
        bill: String,
    },
    /// Generate a markdown regulatory report
    Report {
        /// Filter by state (e.g., SC)
        #[arg(long)]
        state: Option<String>,
    },
}

/// Format an optional date for display, returning `"—"` when `None`.
fn fmt_date(date: Option<NaiveDate>) -> String {
    date.map_or_else(
        || "\u{2014}".to_string(),
        |d| d.format("%Y-%m-%d").to_string(),
    )
}

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
        println!("dry-run: would ingest bills for state {state}");
        return Ok(());
    }

    let run = scbdb_db::create_collection_run(pool, "regs", "cli").await?;
    scbdb_db::start_collection_run(pool, run.id).await?;

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
                fail_run_best_effort(pool, run.id, "regs", message).await;
                return Err(err.into());
            }
            println!("ingested {total_bills} bills, {total_events} events for state {state}");
            Ok(())
        }
        Err(err) => {
            fail_run_best_effort(pool, run.id, "regs", format!("{err:#}")).await;
            Err(err)
        }
    }
}

/// Show current status of tracked bills.
///
/// Prints a table of bills optionally filtered by state, up to `limit` rows.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub(crate) async fn run_regs_status(
    pool: &sqlx::PgPool,
    state_filter: Option<&str>,
    limit: i64,
) -> anyhow::Result<()> {
    let bills = scbdb_db::list_bills(pool, state_filter, limit).await?;

    if bills.is_empty() {
        println!(
            "no bills found{}; run `regs ingest` first",
            state_filter
                .map(|s| format!(" for state {s}"))
                .unwrap_or_default()
        );
        return Ok(());
    }

    let header = format!(
        "{:<14}{:<11}{:<13}{:<13}TITLE",
        "JURISDICTION", "BILL", "STATUS", "LAST ACTION"
    );
    println!("{header}");
    for bill in &bills {
        let last_action = fmt_date(bill.last_action_date);
        let title_display = if bill.title.chars().count() > 50 {
            format!("{}...", bill.title.chars().take(50).collect::<String>())
        } else {
            bill.title.clone()
        };
        println!(
            "{:<14}{:<11}{:<13}{:<13}{}",
            bill.jurisdiction, bill.bill_number, bill.status, last_action, title_display
        );
    }

    Ok(())
}

/// Show event timeline for a specific bill.
///
/// Looks up the bill by jurisdiction and bill number, then prints its event
/// history in chronological order (ascending by date).
///
/// # Errors
///
/// Returns an error if the bill is not found or the database query fails.
pub(crate) async fn run_regs_timeline(
    pool: &sqlx::PgPool,
    state: &str,
    bill_number: &str,
) -> anyhow::Result<()> {
    let bill = scbdb_db::get_bill_by_jurisdiction_number(pool, state, bill_number)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "bill '{bill_number}' in state '{state}' not found; run `regs ingest` first"
            )
        })?;

    let mut events = scbdb_db::list_bill_events(pool, bill.id).await?;
    // DB returns DESC; reverse for chronological display.
    events.reverse();

    println!("Bill: {} \u{2014} {}", bill.bill_number, bill.title);
    println!("Status: {}", bill.status);
    println!();
    let header = format!("{:<12}{:<9}ACTION", "DATE", "CHAMBER");
    println!("{header}");
    for event in &events {
        let date = fmt_date(event.event_date);
        let chamber = event.chamber.as_deref().unwrap_or("\u{2014}");
        println!("{:<12}{:<9}{}", date, chamber, event.description);
    }

    Ok(())
}

/// Generate a markdown regulatory report to stdout.
///
/// Lists all tracked bills (optionally filtered by state), including their
/// timeline events, formatted as a markdown document.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub(crate) async fn run_regs_report(
    pool: &sqlx::PgPool,
    state_filter: Option<&str>,
) -> anyhow::Result<()> {
    const REPORT_LIMIT: i64 = 100;
    let bills = scbdb_db::list_bills(pool, state_filter, REPORT_LIMIT).await?;

    if bills.is_empty() {
        println!("no bills to report");
        return Ok(());
    }

    if bills.len() as i64 >= REPORT_LIMIT {
        eprintln!("warning: report limit of {REPORT_LIMIT} reached — some bills may be omitted");
    }

    let now = Utc::now().format("%Y-%m-%d %H:%M UTC");
    let jurisdiction = state_filter.unwrap_or("All");

    println!("# Regulatory Report");
    println!();
    println!("**Generated**: {now}");
    println!("**Jurisdiction**: {jurisdiction}");
    println!("**Bills tracked**: {}", bills.len());
    println!();
    println!("---");

    for bill in &bills {
        println!();
        println!("## {}: {}", bill.bill_number, bill.title);
        println!();

        let last_action = fmt_date(bill.last_action_date);
        let session = bill.session.as_deref().unwrap_or("\u{2014}");

        println!(
            "**Status**: {} | **Last Action**: {} | **Session**: {}",
            bill.status, last_action, session
        );
        println!();

        if let Some(ref summary) = bill.summary {
            println!("{summary}");
            println!();
        }

        let mut events = scbdb_db::list_bill_events(pool, bill.id).await?;
        if !events.is_empty() {
            events.reverse();
            println!("### Timeline");
            println!();
            println!("| Date | Chamber | Action |");
            println!("|------|---------|--------|");
            for event in &events {
                let date = fmt_date(event.event_date);
                let chamber = event.chamber.as_deref().unwrap_or("\u{2014}");
                let description = event.description.replace('|', "\\|");
                println!("| {date} | {chamber} | {description} |");
            }
        }

        println!();
        println!("---");
    }

    Ok(())
}

/// Attempt to mark a collection run as failed, logging any secondary error.
async fn fail_run_best_effort(
    pool: &sqlx::PgPool,
    run_id: i64,
    context: &'static str,
    message: String,
) {
    if let Err(mark_err) = scbdb_db::fail_collection_run(pool, run_id, &message).await {
        tracing::error!(
            run_id,
            error = %mark_err,
            "failed to mark {context} run as failed"
        );
    }
}
