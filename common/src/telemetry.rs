//! Structured logging setup shared by all services.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the global tracing subscriber.
///
/// Honors the `RUST_LOG` environment variable; defaults to `info` when unset.
/// Safe to call once per process at startup.
pub fn init(service: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();

    tracing::info!(service, "telemetry initialized");
}
