//! Shared, process-wide bot state.
//!
//! Stored in serenity's per-client `TypeMap` (`ctx.data`) so every command
//! handler can read it without global statics.

use std::sync::Arc;
use std::time::Instant;

use serenity::prelude::TypeMapKey;
use sqlx::PgPool;

pub struct BotState {
    /// When the process started — used to compute uptime for `/bot status`.
    pub started_at: Instant,
    /// Database pool, present when `DATABASE_URL` is configured.
    pub db: Option<PgPool>,
    /// HTTP client used for music sources (yt-dlp/URL streaming) and, later,
    /// the Sonar/Radar integrations.
    pub http: reqwest::Client,
    /// Base directory for local music files (mounted into the container).
    pub music_path: String,
}

impl BotState {
    pub fn new(db: Option<PgPool>, http: reqwest::Client, music_path: String) -> Arc<Self> {
        Arc::new(Self {
            started_at: Instant::now(),
            db,
            http,
            music_path,
        })
    }
}

/// `TypeMap` key under which `Arc<BotState>` is stored in `ctx.data`.
pub struct BotStateKey;

impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
