//! Telegram panel for the web GUI: config status and the `telegram_users`
//! directory with role management.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct TelegramUserRow {
    pub telegram_id: i64,
    pub username: Option<String>,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/telegram/status`
pub async fn status(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM telegram_users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    HttpResponse::Ok().json(json!({
        "configured": state.config.telegram_configured,
        "chat_id": state.config.telegram_chat_id,
        "users": users,
    }))
}

/// `GET /api/telegram/users`
pub async fn list_users(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let rows = sqlx::query_as::<_, TelegramUserRow>(
        "SELECT telegram_id, username, role, created_at FROM telegram_users \
         ORDER BY CASE role WHEN 'admin' THEN 0 WHEN 'viewer' THEN 2 ELSE 1 END, username",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "users": rows })),
        Err(e) => {
            error!(?e, "failed to list telegram users");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Deserialize)]
pub struct RoleBody {
    pub role: String,
}

/// `POST /api/telegram/users/{telegram_id}/role`
pub async fn set_role(
    state: web::Data<AppState>,
    _user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<RoleBody>,
) -> impl Responder {
    if !matches!(body.role.as_str(), "admin" | "user" | "viewer") {
        return HttpResponse::BadRequest().json(json!({ "error": "invalid_role" }));
    }
    let result = sqlx::query(
        "INSERT INTO telegram_users (telegram_id, role) VALUES ($1, $2) \
         ON CONFLICT (telegram_id) DO UPDATE SET role = EXCLUDED.role, updated_at = now()",
    )
    .bind(id.into_inner())
    .bind(&body.role)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to set telegram role");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}
