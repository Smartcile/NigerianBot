//! NigerianBot Telegram surface.
//!
//! A thin bot over the same core as the Discord bot and web GUI: it reads/writes
//! the shared Postgres (download queue, identity) and relies on the worker for
//! background work. Notifications on completion are sent by the worker.

mod commands;
mod config;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use sqlx::PgPool;
use teloxide::dptree;
use teloxide::prelude::*;
use tracing::{info, warn};

use crate::config::TelegramConfig;

/// Dependency-injected state for command handlers.
pub struct AppState {
    pub db: Option<PgPool>,
    pub admin_ids: Vec<u64>,
    pub public_base_url: Option<String>,
}

/// Connect to Postgres, retrying briefly so the service tolerates the database
/// still warming up on first stack startup.
async fn connect_db_with_retry(url: &str) -> anyhow::Result<PgPool> {
    const MAX_ATTEMPTS: u32 = 10;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match common::db::connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(attempt, error = %e, "database not ready, retrying in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e.context("could not connect to the database")),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("telegram");

    let config = TelegramConfig::from_env().context("invalid telegram configuration")?;

    let db = match config.database_url.clone() {
        Some(url) => {
            let pool = connect_db_with_retry(&url).await?;
            sqlx::migrate!("../migrations")
                .run(&pool)
                .await
                .context("failed to run database migrations")?;
            info!("database connected and migrations applied");
            Some(pool)
        }
        None => {
            warn!("no DATABASE_URL set — download commands are unavailable");
            None
        }
    };

    let state = Arc::new(AppState {
        db,
        admin_ids: config.admin_ids.clone(),
        public_base_url: config.public_base_url.clone(),
    });

    let bot = Bot::new(config.bot_token.clone());
    info!("telegram bot starting");

    let handler = Update::filter_message()
        .filter_command::<commands::Command>()
        .endpoint(commands::handle);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
