//! NigerianBot API backend — Actix-web entry point.
//!
//! Phase 1 exposes a `/health` endpoint plus the route surface for the dashboard
//! API. Concrete handlers (auth, bot control, workflows, service proxies) arrive
//! in Phase 4 and beyond; for now the `/api/*` routes return 501 Not Implemented.

mod auth;
mod config;
mod handlers;
mod routes;

use actix_web::{App, HttpServer};
use tracing::info;

use crate::config::ApiConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("api");

    let config = ApiConfig::from_env().expect("invalid API configuration");
    let bind = (config.host.clone(), config.port);
    info!(host = %bind.0, port = bind.1, "starting API server");

    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .configure(routes::configure)
    })
    .bind(bind)?
    .run()
    .await
}
