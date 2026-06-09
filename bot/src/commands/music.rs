//! `/music` — voice playback via songbird.
//!
//! Sources: a URL/YouTube link (streamed with yt-dlp), or a local file under the
//! mounted music directory (`MUSIC_MOUNT_PATH`). Uses songbird's built-in track
//! queue for play/pause/stop/queue/volume.

use anyhow::anyhow;
use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, EditInteractionResponse, GuildId, UserId,
};
use songbird::input::{ChildContainer, Compose, Input, RawAdapter, YoutubeDl};
use tracing::warn;

pub fn definition() -> CreateCommand {
    CreateCommand::new("music")
        .description("Music playback controls")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "play",
                "Play a URL/YouTube link or a file from the music folder",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "url", "Link or file name")
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
            "Stop playback and leave the channel",
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
    match super::subcommand_name(command) {
        "play" => play(ctx, command).await,
        "pause" => pause(ctx, command).await,
        "stop" => stop(ctx, command).await,
        "queue" => queue(ctx, command).await,
        "volume" => volume(ctx, command).await,
        other => {
            super::respond_ephemeral(ctx, command, format!("Unknown music action: `{other}`")).await
        }
    }
}

// ── play ───────────────────────────────────────────────────────────────────

async fn play(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let arg = sub_option_str(command, "url")
        .unwrap_or_default()
        .to_string();

    // Joining voice + resolving metadata can exceed Discord's 3s window, so defer.
    command.defer(&ctx.http).await?;

    let text = match play_inner(ctx, command, &arg).await {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "music play failed");
            format!("⚠️ {e}")
        }
    };

    command
        .edit_response(&ctx.http, EditInteractionResponse::new().content(text))
        .await?;
    Ok(())
}

async fn play_inner(
    ctx: &Context,
    command: &CommandInteraction,
    arg: &str,
) -> anyhow::Result<String> {
    if arg.is_empty() {
        return Err(anyhow!("Provide a link or file name."));
    }

    let guild_id = command
        .guild_id
        .ok_or_else(|| anyhow!("Use this in a server."))?;
    let channel_id = user_voice_channel(ctx, guild_id, command.user.id)
        .ok_or_else(|| anyhow!("Join a voice channel first."))?;

    let manager = songbird::get(ctx)
        .await
        .ok_or_else(|| anyhow!("voice subsystem not initialized"))?
        .clone();
    let call_lock = manager
        .join(guild_id, channel_id)
        .await
        .map_err(|e| anyhow!("couldn't join the voice channel: {e}"))?;

    let state = super::state(ctx).await;

    // Build the audio source: URL -> yt-dlp; otherwise a local file.
    let (input, title): (Input, String) =
        if arg.starts_with("http://") || arg.starts_with("https://") {
            let mut src = YoutubeDl::new(state.http.clone(), arg.to_string());
            let title = src
                .aux_metadata()
                .await
                .ok()
                .and_then(|m| m.title)
                .unwrap_or_else(|| arg.to_string());
            (src.into(), title)
        } else {
            if arg.contains("..") {
                return Err(anyhow!("Invalid path."));
            }
            let path = std::path::Path::new(&state.music_path).join(arg.trim_start_matches('/'));
            if !path.is_file() {
                return Err(anyhow!("File not found: `{arg}`"));
            }
            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(arg)
                .to_string();
            (ffmpeg_input(&path)?, title)
        };

    let was_empty = {
        let call = call_lock.lock().await;
        call.queue().is_empty()
    };
    {
        let mut call = call_lock.lock().await;
        call.enqueue_input(input).await;
    }

    // Best-effort persistence to the music_queue table.
    if let Some(pool) = &state.db {
        let _ = sqlx::query(
            "INSERT INTO music_queue (guild_id, position, title, source, requested_by) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(guild_id.get() as i64)
        .bind(0_i32)
        .bind(&title)
        .bind(arg)
        .bind(command.user.id.get() as i64)
        .execute(pool)
        .await;
    }

    Ok(if was_empty {
        format!("🎵 Now playing: **{title}**")
    } else {
        format!("➕ Added to queue: **{title}**")
    })
}

// ── pause / stop / queue / volume ───────────────────────────────────────────

async fn pause(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let manager = match songbird::get(ctx).await {
        Some(m) => m.clone(),
        None => return super::respond_ephemeral(ctx, command, "Voice not available.").await,
    };

    let msg = if let Some(call_lock) = manager.get(guild_id) {
        let call = call_lock.lock().await;
        let _ = call.queue().pause();
        "⏸️ Paused."
    } else {
        "Nothing is playing."
    };
    super::respond(ctx, command, msg).await
}

async fn stop(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let manager = match songbird::get(ctx).await {
        Some(m) => m.clone(),
        None => return super::respond_ephemeral(ctx, command, "Voice not available.").await,
    };

    let msg = if let Some(call_lock) = manager.get(guild_id) {
        {
            let call = call_lock.lock().await;
            call.queue().stop();
        }
        let _ = manager.remove(guild_id).await;
        if let Some(pool) = &super::state(ctx).await.db {
            let _ = sqlx::query("DELETE FROM music_queue WHERE guild_id = $1")
                .bind(guild_id.get() as i64)
                .execute(pool)
                .await;
        }
        "⏹️ Stopped and left the channel."
    } else {
        "Nothing is playing."
    };
    super::respond(ctx, command, msg).await
}

async fn queue(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let manager = match songbird::get(ctx).await {
        Some(m) => m.clone(),
        None => return super::respond_ephemeral(ctx, command, "Voice not available.").await,
    };

    let msg = if let Some(call_lock) = manager.get(guild_id) {
        let call = call_lock.lock().await;
        let n = call.queue().current_queue().len();
        if n == 0 {
            "The queue is empty.".to_string()
        } else {
            format!("🎶 {n} track(s) in the queue.")
        }
    } else {
        "Nothing is playing.".to_string()
    };
    super::respond(ctx, command, msg).await
}

async fn volume(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let level = sub_option_i64(command, "level")
        .unwrap_or(100)
        .clamp(0, 100);
    let vol = level as f32 / 100.0;

    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let manager = match songbird::get(ctx).await {
        Some(m) => m.clone(),
        None => return super::respond_ephemeral(ctx, command, "Voice not available.").await,
    };

    let msg = if let Some(call_lock) = manager.get(guild_id) {
        let call = call_lock.lock().await;
        if let Some(track) = call.queue().current() {
            let _ = track.set_volume(vol);
        }
        format!("🔊 Volume set to {level}%.")
    } else {
        "Nothing is playing.".to_string()
    };
    super::respond(ctx, command, msg).await
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Locate the voice channel the invoking user is currently in (via the cache).
fn user_voice_channel(ctx: &Context, guild_id: GuildId, user_id: UserId) -> Option<ChannelId> {
    let guild = ctx.cache.guild(guild_id)?;
    guild
        .voice_states
        .get(&user_id)
        .and_then(|vs| vs.channel_id)
}

/// Decode a local file through ffmpeg into raw f32 PCM that songbird plays
/// directly. Using ffmpeg instead of songbird's built-in symphonia decoder makes
/// playback tolerant of any container/codec and of metadata tags (e.g. ID3v2)
/// that would otherwise abort decoding.
fn ffmpeg_input(path: &std::path::Path) -> anyhow::Result<Input> {
    use songbird::input::core::io::ReadOnlySource;
    use std::process::{Command, Stdio};

    let child = Command::new("ffmpeg")
        .arg("-i")
        .arg(path)
        // Raw interleaved 32-bit float PCM, stereo, 48 kHz (what songbird wants).
        .args(["-f", "f32le", "-ac", "2", "-ar", "48000", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to start ffmpeg: {e}"))?;

    let source = ReadOnlySource::new(ChildContainer::from(child));
    Ok(RawAdapter::new(source, 48000, 2).into())
}

fn sub_option_str<'a>(command: &'a CommandInteraction, name: &str) -> Option<&'a str> {
    let sub = command.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        opts.iter()
            .find(|o| o.name == name)
            .and_then(|o| o.value.as_str())
    } else {
        None
    }
}

fn sub_option_i64(command: &CommandInteraction, name: &str) -> Option<i64> {
    let sub = command.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        opts.iter()
            .find(|o| o.name == name)
            .and_then(|o| o.value.as_i64())
    } else {
        None
    }
}
