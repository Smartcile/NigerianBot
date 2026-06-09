//! NigerianBot scheduler service — manages cron-style scheduled workflows.
//!
//! Phase 1 starts a [`JobScheduler`] with a heartbeat job. Phase 8 loads real
//! task definitions from the database and dispatches work to the worker.

mod config;
mod jobs;

use anyhow::Context as _;
use tokio_cron_scheduler::JobScheduler;
use tracing::info;

use crate::config::SchedulerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("scheduler");

    let _config = SchedulerConfig::from_env().context("invalid scheduler configuration")?;

    let scheduler = JobScheduler::new()
        .await
        .context("failed to create job scheduler")?;

    scheduler
        .add(jobs::heartbeat()?)
        .await
        .context("failed to register heartbeat job")?;

    scheduler
        .start()
        .await
        .context("failed to start scheduler")?;
    info!("scheduler started — waiting for jobs");

    // Run until interrupted.
    tokio::signal::ctrl_c()
        .await
        .context("failed to listen for shutdown signal")?;
    info!("shutting down scheduler");

    Ok(())
}
