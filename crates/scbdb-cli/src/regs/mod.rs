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
        /// States to search (repeat for multiple: --state SC --state US).
        /// "US" fetches US Congress bills.
        #[arg(long, default_value = "SC")]
        state: Vec<String>,

        /// Keywords to search (repeat for multiple: --keyword hemp --keyword "intoxicating hemp").
        /// Defaults to "hemp" when not specified.
        #[arg(long)]
        keyword: Vec<String>,

        /// Maximum result pages per keyword search (50 results/page).
        /// Lower this to reduce API request usage.
        #[arg(long, default_value = "3")]
        max_pages: u32,

        /// Hard ceiling on `LegiScan` API requests for this run.
        /// Protects the 30 000/month quota. A full sweep of 8 keywords × 2
        /// states × 3 pages + ~250 bill fetches uses ≈ 300 requests.
        #[arg(long, default_value = "300")]
        max_requests: u32,

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
