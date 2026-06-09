//! Async task processing.
//!
//! Phase 8 pulls queued jobs (external API calls, scheduled workflow steps) from
//! the database and executes them. For now `run_once` is a no-op tick.

use tracing::debug;

/// Process one batch of queued work. Currently a placeholder.
pub async fn run_once() {
    debug!("worker tick — no tasks queued");
}
