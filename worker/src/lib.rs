//! Worker service: the background job queue (currently the download pipeline).
//!
//! Exposed as a library so the all-in-one `nigerianbot` binary can run it
//! in-process; the `nigerian-worker` binary runs it standalone.

mod config;
mod downloads;
mod notify;
mod tasks;

use std::time::Duration;

use anyhow::Context as _;
use tracing::{info, warn};

use crate::config::WorkerConfig;

/// Connect to Postgres, retrying briefly so the worker tolerates the database
/// still warming up on first stack startup.
async fn connect_db_with_retry(url: &str) -> anyhow::Result<sqlx::PgPool> {
    const MAX_ATTEMPTS: u32 = 10;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match common::db::connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(attempt, error = %e, "database not ready, retrying in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e.context("could not connect to the database")),
        }
    }
}

/// Connect, migrate, and process queued jobs until the process stops.
pub async fn run() -> anyhow::Result<()> {
    let config = WorkerConfig::from_env().context("invalid worker configuration")?;

    let pool = connect_db_with_retry(&config.database_url).await?;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    info!("database connected and migrations applied");

    // Make sure the download directory exists (the volume mount usually does).
    if let Err(e) = std::fs::create_dir_all(&config.downloads_path) {
        warn!(path = %config.downloads_path, error = %e, "could not create downloads directory");
    }

    info!(
        interval_secs = config.poll_interval_secs,
        downloads_path = %config.downloads_path,
        "worker started"
    );

    let interval = Duration::from_secs(config.poll_interval_secs);
    loop {
        tasks::run_once(&pool, &config).await;
        tokio::time::sleep(interval).await;
    }
}
