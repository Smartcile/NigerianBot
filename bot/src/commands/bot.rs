//! `/bot` — bot control & diagnostics.
//!
//! `/bot status` is live. The control actions (`logs`, `config`, `restart`)
//! require the API/control-plane and role checks and arrive in Phase 4.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
};

pub fn definition() -> CreateCommand {
    CreateCommand::new("bot")
        .description("Bot control and diagnostics")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "status",
            "Bot health status",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "logs", "View service logs")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "service", "Service name")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "config", "Update a setting")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "key", "Setting key")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "value", "Setting value")
                        .required(true),
                ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "restart",
            "Restart services (admin only)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match super::subcommand_name(command) {
        "status" => status(ctx, command).await,
        other => {
            super::respond_ephemeral(
                ctx,
                command,
                format!("🤖 `/bot {other}` needs the control plane and will arrive in Phase 4."),
            )
            .await
        }
    }
}

/// `/bot status` — live health snapshot of the bot process.
async fn status(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let state = super::state(ctx).await;
    let uptime = super::format_uptime(state.started_at.elapsed());

    // Report database health and how many commands have been audit-logged.
    let (db_status, logged) = match &state.db {
        Some(pool) => match crate::audit::count(pool).await {
            Ok(n) => ("🟢 Connected", n.to_string()),
            Err(_) => ("🟠 Error", "—".to_string()),
        },
        None => ("⚪ Not configured", "—".to_string()),
    };

    let embed = CreateEmbed::new()
        .title("🤖 NigerianBot status")
        .colour(0x57F287_u32)
        .field("Status", "🟢 Online", true)
        .field("Uptime", uptime, true)
        .field("Version", env!("CARGO_PKG_VERSION"), true)
        .field("Database", db_status, true)
        .field("Commands logged", logged, true)
        .field("Voice bots", state.pool.len().to_string(), true)
        .field(
            "Commands available",
            super::all_definitions().len().to_string(),
            true,
        );

    super::respond_embed(ctx, command, embed).await
}
