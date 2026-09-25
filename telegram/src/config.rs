//! Telegram-surface configuration.

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct TelegramConfig {
    /// Bot token from @BotFather (required).
    pub bot_token: String,
    /// Same database as the other services (optional; commands degrade without it).
    pub database_url: Option<String>,
    /// Telegram user ids always treated as Admin (bootstrap), comma-separated.
    pub admin_ids: Vec<u64>,
    /// Public base URL of the API, used to link download files.
    pub public_base_url: Option<String>,
}

impl TelegramConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bot_token: common::config::require("TELEGRAM_BOT_TOKEN")?,
            database_url: common::config::optional("DATABASE_URL"),
            admin_ids: common::config::optional("TELEGRAM_ADMIN_IDS")
                .map(|s| {
                    s.split(',')
                        .filter_map(|p| p.trim().parse::<u64>().ok())
                        .collect()
                })
                .unwrap_or_default(),
            public_base_url: common::config::optional("PUBLIC_BASE_URL"),
        })
    }
}
