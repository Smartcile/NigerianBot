//! Schedule management for the web GUI. The bot's background scheduler executes
//! these rows; the GUI can list, create, enable/disable, and delete them.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct ScheduleRow {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub kind: String,
    pub message: Option<String>,
    pub interval_seconds: Option<i64>,
    pub next_run_at: chrono::DateTime<chrono::Utc>,
    pub enabled: bool,
    pub created_by: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

const COLUMNS: &str = "id, guild_id, channel_id, kind, message, interval_seconds, \
                       next_run_at, enabled, created_by, created_at";

#[derive(Deserialize)]
pub struct ListQuery {
    pub guild_id: Option<i64>,
}

/// `GET /api/schedules` — list schedules (optionally for one guild).
pub async fn list(
    state: web::Data<AppState>,
    _user: AuthUser,
    query: web::Query<ListQuery>,
) -> impl Responder {
    let rows = match query.guild_id {
        Some(guild) => {
            sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {COLUMNS} FROM schedules WHERE guild_id = $1 ORDER BY next_run_at"
            ))
            .bind(guild)
            .fetch_all(&state.db)
            .await
        }
        None => {
            sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {COLUMNS} FROM schedules ORDER BY next_run_at"
            ))
            .fetch_all(&state.db)
            .await
        }
    };

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "schedules": rows })),
        Err(e) => {
            error!(?e, "failed to list schedules");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Deserialize)]
pub struct CreateSchedule {
    pub guild_id: i64,
    pub channel_id: i64,
    pub kind: String,
    pub message: Option<String>,
    /// Repeat interval; `null` = one-off.
    pub interval_seconds: Option<i64>,
    /// When the next run is due, in minutes from now (default 1).
    pub minutes: Option<i64>,
}

/// `POST /api/schedules` — create a schedule.
pub async fn create(
    state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Json<CreateSchedule>,
) -> impl Responder {
    if !matches!(
        body.kind.as_str(),
        "message" | "digest_sonarr" | "digest_radarr"
    ) {
        return HttpResponse::BadRequest().json(json!({ "error": "invalid_kind" }));
    }
    let minutes = body.minutes.unwrap_or(1).max(1);

    let row = sqlx::query_as::<_, ScheduleRow>(&format!(
        "INSERT INTO schedules (guild_id, channel_id, kind, message, interval_seconds, next_run_at) \
         VALUES ($1, $2, $3, $4, $5, now() + ($6 * interval '1 minute')) RETURNING {COLUMNS}"
    ))
    .bind(body.guild_id)
    .bind(body.channel_id)
    .bind(&body.kind)
    .bind(&body.message)
    .bind(body.interval_seconds)
    .bind(minutes)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(row) => HttpResponse::Created().json(row),
        Err(e) => {
            error!(?e, "failed to create schedule");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Deserialize)]
pub struct Toggle {
    pub enabled: bool,
}

/// `POST /api/schedules/{id}/enabled` — enable or disable a schedule.
pub async fn set_enabled(
    state: web::Data<AppState>,
    _user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<Toggle>,
) -> impl Responder {
    let result = sqlx::query("UPDATE schedules SET enabled = $2 WHERE id = $1")
        .bind(id.into_inner())
        .bind(body.enabled)
        .execute(&state.db)
        .await;
    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to toggle schedule");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

/// `DELETE /api/schedules/{id}`
pub async fn delete(
    state: web::Data<AppState>,
    _user: AuthUser,
    id: web::Path<i64>,
) -> impl Responder {
    match sqlx::query("DELETE FROM schedules WHERE id = $1")
        .bind(id.into_inner())
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to delete schedule");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}
