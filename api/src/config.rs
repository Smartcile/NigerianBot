//! API-service configuration, sourced from the environment.

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    /// Secret used to sign/verify JWTs.
    pub jwt_secret: String,
    /// Shared secret clients present to `/api/auth/login` to obtain a JWT.
    /// Empty means login is disabled (the server logs a warning at startup).
    pub api_key: String,
    /// Lifetime of issued tokens, in seconds.
    pub token_ttl_secs: i64,
}

impl ApiConfig {
    pub fn from_env() -> Result<Self> {
        let port = common::config::optional_or("API_PORT", "8000")
            .parse::<u16>()
            .context("API_PORT must be a valid port number")?;

        let token_ttl_secs = common::config::optional_or("JWT_TTL_SECS", "3600")
            .parse::<i64>()
            .unwrap_or(3600);

        Ok(Self {
            host: common::config::optional_or("API_HOST", "0.0.0.0"),
            port,
            database_url: common::config::optional("DATABASE_URL"),
            jwt_secret: common::config::optional_or("JWT_SECRET", "dev-insecure-change-me"),
            api_key: common::config::optional_or("API_KEY", ""),
            token_ttl_secs,
        })
    }
}
