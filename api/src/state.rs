//! Shared application state injected into every handler via `web::Data`.

use sqlx::PgPool;

use crate::config::ApiConfig;

pub struct AppState {
    pub db: PgPool,
    pub config: ApiConfig,
    /// Sonarr (TV) client, when configured.
    pub sonarr: Option<common::arr::Arr>,
    /// Radarr (movies) client, when configured.
    pub radarr: Option<common::arr::Arr>,
}
