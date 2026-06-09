//! Small environment-variable helpers shared by all services.

use anyhow::{anyhow, Result};
use std::env;

/// Load variables from a `.env` file if one is present.
///
/// Missing file is not an error — in production we rely on real environment
/// variables (e.g. injected by Docker / the orchestrator).
pub fn load_dotenv() {
    match dotenvy::dotenv() {
        Ok(path) => tracing::debug!(?path, "loaded .env file"),
        Err(e) if e.not_found() => {}
        Err(e) => tracing::warn!(?e, "failed to load .env file"),
    }
}

/// Fetch a required variable, returning a descriptive error if it is unset.
pub fn require(key: &str) -> Result<String> {
    env::var(key).map_err(|_| anyhow!("missing required environment variable: {key}"))
}

/// Fetch an optional variable, treating empty strings as absent.
pub fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.is_empty())
}

/// Fetch an optional variable or fall back to `default`.
pub fn optional_or(key: &str, default: &str) -> String {
    optional(key).unwrap_or_else(|| default.to_string())
}
