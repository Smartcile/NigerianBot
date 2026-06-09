//! NigerianBot — Discord bot service entry point.
//!
//! Phase 1 establishes the skeleton: connect to Discord, register the slash
//! command definitions, and route incoming interactions to per-feature command
//! modules. Most handlers currently return a "not implemented yet" placeholder
//! that names the phase where they get fleshed out.

mod audit;
mod commands;
mod config;
mod handlers;
mod services;
mod state;

use anyhow::Context as _;
use serenity::all::{
    Client, Command, Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready,
};
use serenity::async_trait;
use songbird::SerenityInit;
use tracing::{error, info, warn};

use crate::config::BotConfig;

struct Handler {
    config: BotConfig,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(user = %ready.user.name, "connected to Discord");

        let definitions = commands::all_definitions();

        match self.config.guild_id {
            Some(id) => {
                // Guild-scoped registration overwrites the whole command set for
                // this guild (instant propagation).
                match GuildId::new(id).set_commands(&ctx.http, definitions).await {
                    Ok(cmds) => info!(guild = id, count = cmds.len(), "registered guild commands"),
                    Err(e) => error!(?e, "failed to register guild commands"),
                }

                // Also clear any GLOBAL commands left over from this app's
                // previous life, so only the guild-scoped set above shows up in
                // the slash-command picker (no stale duplicates).
                match Command::set_global_commands(&ctx.http, Vec::new()).await {
                    Ok(_) => info!("cleared stale global commands"),
                    Err(e) => error!(?e, "failed to clear global commands"),
                }
            }
            None => {
                // No guild configured: register globally (overwrites all globals).
                match Command::set_global_commands(&ctx.http, definitions).await {
                    Ok(cmds) => info!(count = cmds.len(), "registered global commands"),
                    Err(e) => error!(?e, "failed to register global commands"),
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if let Err(e) = commands::dispatch(&ctx, &command).await {
                error!(?e, command = %command.data.name, "command handler failed");
            }
        }
    }
}

/// Connect to Postgres, retrying briefly so the bot tolerates the database
/// still warming up (e.g. on first stack startup).
async fn connect_db_with_retry(url: &str) -> anyhow::Result<sqlx::PgPool> {
    const MAX_ATTEMPTS: u32 = 10;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match common::db::connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(attempt, error = %e, "database not ready, retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e.context("could not connect to the database")),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("bot");

    let config = BotConfig::from_env().context("invalid bot configuration")?;
    let token = config.discord_token.clone();

    // Connect to Postgres and apply migrations when a database is configured.
    let db = match config.database_url.clone() {
        Some(url) => {
            let pool = connect_db_with_retry(&url).await?;
            sqlx::migrate!("../migrations")
                .run(&pool)
                .await
                .context("failed to run database migrations")?;
            info!("database connected and migrations applied");
            Some(pool)
        }
        None => {
            info!("no DATABASE_URL set — running without persistence");
            None
        }
    };

    // Music: HTTP client for URL/yt-dlp sources and the mounted music directory.
    let http = reqwest::Client::new();
    let music_path = common::config::optional_or("MUSIC_MOUNT_PATH", "/music");

    // GUILD_VOICE_STATES feeds the cache so we can find a user's voice channel.
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { config })
        .type_map_insert::<state::BotStateKey>(state::BotState::new(db, http, music_path))
        .register_songbird()
        .await
        .context("failed to build Discord client")?;

    info!("starting NigerianBot…");
    client.start().await.context("Discord client error")?;

    Ok(())
}
