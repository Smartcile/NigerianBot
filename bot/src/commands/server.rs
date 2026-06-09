//! `/server` — guild management. Wired up in a later phase.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

fn action_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::String, "action", "Action to perform")
        .required(true)
}

pub fn definition() -> CreateCommand {
    CreateCommand::new("server")
        .description("Server management")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "members",
                "Add/remove members",
            )
            .add_sub_option(action_option()),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "roles", "Manage roles")
                .add_sub_option(action_option()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "channels",
                "Create/delete channels",
            )
            .add_sub_option(action_option()),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "info",
            "Show server information",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let sub = super::subcommand_name(command);
    super::respond(
        ctx,
        command,
        format!("🛠️ `/server {sub}` is not implemented yet (server management phase)."),
    )
    .await
}
