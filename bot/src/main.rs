//! NigerianBot — Discord bot service entry point.
//!
//! Phase 1 establishes the skeleton: connect to Discord, register the slash
//! command definitions, and route incoming interactions to per-feature command
//! modules. Most handlers currently return a "not implemented yet" placeholder
//! that names the phase where they get fleshed out.

mod commands;
mod config;
mod handlers;
mod services;

use anyhow::Context as _;
use serenity::all::{
    Client, Command, Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready,
};
use serenity::async_trait;
use tracing::{error, info};

use crate::config::BotConfig;

struct Handler {
    config: BotConfig,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(user = %ready.user.name, "connected to Discord");

        let definitions = commands::all_definitions();

        let result = match self.config.guild_id {
            Some(id) => GuildId::new(id)
                .set_commands(&ctx.http, definitions)
                .await
                .map(|cmds| info!(guild = id, count = cmds.len(), "registered guild commands")),
            None => Command::set_global_commands(&ctx.http, definitions)
                .await
                .map(|cmds| info!(count = cmds.len(), "registered global commands")),
        };

        if let Err(e) = result {
            error!(?e, "failed to register slash commands");
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("bot");

    let config = BotConfig::from_env().context("invalid bot configuration")?;
    let token = config.discord_token.clone();

    // GUILD_VOICE_STATES is needed for music (Phase 6); the rest cover commands
    // and basic guild/message events.
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { config })
        .await
        .context("failed to build Discord client")?;

    info!("starting NigerianBot…");
    client.start().await.context("Discord client error")?;

    Ok(())
}
