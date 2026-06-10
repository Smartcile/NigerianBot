//! Slash command registry, dispatcher, and shared response helpers.
//!
//! Each feature area lives in its own module and exposes:
//!   * `definition()` — the [`CreateCommand`] registered with Discord
//!   * `handle()`     — the async handler invoked when the command is used

pub mod autoplay;
pub mod bot;
pub mod joinsound;
pub mod music;
pub mod radarr;
pub mod schedule;
pub mod server;
pub mod sonarr;

use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, ComponentInteraction, Context,
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use tracing::{error, warn};

use crate::state::{BotState, BotStateKey};

/// Every slash command definition registered with Discord on startup.
pub fn all_definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("ping").description("Health check — replies with Pong!"),
        music::definition(),
        sonarr::definition(),
        radarr::definition(),
        server::definition(),
        bot::definition(),
        autoplay::definition(),
        joinsound::definition(),
        schedule::definition(),
    ]
}

/// Route an incoming command interaction to its handler, and on failure send the
/// user an ephemeral error (best effort) so they aren't left with a silent
/// "interaction failed".
pub async fn dispatch(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    // Best-effort audit logging — never let a DB hiccup block the command.
    let bot_state = state(ctx).await;
    if let Some(pool) = &bot_state.db {
        if let Err(e) = crate::audit::record(pool, command).await {
            warn!(?e, "failed to write audit log entry");
        }
    }

    let result = route(ctx, command).await;
    if let Err(ref e) = result {
        error!(?e, command = %command.data.name, "command handler error");
        // If the handler already replied this is a no-op; ignore the secondary error.
        let _ = respond_ephemeral(
            ctx,
            command,
            "⚠️ Something went wrong while handling that command. Please try again.",
        )
        .await;
    }
    result
}

async fn route(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match command.data.name.as_str() {
        "ping" => respond(ctx, command, "🏓 Pong!").await,
        "music" => music::handle(ctx, command).await,
        "sonarr" => sonarr::handle(ctx, command).await,
        "radarr" => radarr::handle(ctx, command).await,
        "server" => server::handle(ctx, command).await,
        "bot" => bot::handle(ctx, command).await,
        "autoplay" => autoplay::handle(ctx, command).await,
        "joinsound" => joinsound::handle(ctx, command).await,
        "schedule" => schedule::handle(ctx, command).await,
        other => respond_ephemeral(ctx, command, format!("Unknown command: `{other}`")).await,
    }
}

/// Route an autocomplete interaction to the command that owns the focused option.
pub async fn dispatch_autocomplete(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<()> {
    match command.data.name.as_str() {
        "music" => music::handle_autocomplete(ctx, command).await,
        "autoplay" => autoplay::handle_autocomplete(ctx, command).await,
        "joinsound" => joinsound::handle_autocomplete(ctx, command).await,
        _ => Ok(()),
    }
}

/// Route a message-component (button) interaction by its custom id prefix.
pub async fn dispatch_component(
    ctx: &Context,
    component: &ComponentInteraction,
) -> anyhow::Result<()> {
    let id = &component.data.custom_id;
    if id.starts_with("musicadd|") {
        music::handle_add_button(ctx, component).await
    } else if id.starts_with("music_") {
        music::handle_component(ctx, component).await
    } else {
        Ok(())
    }
}

// ── Response helpers ───────────────────────────────────────────────────────

/// Send a plain text reply.
pub async fn respond(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new().content(content.into());
    command
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

/// Send an ephemeral text reply (only the invoking user sees it).
pub async fn respond_ephemeral(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .content(content.into())
        .ephemeral(true);
    command
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

/// Edit a previously-deferred response with text (for slow commands that called
/// `command.defer(...)` first).
pub async fn respond_edit(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> anyhow::Result<()> {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(content.into()),
        )
        .await?;
    Ok(())
}

/// Send an embed reply.
pub async fn respond_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new().embed(embed);
    command
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

// ── Shared helpers ─────────────────────────────────────────────────────────

/// Name of the invoked subcommand, if any (e.g. `status` for `/bot status`).
pub fn subcommand_name(command: &CommandInteraction) -> &str {
    command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("")
}

/// Options of the invoked subcommand (the nested options under e.g. `/music play`).
fn subcommand_options(command: &CommandInteraction) -> &[serenity::all::CommandDataOption] {
    match command.data.options.first() {
        Some(o) => match &o.value {
            CommandDataOptionValue::SubCommand(opts)
            | CommandDataOptionValue::SubCommandGroup(opts) => opts,
            _ => &[],
        },
        None => &[],
    }
}

/// Read a string option from the invoked subcommand.
pub fn sub_option_str<'a>(command: &'a CommandInteraction, name: &str) -> Option<&'a str> {
    subcommand_options(command)
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| o.value.as_str())
}

/// Read an integer option from the invoked subcommand.
pub fn sub_option_i64(command: &CommandInteraction, name: &str) -> Option<i64> {
    subcommand_options(command)
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| o.value.as_i64())
}

/// Read a channel option from the invoked subcommand.
pub fn sub_option_channel(command: &CommandInteraction, name: &str) -> Option<ChannelId> {
    subcommand_options(command)
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Channel(id) => Some(*id),
            _ => None,
        })
}

/// Fetch the shared [`BotState`] from the client's type map.
pub async fn state(ctx: &Context) -> Arc<BotState> {
    ctx.data
        .read()
        .await
        .get::<BotStateKey>()
        .cloned()
        .expect("BotState is inserted at startup")
}

/// Human-friendly uptime like `2d 3h 7m 12s`.
pub fn format_uptime(d: Duration) -> String {
    let s = d.as_secs();
    let (days, hours, mins, secs) = (s / 86_400, (s % 86_400) / 3_600, (s % 3_600) / 60, s % 60);
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if mins > 0 {
        parts.push(format!("{mins}m"));
    }
    parts.push(format!("{secs}s"));
    parts.join(" ")
}
