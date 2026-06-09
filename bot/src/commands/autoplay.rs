//! `/autoplay` — when a user joins a configured voice channel, the bot joins and
//! plays a configured song. Triggers are stored per (guild, channel) in the
//! `voice_triggers` table and fired from the gateway's voice-state-update event.

use serenity::all::{
    ChannelId, ChannelType, CommandInteraction, CommandOptionType, Context,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    GuildId,
};
use tracing::warn;

pub fn definition() -> CreateCommand {
    let voice_channel = || {
        CreateCommandOption::new(CommandOptionType::Channel, "channel", "Voice channel")
            .required(true)
            .channel_types(vec![ChannelType::Voice])
    };

    CreateCommand::new("autoplay")
        .description("Auto-play a song when someone joins a voice channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "set",
                "Set a channel's auto-play song",
            )
            .add_sub_option(voice_channel())
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "song",
                    "File from your library or a URL",
                )
                .required(true)
                .set_autocomplete(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "clear",
                "Remove a channel's auto-play",
            )
            .add_sub_option(voice_channel()),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List configured auto-play channels",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match super::subcommand_name(command) {
        "set" => set(ctx, command).await,
        "clear" => clear(ctx, command).await,
        "list" => list(ctx, command).await,
        other => {
            super::respond_ephemeral(ctx, command, format!("Unknown autoplay action: `{other}`"))
                .await
        }
    }
}

async fn set(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let Some(pool) = &state.db else {
        return super::respond_ephemeral(ctx, command, "Database not available.").await;
    };
    let (Some(channel), Some(song)) = (
        super::sub_option_channel(command, "channel"),
        super::sub_option_str(command, "song"),
    ) else {
        return super::respond_ephemeral(ctx, command, "Missing channel or song.").await;
    };

    let title = song_title(song);
    sqlx::query(
        "INSERT INTO voice_triggers (guild_id, channel_id, source, title) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (guild_id, channel_id) DO UPDATE SET source = EXCLUDED.source, title = EXCLUDED.title",
    )
    .bind(guild_id.get() as i64)
    .bind(channel.get() as i64)
    .bind(song)
    .bind(&title)
    .execute(pool)
    .await?;

    super::respond(
        ctx,
        command,
        format!(
            "✅ When someone joins <#{}>, I'll play **{}**.",
            channel.get(),
            title
        ),
    )
    .await
}

async fn clear(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let Some(pool) = &state.db else {
        return super::respond_ephemeral(ctx, command, "Database not available.").await;
    };
    let Some(channel) = super::sub_option_channel(command, "channel") else {
        return super::respond_ephemeral(ctx, command, "Missing channel.").await;
    };

    let result = sqlx::query("DELETE FROM voice_triggers WHERE guild_id = $1 AND channel_id = $2")
        .bind(guild_id.get() as i64)
        .bind(channel.get() as i64)
        .execute(pool)
        .await?;

    let msg = if result.rows_affected() > 0 {
        format!("🗑️ Removed auto-play for <#{}>.", channel.get())
    } else {
        format!("No auto-play was set for <#{}>.", channel.get())
    };
    super::respond(ctx, command, msg).await
}

async fn list(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let Some(pool) = &state.db else {
        return super::respond_ephemeral(ctx, command, "Database not available.").await;
    };

    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT channel_id, title FROM voice_triggers WHERE guild_id = $1 ORDER BY created_at",
    )
    .bind(guild_id.get() as i64)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return super::respond(
            ctx,
            command,
            "No auto-play channels configured yet. Use `/autoplay set`.",
        )
        .await;
    }

    let mut text = String::from("**Auto-play channels:**\n");
    for (channel_id, title) in rows {
        text.push_str(&format!(
            "• <#{}> → {}\n",
            channel_id,
            title.unwrap_or_else(|| "(unknown)".to_string())
        ));
    }
    super::respond(ctx, command, text).await
}

/// Autocomplete for the `song` option — suggest files from the local library.
pub async fn handle_autocomplete(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<()> {
    let state = super::state(ctx).await;
    let partial = command
        .data
        .autocomplete()
        .map(|o| o.value.to_string())
        .unwrap_or_default();
    let music_path = state.music_path.clone();

    let files = tokio::task::spawn_blocking(move || {
        super::music::list_music_files(&music_path, &partial, 25)
    })
    .await
    .unwrap_or_default();

    let mut resp = CreateAutocompleteResponse::new();
    for f in files {
        resp = resp.add_string_choice(f.clone(), f);
    }
    command
        .create_response(&ctx.http, CreateInteractionResponse::Autocomplete(resp))
        .await?;
    Ok(())
}

/// Called from the voice-state-update handler when a user joins `channel_id`.
/// Plays the configured trigger if one exists and the bot isn't already busy.
pub async fn maybe_autoplay(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) {
    let state = super::state(ctx).await;
    let Some(pool) = &state.db else {
        return;
    };

    let row: Option<(String,)> = match sqlx::query_as(
        "SELECT source FROM voice_triggers WHERE guild_id = $1 AND channel_id = $2",
    )
    .bind(guild_id.get() as i64)
    .bind(channel_id.get() as i64)
    .fetch_optional(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(?e, "autoplay lookup failed");
            return;
        }
    };
    let Some((source,)) = row else {
        return;
    };

    // Don't interrupt if the bot is already connected in this guild.
    if let Some(manager) = songbird::get(ctx).await {
        if manager.get(guild_id).is_some() {
            return;
        }
    }

    if let Err(e) = super::music::play_source_in_channel(ctx, guild_id, channel_id, &source).await {
        warn!(error = %e, "autoplay failed to start");
    }
}

fn song_title(song: &str) -> String {
    if song.starts_with("http://") || song.starts_with("https://") {
        song.to_string()
    } else {
        std::path::Path::new(song)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(song)
            .to_string()
    }
}
