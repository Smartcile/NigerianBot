//! Worker-service configuration.

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    /// Database URL the worker polls for queued tasks (Phase 8).
    #[allow(dead_code)] // reserved for Phase 8 (worker logic)
    pub database_url: Option<String>,
    /// Seconds between poll cycles.
    pub poll_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self> {
        let poll_interval_secs = common::config::optional_or("WORKER_POLL_INTERVAL_SECS", "30")
            .parse::<u64>()
            .unwrap_or(30);

        Ok(Self {
            database_url: common::config::optional("DATABASE_URL"),
            poll_interval_secs,
        })
    }
}
