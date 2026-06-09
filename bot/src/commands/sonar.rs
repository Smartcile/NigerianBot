//! `/sonar` — SonarQube project metrics. Wired up in Phase 5.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

fn project_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::String, "project", "Project key").required(true)
}

pub fn definition() -> CreateCommand {
    CreateCommand::new("sonar")
        .description("Query SonarQube project data")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "query",
                "Query project metrics",
            )
            .add_sub_option(project_option()),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "issues", "List code issues")
                .add_sub_option(project_option()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "quality",
                "Get quality gate status",
            )
            .add_sub_option(project_option()),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let sub = super::subcommand_name(command);
    super::respond(
        ctx,
        command,
        format!("📊 `/sonar {sub}` is not implemented yet (Phase 5: Sonar integration)."),
    )
    .await
}
