//! Background job scheduler.
//!
//! Initialises a [`JobScheduler`] at server startup. No collection jobs are
//! registered yet — scheduled product and pricing runs will be wired here in
//! Phase 2 once the CLI collection pipeline is fully implemented.

use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};

/// Builds and starts the background job scheduler.
///
/// Returns the running [`JobScheduler`] handle, which must be kept alive for
/// the lifetime of the process. Dropping it shuts down all scheduled jobs.
///
/// # Errors
///
/// Returns [`JobSchedulerError`] if the scheduler cannot be initialised or
/// started.
pub async fn build_scheduler() -> Result<JobScheduler, JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    // Register scheduled collection jobs here when server-driven scheduling is implemented.
    scheduler.start().await?;
    Ok(scheduler)
}
