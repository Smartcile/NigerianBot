//! Scheduled job definitions.
//!
//! Phase 8 loads `scheduled_tasks` from the database and registers them with the
//! cron scheduler. For now we expose a single heartbeat job so the service has
//! observable, healthy behavior end to end.

use tokio_cron_scheduler::Job;
use tracing::info;

/// A once-a-minute heartbeat, proving the scheduler is alive.
pub fn heartbeat() -> anyhow::Result<Job> {
    let job = Job::new_async("0 * * * * *", |_uuid, _lock| {
        Box::pin(async {
            info!("scheduler heartbeat");
        })
    })?;
    Ok(job)
}
