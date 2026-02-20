use chrono::Utc;

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
        let last_action = super::fmt_date(bill.last_action_date);
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
        let date = super::fmt_date(event.event_date);
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

    if bills.len() >= usize::try_from(REPORT_LIMIT).unwrap_or(usize::MAX) {
        eprintln!("warning: report limit of {REPORT_LIMIT} reached — some bills may be omitted");
    }

    let bill_ids: Vec<i64> = bills.iter().map(|b| b.id).collect();
    let mut events_by_bill = scbdb_db::list_bill_events_batch(pool, &bill_ids).await?;

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

        let last_action = super::fmt_date(bill.last_action_date);
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

        if let Some(mut events) = events_by_bill.remove(&bill.id) {
            if !events.is_empty() {
                events.reverse();
                println!("### Timeline");
                println!();
                println!("| Date | Chamber | Action |");
                println!("|------|---------|--------|");
                for event in &events {
                    let date = super::fmt_date(event.event_date);
                    let chamber = event.chamber.as_deref().unwrap_or("\u{2014}");
                    let description = event.description.replace('|', "\\|");
                    println!("| {date} | {chamber} | {description} |");
                }
            }
        }

        println!();
        println!("---");
    }

    Ok(())
}
