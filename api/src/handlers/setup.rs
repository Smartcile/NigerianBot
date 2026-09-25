//! Live integration status for the Setup page. Reports only *presence* and
//! non-secret values (URLs, counts) — never secrets.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::auth::AuthUser;
use crate::state::AppState;

/// `GET /api/setup/status` — which services are wired up right now.
pub async fn status(state: web::Data<AppState>, _user: AuthUser) -> HttpResponse {
    let downloads_total: i64 = scalar(&state, "SELECT count(*) FROM downloads").await;
    let downloads_done: i64 = scalar(
        &state,
        "SELECT count(*) FROM downloads WHERE status = 'done'",
    )
    .await;
    let downloads_active: i64 = scalar(
        &state,
        "SELECT count(*) FROM downloads WHERE status IN ('queued','downloading')",
    )
    .await;
    let bot_commands: i64 = scalar(&state, "SELECT count(*) FROM audit_log").await;
    let bot_last: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT max(created_at) FROM audit_log")
            .fetch_one(&state.db)
            .await
            .unwrap_or(None);

    HttpResponse::Ok().json(json!({
        "database": true,
        "api": { "configured": true },
        "pin_default": crate::auth::pin_is_default(&state.db).await,
        "bot": {
            "configured": bot_commands > 0,
            "commands": bot_commands,
            "last_seen": bot_last,
        },
        "sonarr": {
            "configured": state.sonarr.is_some(),
            "url": state.config.sonarr_url,
        },
        "radarr": {
            "configured": state.radarr.is_some(),
            "url": state.config.radarr_url,
        },
        "telegram": {
            "configured": state.config.telegram_configured,
            "chat_id": state.config.telegram_chat_id,
        },
        "downloads": {
            "path": state.config.downloads_path,
            "total": downloads_total,
            "done": downloads_done,
            "active": downloads_active,
        },
        "public_base_url": state.config.public_base_url,
    }))
}

async fn scalar(state: &AppState, query: &str) -> i64 {
    sqlx::query_scalar(query)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0)
}
