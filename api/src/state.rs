//! Shared application state injected into every handler via `web::Data`.

use sqlx::PgPool;

use crate::config::ApiConfig;

pub struct AppState {
    pub db: PgPool,
    pub config: ApiConfig,
}
