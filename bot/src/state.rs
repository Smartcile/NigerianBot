//! Shared, process-wide bot state.
//!
//! Stored in serenity's per-client `TypeMap` (`ctx.data`) so every command
//! handler can read it without global statics. Extend `BotState` in later phases
//! with the database pool, external-service HTTP clients, cached config, etc.

use std::sync::Arc;
use std::time::Instant;

use serenity::prelude::TypeMapKey;

pub struct BotState {
    /// When the process started — used to compute uptime for `/bot status`.
    pub started_at: Instant,
}

impl BotState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started_at: Instant::now(),
        })
    }
}

/// `TypeMap` key under which `Arc<BotState>` is stored in `ctx.data`.
pub struct BotStateKey;

impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
