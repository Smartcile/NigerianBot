//! Shared, process-wide bot state.
//!
//! Stored in serenity's per-client `TypeMap` (`ctx.data`) so every command
//! handler can read it without global statics.

use std::sync::Arc;
use std::time::Instant;

use serenity::all::{ChannelId, GuildId};
use serenity::prelude::TypeMapKey;
use songbird::Songbird;
use sqlx::PgPool;

/// One member of the voice pool: a Discord bot that can hold a voice connection.
pub struct VoiceBot {
    pub songbird: Arc<Songbird>,
    #[allow(dead_code)]
    pub label: String,
}

/// The pool of bots available for voice. Index 0 is the primary (command) bot;
/// the rest come from `DISCORD_TOKEN_2..` and exist only to add voice slots.
#[derive(Default)]
pub struct VoicePool {
    pub bots: Vec<VoiceBot>,
}

impl VoicePool {
    pub fn len(&self) -> usize {
        self.bots.len()
    }

    /// The first bot with no active voice call in this guild, with its index.
    pub fn pick_free(&self, guild_id: GuildId) -> Option<(usize, Arc<Songbird>)> {
        self.bots
            .iter()
            .enumerate()
            .find(|(_, b)| b.songbird.get(guild_id).is_none())
            .map(|(i, b)| (i, b.songbird.clone()))
    }

    /// The pool bot at `index`.
    pub fn get(&self, index: usize) -> Option<Arc<Songbird>> {
        self.bots.get(index).map(|b| b.songbird.clone())
    }

    /// Any pool bot with an active call in this guild (first one found).
    pub fn any_active(&self, guild_id: GuildId) -> Option<Arc<Songbird>> {
        self.bots
            .iter()
            .find(|b| b.songbird.get(guild_id).is_some())
            .map(|b| b.songbird.clone())
    }

    /// The pool bot whose active call is in `channel_id`, if any.
    pub async fn bot_in_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Arc<Songbird>> {
        for b in &self.bots {
            if let Some(call_lock) = b.songbird.get(guild_id) {
                let current = call_lock.lock().await.current_channel();
                if current.map(|c| c.0.get()) == Some(channel_id.get()) {
                    return Some(b.songbird.clone());
                }
            }
        }
        None
    }
}

pub struct BotState {
    /// When the process started — used to compute uptime for `/bot status`.
    pub started_at: Instant,
    /// Database pool, present when `DATABASE_URL` is configured.
    pub db: Option<PgPool>,
    /// HTTP client used for music sources (yt-dlp/URL streaming).
    pub http: reqwest::Client,
    /// Base directory for local music files (mounted into the container).
    pub music_path: String,
    /// Voice bots available for simultaneous playback.
    pub pool: VoicePool,
}

impl BotState {
    pub fn new(
        db: Option<PgPool>,
        http: reqwest::Client,
        music_path: String,
        pool: VoicePool,
    ) -> Arc<Self> {
        Arc::new(Self {
            started_at: Instant::now(),
            db,
            http,
            music_path,
            pool,
        })
    }
}

/// `TypeMap` key under which `Arc<BotState>` is stored in `ctx.data`.
pub struct BotStateKey;

impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
