//! Worker-service configuration.

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    /// Database URL the worker polls for queued jobs (required).
    pub database_url: String,
    /// Seconds between poll cycles.
    pub poll_interval_secs: u64,
    /// Directory (inside the container) where finished downloads are written.
    pub downloads_path: String,
    /// Netscape cookies file passed to yt-dlp (for sites needing login, e.g.
    /// some Vimeo videos). Uploaded via the web GUI; path overridable by env.
    pub cookies_file: String,
    /// Optional outbound notification targets (set either or both).
    pub discord_webhook: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    /// Optional public base URL of the API, used to build file links in
    /// notifications (e.g. `https://bot.example.com` → `.../media/<file>`).
    pub public_base_url: Option<String>,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self> {
        let poll_interval_secs = common::config::optional_or("WORKER_POLL_INTERVAL_SECS", "30")
            .parse::<u64>()
            .unwrap_or(30);

        Ok(Self {
            database_url: common::config::require("DATABASE_URL")?,
            poll_interval_secs,
            downloads_path: common::config::optional_or("DOWNLOADS_PATH", "/downloads"),
            cookies_file: common::config::optional_or("YTDLP_COOKIES_FILE", "/cookies/cookies.txt"),
            discord_webhook: common::config::optional("DISCORD_NOTIFY_WEBHOOK"),
            telegram_bot_token: common::config::optional("TELEGRAM_BOT_TOKEN"),
            telegram_chat_id: common::config::optional("TELEGRAM_CHAT_ID"),
            public_base_url: common::config::optional("PUBLIC_BASE_URL"),
        })
    }
}
