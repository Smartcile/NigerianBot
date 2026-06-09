//! `/music` — playback controls. Wired up in Phase 6 (songbird).

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

pub fn definition() -> CreateCommand {
    CreateCommand::new("music")
        .description("Music playback controls")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "play",
                "Play music from a URL or mounted path",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "url",
                    "Track URL or file path",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "pause",
            "Pause playback",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "stop",
            "Stop playback and clear the player",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "queue",
            "Show the current queue",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "volume",
                "Set playback volume",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "level", "Volume 0-100")
                    .required(true)
                    .min_int_value(0)
                    .max_int_value(100),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let sub = super::subcommand_name(command);
    super::respond(
        ctx,
        command,
        format!("🎵 `/music {sub}` is not implemented yet (Phase 6: music playback)."),
    )
    .await
}
