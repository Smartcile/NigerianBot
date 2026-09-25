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
    /// PIN seeded on first run (must be changed on first sign-in). Default `1234`.
    pub default_pin: String,
    /// Lifetime of issued tokens, in seconds.
    pub token_ttl_secs: i64,
    /// Directory of finished downloads, served read-only at `/media`.
    pub downloads_path: String,
    /// Path where the GUI-uploaded yt-dlp cookies file is stored.
    pub cookies_path: String,
    /// Sonarr (TV) base URL + API key, when configured.
    pub sonarr_url: Option<String>,
    pub sonarr_api_key: Option<String>,
    /// Radarr (movies) base URL + API key, when configured.
    pub radarr_url: Option<String>,
    pub radarr_api_key: Option<String>,
    /// Path (as Sonarr sees it) of the shared downloads folder used for imports.
    pub sonarr_import_path: String,
    /// Path (as Radarr sees it) of the shared downloads folder used for imports.
    pub radarr_import_path: String,
    /// Whether a Telegram bot token is present (the telegram service is expected).
    pub telegram_configured: bool,
    /// Chat id the worker notifies on finished downloads (for display).
    pub telegram_chat_id: Option<String>,
    /// Public base URL of this API (for display / link building).
    pub public_base_url: Option<String>,
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
            default_pin: common::config::optional_or("DASHBOARD_PIN", "1234"),
            token_ttl_secs,
            downloads_path: common::config::optional_or("DOWNLOADS_PATH", "/downloads"),
            cookies_path: common::config::optional_or("YTDLP_COOKIES_FILE", "/cookies/cookies.txt"),
            sonarr_url: common::config::optional("SONARR_URL"),
            sonarr_api_key: common::config::optional("SONARR_API_KEY"),
            radarr_url: common::config::optional("RADARR_URL"),
            radarr_api_key: common::config::optional("RADARR_API_KEY"),
            sonarr_import_path: common::config::optional_or("SONARR_IMPORT_PATH", "/downloads"),
            radarr_import_path: common::config::optional_or("RADARR_IMPORT_PATH", "/downloads"),
            telegram_configured: common::config::optional("TELEGRAM_BOT_TOKEN").is_some(),
            telegram_chat_id: common::config::optional("TELEGRAM_CHAT_ID"),
            public_base_url: common::config::optional("PUBLIC_BASE_URL"),
        })
    }
}
