//! Async task processing. Drains the download queue, one job at a time until
//! empty, then the caller sleeps until the next poll.

use sqlx::PgPool;
use tracing::warn;

use crate::config::WorkerConfig;
use crate::downloads;

/// Process queued downloads until the queue is empty (or the first job errors).
pub async fn run_once(pool: &PgPool, config: &WorkerConfig) {
    loop {
        match downloads::process_next(pool, config).await {
            Ok(true) => continue,
            Ok(false) => break,
            Err(e) => {
                warn!(?e, "worker job error");
                break;
            }
        }
    }
}
