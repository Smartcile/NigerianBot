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
mod identity;
mod scheduler;
mod services;
mod state;

use anyhow::Context as _;
use serenity::all::{
    Client, Command, Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready, VoiceState,
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
        match interaction {
            Interaction::Command(command) => {
                if let Err(e) = commands::dispatch(&ctx, &command).await {
                    error!(?e, command = %command.data.name, "command handler failed");
                }
            }
            Interaction::Autocomplete(command) => {
                if let Err(e) = commands::dispatch_autocomplete(&ctx, &command).await {
                    error!(?e, "autocomplete handler failed");
                }
            }
            Interaction::Component(component) => {
                if let Err(e) = commands::dispatch_component(&ctx, &component).await {
                    error!(?e, "component handler failed");
                }
            }
            _ => {}
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let (Some(guild_id), Some(channel_id)) = (new.guild_id, new.channel_id) else {
            return;
        };
        // Ignore the bot's own voice updates.
        if new.user_id == ctx.cache.current_user().id {
            return;
        }
        // Only react to an actual join/move into the channel (not mute/deafen).
        if old.and_then(|o| o.channel_id) == Some(channel_id) {
            return;
        }
        // One-shot join sounds take priority; otherwise fall through to autoplay.
        if commands::joinsound::maybe_play_joinsound(&ctx, guild_id, channel_id).await {
            return;
        }
        commands::autoplay::maybe_autoplay(&ctx, guild_id, channel_id).await;
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

    // Sonarr / Radarr clients, when configured.
    let sonarr = match (config.sonarr_url.clone(), config.sonarr_api_key.clone()) {
        (Some(url), Some(key)) => Some(services::arr::Arr::new(http.clone(), url, key)),
        _ => None,
    };
    let radarr = match (config.radarr_url.clone(), config.radarr_api_key.clone()) {
        (Some(url), Some(key)) => Some(services::arr::Arr::new(http.clone(), url, key)),
        _ => None,
    };
    info!(
        sonarr = sonarr.is_some(),
        radarr = radarr.is_some(),
        "media integrations"
    );

    // Build the voice pool: one Songbird manager per bot (primary + extras).
    let primary_songbird = songbird::Songbird::serenity();
    let mut pool_bots = vec![state::VoiceBot {
        songbird: primary_songbird.clone(),
        label: "primary".to_string(),
    }];
    let mut worker_songbirds = Vec::new();
    for i in 0..config.pool_tokens.len() {
        let sb = songbird::Songbird::serenity();
        worker_songbirds.push(sb.clone());
        pool_bots.push(state::VoiceBot {
            songbird: sb,
            label: format!("bot-{}", i + 2),
        });
    }
    let pool = state::VoicePool { bots: pool_bots };
    let pool_size = pool.len();
    let admin_ids = config.admin_ids.clone();
    let bot_state = state::BotState::new(db, http, music_path, pool, sonarr, radarr, admin_ids);
    let scheduler_state = bot_state.clone();

    // GUILD_VOICE_STATES feeds the cache so we can find a user's voice channel.
    let primary_intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;
    // Worker bots only need voice; they don't handle commands or messages.
    let worker_intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES;

    // Start the worker bots (each its own gateway connection for voice).
    for (i, worker_token) in config.pool_tokens.iter().enumerate() {
        let songbird = worker_songbirds[i].clone();
        let mut worker = Client::builder(worker_token, worker_intents)
            .register_songbird_with(songbird)
            .await
            .context("failed to build worker bot client")?;
        let label = format!("bot-{}", i + 2);
        tokio::spawn(async move {
            info!(bot = %label, "starting voice-pool worker");
            if let Err(e) = worker.start().await {
                error!(bot = %label, ?e, "voice-pool worker error");
            }
        });
    }

    let mut client = Client::builder(&token, primary_intents)
        .event_handler(Handler { config })
        .type_map_insert::<state::BotStateKey>(bot_state)
        .register_songbird_with(primary_songbird)
        .await
        .context("failed to build Discord client")?;

    // Background scheduler (recurring messages, reminders, media digests, pruning).
    scheduler::spawn(client.http.clone(), scheduler_state);

    info!(voice_bots = pool_size, "starting NigerianBot…");
    client.start().await.context("Discord client error")?;

    Ok(())
}
