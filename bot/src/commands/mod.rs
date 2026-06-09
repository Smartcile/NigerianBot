//! Slash command registry and dispatcher.
//!
//! Each feature area lives in its own module and exposes:
//!   * `definition()` — the [`CreateCommand`] registered with Discord
//!   * `handle()`     — the async handler invoked when the command is used

pub mod bot;
pub mod music;
pub mod radar;
pub mod server;
pub mod sonar;

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

/// Every slash command definition registered with Discord on startup.
pub fn all_definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("ping").description("Health check — replies with Pong!"),
        music::definition(),
        sonar::definition(),
        radar::definition(),
        server::definition(),
        bot::definition(),
    ]
}

/// Route an incoming command interaction to the appropriate handler.
pub async fn dispatch(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match command.data.name.as_str() {
        "ping" => respond(ctx, command, "🏓 Pong!").await,
        "music" => music::handle(ctx, command).await,
        "sonar" => sonar::handle(ctx, command).await,
        "radar" => radar::handle(ctx, command).await,
        "server" => server::handle(ctx, command).await,
        "bot" => bot::handle(ctx, command).await,
        other => respond(ctx, command, &format!("Unknown command: `{other}`")).await,
    }
}

/// Send a simple ephemeral-free text reply for a command interaction.
pub async fn respond(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> anyhow::Result<()> {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(content.into()),
    );
    command.create_response(&ctx.http, response).await?;
    Ok(())
}

/// Name of the invoked subcommand, if any (e.g. `play` for `/music play`).
pub fn subcommand_name(command: &CommandInteraction) -> &str {
    command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("")
}
