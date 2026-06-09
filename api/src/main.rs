//! NigerianBot API backend — Actix-web entry point.
//!
//! Connects to the same PostgreSQL as the bot, applies migrations (safe to run
//! concurrently — sqlx takes an advisory lock), and serves the dashboard/control
//! API. `/health` is public; `/api/*` endpoints require a JWT obtained from
//! `/api/auth/login` with the configured `API_KEY`.

mod auth;
mod config;
mod handlers;
mod routes;
mod state;

use actix_web::{web, App, HttpServer};
use tracing::{error, info, warn};

use crate::config::ApiConfig;
use crate::state::AppState;

/// Connect to Postgres, retrying briefly to tolerate the database warming up.
async fn connect_db_with_retry(url: &str) -> anyhow::Result<sqlx::PgPool> {
    const MAX_ATTEMPTS: u32 = 10;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match common::db::connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(attempt, error = %e, "database not ready, retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("api");

    let config = ApiConfig::from_env().expect("invalid API configuration");
    let bind = (config.host.clone(), config.port);

    if config.api_key.is_empty() {
        warn!("API_KEY is not set — /api/auth/login will reject all logins");
    }

    let Some(url) = config.database_url.clone() else {
        error!("DATABASE_URL is required for the API service");
        std::process::exit(1);
    };

    let db = connect_db_with_retry(&url).await.unwrap_or_else(|e| {
        error!(error = %e, "could not connect to the database");
        std::process::exit(1);
    });
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .unwrap_or_else(|e| {
            error!(error = %e, "failed to run database migrations");
            std::process::exit(1);
        });
    info!("database connected and migrations applied");

    let state = web::Data::new(AppState { db, config });

    info!(host = %bind.0, port = bind.1, "starting API server");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(tracing_actix_web::TracingLogger::default())
            .configure(routes::configure)
    })
    .bind(bind)?
    .run()
    .await
}
