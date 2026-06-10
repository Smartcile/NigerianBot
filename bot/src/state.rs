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

use crate::services::arr::Arr;

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

    /// The pool bot at `index`.
    pub fn get(&self, index: usize) -> Option<Arc<Songbird>> {
        self.bots.get(index).map(|b| b.songbird.clone())
    }

    /// The voice channel a bot is actually connected to in this guild, if any.
    /// A leftover call with no live connection reads as `None` (i.e. free), so a
    /// failed join can't permanently mark a bot as busy.
    async fn connected_channel(songbird: &Songbird, guild_id: GuildId) -> Option<u64> {
        let call = songbird.get(guild_id)?;
        let channel = call.lock().await.current_channel();
        channel.map(|c| c.0.get())
    }

    /// All bots not currently in a voice channel in this guild, with their index.
    pub async fn free_bots(&self, guild_id: GuildId) -> Vec<(usize, Arc<Songbird>)> {
        let mut free = Vec::new();
        for (i, b) in self.bots.iter().enumerate() {
            if Self::connected_channel(&b.songbird, guild_id)
                .await
                .is_none()
            {
                free.push((i, b.songbird.clone()));
            }
        }
        free
    }

    /// The bot (and its index) currently connected to `channel_id`, if any.
    pub async fn find_in_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<(usize, Arc<Songbird>)> {
        for (i, b) in self.bots.iter().enumerate() {
            if Self::connected_channel(&b.songbird, guild_id).await == Some(channel_id.get()) {
                return Some((i, b.songbird.clone()));
            }
        }
        None
    }

    /// Any bot actively connected somewhere in this guild (first found).
    pub async fn any_active(&self, guild_id: GuildId) -> Option<Arc<Songbird>> {
        for b in &self.bots {
            if Self::connected_channel(&b.songbird, guild_id)
                .await
                .is_some()
            {
                return Some(b.songbird.clone());
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
    /// Sonarr (TV) client, when configured.
    pub sonarr: Option<Arr>,
    /// Radarr (movies) client, when configured.
    pub radarr: Option<Arr>,
    /// Discord ids always treated as Admin (bootstrap).
    pub admin_ids: Vec<u64>,
}

impl BotState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Option<PgPool>,
        http: reqwest::Client,
        music_path: String,
        pool: VoicePool,
        sonarr: Option<Arr>,
        radarr: Option<Arr>,
        admin_ids: Vec<u64>,
    ) -> Arc<Self> {
        Arc::new(Self {
            started_at: Instant::now(),
            db,
            http,
            music_path,
            pool,
            sonarr,
            radarr,
            admin_ids,
        })
    }
}

/// `TypeMap` key under which `Arc<BotState>` is stored in `ctx.data`.
pub struct BotStateKey;

impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
