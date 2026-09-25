//! API service: REST endpoints + the served React dashboard.
//!
//! Exposed as a library so the all-in-one `nigerianbot` binary can run it
//! in-process; the `nigerian-api` binary runs it standalone.

mod auth;
mod config;
mod handlers;
mod routes;
mod state;

use actix_web::{web, App, HttpServer};
use anyhow::Context as _;
use tracing::{info, warn};

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

/// Connect, migrate, and serve the API + dashboard until the process stops.
pub async fn run() -> anyhow::Result<()> {
    let config = ApiConfig::from_env().context("invalid API configuration")?;
    let bind = (config.host.clone(), config.port);

    if config.api_key.is_empty() {
        warn!("API_KEY is not set — /api/auth/login will reject all logins");
    }

    let url = config
        .database_url
        .clone()
        .context("DATABASE_URL is required for the API service")?;

    let db = connect_db_with_retry(&url).await?;
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .context("failed to run database migrations")?;
    info!("database connected and migrations applied");

    let http = reqwest::Client::new();
    let sonarr = match (config.sonarr_url.clone(), config.sonarr_api_key.clone()) {
        (Some(url), Some(key)) => Some(common::arr::Arr::new(http.clone(), url, key)),
        _ => None,
    };
    let radarr = match (config.radarr_url.clone(), config.radarr_api_key.clone()) {
        (Some(url), Some(key)) => Some(common::arr::Arr::new(http.clone(), url, key)),
        _ => None,
    };

    let state = web::Data::new(AppState {
        db,
        config,
        sonarr,
        radarr,
    });

    // Directory of the built React dashboard (served as static files). Registered
    // after the API routes so `/health` and `/api/*` always take precedence.
    let static_dir = common::config::optional_or("DASHBOARD_DIR", "/app/static");
    // Finished downloads, served read-only so the GUI can link/open files.
    let downloads_dir = state.config.downloads_path.clone();

    info!(host = %bind.0, port = bind.1, "starting API server");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(tracing_actix_web::TracingLogger::default())
            .configure(routes::configure)
            .service(actix_files::Files::new("/media", downloads_dir.clone()))
            .service(actix_files::Files::new("/", static_dir.clone()).index_file("index.html"))
    })
    .bind(bind)?
    .run()
    .await?;

    Ok(())
}
