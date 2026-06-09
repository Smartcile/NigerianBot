//! NigerianBot worker service — processes async tasks and external API calls.
//!
//! Phase 1 runs a poll loop that ticks on an interval. Phase 8 replaces the
//! no-op tick with real queue processing.

mod config;
mod tasks;

use std::time::Duration;

use anyhow::Context as _;
use tracing::info;

use crate::config::WorkerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("worker");

    let config = WorkerConfig::from_env().context("invalid worker configuration")?;
    info!(
        interval_secs = config.poll_interval_secs,
        "worker started — task processing arrives in Phase 8"
    );

    let interval = Duration::from_secs(config.poll_interval_secs);
    loop {
        tasks::run_once().await;
        tokio::time::sleep(interval).await;
    }
}
