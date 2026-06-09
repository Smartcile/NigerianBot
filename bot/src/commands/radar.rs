//! `/radar` — service status & metrics. Wired up in Phase 5.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

fn service_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::String, "service", "Service name").required(true)
}

pub fn definition() -> CreateCommand {
    CreateCommand::new("radar")
        .description("Query Radar monitoring data")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "Check service status",
            )
            .add_sub_option(service_option()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "metrics",
                "Get service metrics",
            )
            .add_sub_option(service_option()),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "alerts",
            "List active alerts",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let sub = super::subcommand_name(command);
    super::respond(
        ctx,
        command,
        format!("📡 `/radar {sub}` is not implemented yet (Phase 5: Radar integration)."),
    )
    .await
}
