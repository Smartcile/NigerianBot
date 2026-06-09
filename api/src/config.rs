//! API-service configuration, sourced from the environment.

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    #[allow(dead_code)] // reserved for Phase 3 (persistence)
    pub database_url: Option<String>,
    /// Secret used to sign/verify JWTs (Phase 4).
    #[allow(dead_code)] // reserved for Phase 4 (auth)
    pub jwt_secret: String,
}

impl ApiConfig {
    pub fn from_env() -> Result<Self> {
        let port = common::config::optional_or("API_PORT", "8000")
            .parse::<u16>()
            .context("API_PORT must be a valid port number")?;

        Ok(Self {
            host: common::config::optional_or("API_HOST", "0.0.0.0"),
            port,
            database_url: common::config::optional("DATABASE_URL"),
            jwt_secret: common::config::optional_or("JWT_SECRET", "dev-insecure-change-me"),
        })
    }
}
