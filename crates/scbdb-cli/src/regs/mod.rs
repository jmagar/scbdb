//! Regulatory tracking command handlers for the CLI.
//!
//! These are called from `main` after the database pool and config are
//! established. The `ingest` subcommand fetches bills from the `LegiScan` API
//! and persists them; the remaining subcommands are read-only queries.

mod ingest;
mod query;

use chrono::NaiveDate;
use clap::Subcommand;

pub(crate) use ingest::run_regs_ingest;
pub(crate) use query::{run_regs_report, run_regs_status, run_regs_timeline};

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
        limit: u32,
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
