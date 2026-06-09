//! Scheduler-service configuration.

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct SchedulerConfig {
    /// Database URL where scheduled-task definitions live (Phase 8).
    #[allow(dead_code)] // reserved for Phase 8 (scheduler logic)
    pub database_url: Option<String>,
}

impl SchedulerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: common::config::optional("DATABASE_URL"),
        })
    }
}
