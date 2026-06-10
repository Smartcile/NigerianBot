//! `/music` — voice playback via songbird, across a pool of bots.
//!
//! Each playback request grabs a free bot from the pool (`BotState.pool`), so
//! multiple channels can have audio at once. Control commands/buttons act on the
//! specific bot that's playing (buttons encode the bot index in their custom id).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use serenity::all::{
    ButtonStyle, ChannelId, CommandInteraction, CommandOptionType, ComponentInteraction, Context,
    CreateActionRow, CreateAutocompleteResponse, CreateButton, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse, GuildId,
    UserId,
};
use songbird::input::{ChildContainer, Compose, Input, RawAdapter, YoutubeDl};
use songbird::tracks::PlayMode;
use songbird::{Event, EventContext, Songbird, TrackEvent};
use tracing::warn;

use crate::state::BotState;

const AUDIO_EXTS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "opus", "aac", "wma"];

pub fn definition() -> CreateCommand {
    CreateCommand::new("music")
        .description("Music playback controls")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "play",
                "Play a song from your library or a URL",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "song",
                    "Pick a file from your library, or paste a URL",
                )
                .required(true)
                .set_autocomplete(true),
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
    let arg = super::sub_option_str(command, "song")
        .unwrap_or_default()
        .to_string();

    // Joining voice + resolving metadata can exceed Discord's 3s window, so defer.
    command.defer(&ctx.http).await?;

    let edit = match resolve_and_play(ctx, command, &arg).await {
        Ok((title, was_empty, bot_index)) => {
            let text = if was_empty {
                format!("🎵 Now playing: **{title}**")
            } else {
                format!("➕ Added to queue: **{title}**")
            };
            EditInteractionResponse::new()
                .content(text)
                .components(vec![controls_row(bot_index)])
        }
        Err(e) => {
            warn!(error = %e, "music play failed");
            EditInteractionResponse::new().content(format!("⚠️ {e}"))
        }
    };

    command.edit_response(&ctx.http, edit).await?;
    Ok(())
}

async fn resolve_and_play(
    ctx: &Context,
    command: &CommandInteraction,
    arg: &str,
) -> anyhow::Result<(String, bool, usize)> {
    if arg.is_empty() {
        return Err(anyhow!("Provide a song name or link."));
    }
    let guild_id = command
        .guild_id
        .ok_or_else(|| anyhow!("Use this in a server."))?;
    let channel_id = user_voice_channel(ctx, guild_id, command.user.id)
        .ok_or_else(|| anyhow!("Join a voice channel first."))?;
    play_in_channel(ctx, guild_id, channel_id, arg).await
}

/// Play `arg` in `channel_id`. If a pool bot is already in that channel, queue on
/// it; otherwise grab a free bot and join (trying each until one connects).
/// Returns the title, whether it started playing immediately, and the bot index.
/// Used by `/music play` and `/autoplay`.
pub async fn play_in_channel(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    arg: &str,
) -> anyhow::Result<(String, bool, usize)> {
    let state = super::state(ctx).await;
    let (input, title) = build_source(&state, arg).await?;

    // A bot is already in this channel — queue on it (one bot per channel).
    if let Some((index, songbird)) = state.pool.find_in_channel(guild_id, channel_id).await {
        let call_lock = songbird
            .get(guild_id)
            .ok_or_else(|| anyhow!("the player just left"))?;
        let was_empty = {
            let mut call = call_lock.lock().await;
            let was_empty = call.queue().is_empty();
            call.enqueue_input(input).await;
            was_empty
        };
        persist_queue(&state, guild_id, &title, arg).await;
        return Ok((title, was_empty, index));
    }

    // Otherwise grab a free bot and join.
    let (index, songbird) = join_free_bot(&state, guild_id, channel_id).await?;
    let call_lock = songbird
        .get(guild_id)
        .ok_or_else(|| anyhow!("the player just left"))?;
    {
        let mut call = call_lock.lock().await;
        call.enqueue_input(input).await;
        call.add_global_event(
            Event::Periodic(Duration::from_secs(60), None),
            IdleLeaver {
                manager: songbird.clone(),
                guild_id,
                idle_minutes: Arc::new(AtomicU32::new(0)),
            },
        );
    }
    persist_queue(&state, guild_id, &title, arg).await;
    Ok((title, true, index))
}

/// Try each free pool bot in turn, cleaning up any that fail to join so a failed
/// attempt can't leave a bot permanently "busy". Returns the bot that joined.
async fn join_free_bot(
    state: &BotState,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> anyhow::Result<(usize, Arc<Songbird>)> {
    let free = state.pool.free_bots(guild_id).await;
    if free.is_empty() {
        return Err(anyhow!("All voice bots are busy right now."));
    }
    let mut last_err: Option<String> = None;
    for (index, songbird) in free {
        match songbird.join(guild_id, channel_id).await {
            Ok(_) => return Ok((index, songbird)),
            Err(e) => {
                // Clear the dead call so this bot is free again next time.
                let _ = songbird.remove(guild_id).await;
                warn!(bot = index, error = %e, "voice-pool bot failed to join; trying next");
                last_err = Some(e.to_string());
            }
        }
    }
    Err(anyhow!(
        "couldn't join the voice channel ({})",
        last_err.unwrap_or_else(|| "no bot could connect".to_string())
    ))
}

async fn persist_queue(state: &BotState, guild_id: GuildId, title: &str, source: &str) {
    if let Some(pool) = &state.db {
        let _ = sqlx::query(
            "INSERT INTO music_queue (guild_id, position, title, source, requested_by) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(guild_id.get() as i64)
        .bind(0_i32)
        .bind(title)
        .bind(source)
        .bind(Option::<i64>::None)
        .execute(pool)
        .await;
    }
}

/// Pick a free pool bot, join, play `source` once, and leave when it ends.
pub async fn play_once_on_free_bot(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    source: &str,
) -> anyhow::Result<()> {
    let state = super::state(ctx).await;
    let (input, _title) = build_source(&state, source).await?;

    let (_index, songbird) = join_free_bot(&state, guild_id, channel_id).await?;
    let call_lock = songbird
        .get(guild_id)
        .ok_or_else(|| anyhow!("the player just left"))?;

    let handle = {
        let mut call = call_lock.lock().await;
        call.enqueue_input(input).await
    };
    let _ = handle.add_event(
        Event::Track(TrackEvent::End),
        LeaveOnEnd {
            manager: songbird.clone(),
            guild_id,
        },
    );
    Ok(())
}

async fn build_source(state: &BotState, arg: &str) -> anyhow::Result<(Input, String)> {
    if arg.starts_with("http://") || arg.starts_with("https://") {
        let mut src = YoutubeDl::new(state.http.clone(), arg.to_string());
        let title = src
            .aux_metadata()
            .await
            .ok()
            .and_then(|m| m.title)
            .unwrap_or_else(|| arg.to_string());
        Ok((src.into(), title))
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
        Ok((ffmpeg_input(&path)?, title))
    }
}

/// Decode a local file through ffmpeg into raw f32 PCM — tolerant of any
/// container/codec and of metadata tags (ID3v2) that would otherwise abort
/// songbird's built-in decoder.
fn ffmpeg_input(path: &std::path::Path) -> anyhow::Result<Input> {
    use songbird::input::core::io::ReadOnlySource;
    use std::process::{Command, Stdio};

    let child = Command::new("ffmpeg")
        .arg("-i")
        .arg(path)
        .args(["-f", "f32le", "-ac", "2", "-ar", "48000", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to start ffmpeg: {e}"))?;

    let source = ReadOnlySource::new(ChildContainer::from(child));
    Ok(RawAdapter::new(source, 48000, 2).into())
}

// ── controls (buttons) ───────────────────────────────────────────────────────

fn controls_row(bot_index: usize) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("music_playpause:{bot_index}"))
            .label("⏯ Pause/Resume")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("music_skip:{bot_index}"))
            .label("⏭ Skip")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("music_stop:{bot_index}"))
            .label("⏹ Stop")
            .style(ButtonStyle::Danger),
    ])
}

/// Parse a control custom id like `music_skip:2` into `("music_skip", 2)`.
fn parse_control(custom_id: &str) -> Option<(&str, usize)> {
    let (action, index) = custom_id.rsplit_once(':')?;
    Some((action, index.parse().ok()?))
}

pub async fn handle_component(
    ctx: &Context,
    component: &ComponentInteraction,
) -> anyhow::Result<()> {
    let Some((action, index)) = parse_control(&component.data.custom_id) else {
        return ack(ctx, component, "Unknown control.").await;
    };
    let Some(guild_id) = component.guild_id else {
        return ack(ctx, component, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let Some(songbird) = state.pool.get(index) else {
        return ack(ctx, component, "That player is no longer available.").await;
    };
    let Some(call_lock) = songbird.get(guild_id) else {
        return ack(ctx, component, "Nothing is playing.").await;
    };

    let msg = match action {
        "music_skip" => {
            let call = call_lock.lock().await;
            let _ = call.queue().skip();
            "⏭️ Skipped."
        }
        "music_stop" => {
            {
                let call = call_lock.lock().await;
                call.queue().stop();
            }
            let _ = songbird.remove(guild_id).await;
            "⏹️ Stopped."
        }
        "music_playpause" => {
            let current = {
                let call = call_lock.lock().await;
                call.queue().current()
            };
            match current {
                Some(track) => {
                    let playing = matches!(
                        track.get_info().await.map(|i| i.playing),
                        Ok(PlayMode::Play)
                    );
                    let call = call_lock.lock().await;
                    if playing {
                        let _ = call.queue().pause();
                        "⏸️ Paused."
                    } else {
                        let _ = call.queue().resume();
                        "▶️ Resumed."
                    }
                }
                None => "Nothing is playing.",
            }
        }
        _ => "Unknown control.",
    };

    ack(ctx, component, msg).await
}

async fn ack(ctx: &Context, component: &ComponentInteraction, content: &str) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    component
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

// ── autocomplete (song list) ─────────────────────────────────────────────────

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

    let files = tokio::task::spawn_blocking(move || list_music_files(&music_path, &partial, 25))
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

/// Recursively list audio files under `base`, relative paths, filtered by a
/// case-insensitive substring, capped at `limit` (and at 100 chars per path).
pub fn list_music_files(base: &str, filter: &str, limit: usize) -> Vec<String> {
    let base_path = std::path::Path::new(base);
    let filter_l = filter.to_lowercase();
    let mut out = Vec::new();
    let mut stack = vec![base_path.to_path_buf()];
    let mut scanned = 0u32;

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if out.len() >= limit || scanned > 10_000 {
                return out;
            }
            scanned += 1;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let is_audio = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| AUDIO_EXTS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if !is_audio {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(base_path) {
                if let Some(rel_s) = rel.to_str() {
                    let rel_s = rel_s.replace('\\', "/");
                    if rel_s.len() <= 100
                        && (filter_l.is_empty() || rel_s.to_lowercase().contains(&filter_l))
                    {
                        out.push(rel_s);
                    }
                }
            }
        }
    }
    out
}

// ── pause / stop / queue / volume ───────────────────────────────────────────

/// The pool bot to control for a slash command: the one in the user's current
/// voice channel, else any active bot in the guild.
async fn control_target(ctx: &Context, command: &CommandInteraction) -> Option<Arc<Songbird>> {
    let guild_id = command.guild_id?;
    let state = super::state(ctx).await;
    if let Some(channel) = user_voice_channel(ctx, guild_id, command.user.id) {
        if let Some((_, sb)) = state.pool.find_in_channel(guild_id, channel).await {
            return Some(sb);
        }
    }
    state.pool.any_active(guild_id).await
}

async fn pause(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let msg = match control_target(ctx, command).await {
        Some(songbird) => {
            if let Some(call_lock) = songbird.get(guild_id) {
                let call = call_lock.lock().await;
                let _ = call.queue().pause();
            }
            "⏸️ Paused."
        }
        None => "Nothing is playing.",
    };
    super::respond(ctx, command, msg).await
}

async fn stop(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let msg = match control_target(ctx, command).await {
        Some(songbird) => {
            if let Some(call_lock) = songbird.get(guild_id) {
                let call = call_lock.lock().await;
                call.queue().stop();
            }
            let _ = songbird.remove(guild_id).await;
            "⏹️ Stopped and left the channel."
        }
        None => "Nothing is playing.",
    };
    super::respond(ctx, command, msg).await
}

async fn queue(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let msg = match control_target(ctx, command).await {
        Some(songbird) => {
            if let Some(call_lock) = songbird.get(guild_id) {
                let call = call_lock.lock().await;
                let n = call.queue().current_queue().len();
                if n == 0 {
                    "The queue is empty.".to_string()
                } else {
                    format!("🎶 {n} track(s) in the queue.")
                }
            } else {
                "Nothing is playing.".to_string()
            }
        }
        None => "Nothing is playing.".to_string(),
    };
    super::respond(ctx, command, msg).await
}

async fn volume(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let level = super::sub_option_i64(command, "level")
        .unwrap_or(100)
        .clamp(0, 100);
    let vol = level as f32 / 100.0;

    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let msg = match control_target(ctx, command).await {
        Some(songbird) => {
            if let Some(call_lock) = songbird.get(guild_id) {
                let call = call_lock.lock().await;
                if let Some(track) = call.queue().current() {
                    let _ = track.set_volume(vol);
                }
            }
            format!("🔊 Volume set to {level}%.")
        }
        None => "Nothing is playing.".to_string(),
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

/// Periodic watchdog: leaves the channel after 3 minutes with an empty queue.
struct IdleLeaver {
    manager: Arc<Songbird>,
    guild_id: GuildId,
    idle_minutes: Arc<AtomicU32>,
}

#[serenity::async_trait]
impl songbird::EventHandler for IdleLeaver {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Some(call_lock) = self.manager.get(self.guild_id) {
            let empty = {
                let call = call_lock.lock().await;
                call.queue().is_empty()
            };
            if empty {
                if self.idle_minutes.fetch_add(1, Ordering::SeqCst) + 1 >= 3 {
                    let _ = self.manager.remove(self.guild_id).await;
                }
            } else {
                self.idle_minutes.store(0, Ordering::SeqCst);
            }
        }
        None
    }
}

/// Leaves the voice channel as soon as a track ends (for one-shot join sounds).
struct LeaveOnEnd {
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

#[serenity::async_trait]
impl songbird::EventHandler for LeaveOnEnd {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let _ = self.manager.remove(self.guild_id).await;
        None
    }
}
