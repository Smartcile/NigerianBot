//! `/download` — queue a video URL (Vimeo / YouTube / any yt-dlp-supported site)
//! for the worker service to fetch onto the server.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

pub fn definition() -> CreateCommand {
    CreateCommand::new("download")
        .description("Queue a video URL (Vimeo/YouTube) to download on the server")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "url", "The video URL")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let url = command
        .data
        .options
        .iter()
        .find(|o| o.name == "url")
        .and_then(|o| o.value.as_str())
        .map(str::trim)
        .unwrap_or("");

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return super::respond_ephemeral(ctx, command, "Provide an http(s) URL.").await;
    }

    let state = super::state(ctx).await;
    let Some(db) = &state.db else {
        return super::respond_ephemeral(ctx, command, "Database not available.").await;
    };

    let provider = common::media::provider_of(url);
    let requested_by = format!("discord:{}", command.user.id.get());
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO downloads (url, provider, requested_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(url)
    .bind(provider)
    .bind(&requested_by)
    .fetch_one(db)
    .await?;

    super::respond(
        ctx,
        command,
        format!("📥 Queued download **#{id}** ({provider}). It'll appear in the dashboard when it's done."),
    )
    .await
}
