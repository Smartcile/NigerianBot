//! Bot-service configuration, sourced from the environment.

use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
pub struct BotConfig {
    /// Discord bot token (required).
    pub discord_token: String,
    /// When set, slash commands register instantly to this single guild.
    /// When unset, commands register globally (can take up to ~1 hour).
    pub guild_id: Option<u64>,
    /// Optional database URL — wired up for persistence in later phases.
    #[allow(dead_code)] // reserved for Phase 3 (persistence)
    pub database_url: Option<String>,
    /// Extra bot tokens (`DISCORD_TOKEN_2`, `DISCORD_TOKEN_3`, …) for the voice
    /// pool — each is a separate Discord bot that can hold its own voice
    /// connection, enabling simultaneous playback across channels.
    pub pool_tokens: Vec<String>,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let guild_id = match common::config::optional("DISCORD_GUILD_ID") {
            Some(raw) => Some(
                raw.parse::<u64>()
                    .map_err(|e| anyhow!("DISCORD_GUILD_ID must be a numeric id: {e}"))?,
            ),
            None => None,
        };

        // Collect DISCORD_TOKEN_2 .. DISCORD_TOKEN_9 as extra voice-pool bots.
        let mut pool_tokens = Vec::new();
        for i in 2..=9 {
            if let Some(t) = common::config::optional(&format!("DISCORD_TOKEN_{i}")) {
                pool_tokens.push(t);
            }
        }

        Ok(Self {
            discord_token: common::config::require("DISCORD_TOKEN")?,
            guild_id,
            database_url: common::config::optional("DATABASE_URL"),
            pool_tokens,
        })
    }
}
