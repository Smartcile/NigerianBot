//! `/bot` — bot control & diagnostics. Wired up alongside the API (Phase 4+).

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
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
    let sub = super::subcommand_name(command);
    super::respond(
        ctx,
        command,
        format!("🤖 `/bot {sub}` is not implemented yet (Phase 4: API & control plane)."),
    )
    .await
}
