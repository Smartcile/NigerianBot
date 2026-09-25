//! User directory & role management for the web GUI (the `users` table the bot
//! writes to, keyed on Discord id).

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub discord_id: i64,
    pub discord_name: Option<String>,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/users` — everyone the bot has seen, elevated roles first.
pub async fn list(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let rows = sqlx::query_as::<_, UserRow>(
        "SELECT discord_id, discord_name, role, created_at FROM users \
         ORDER BY CASE role WHEN 'admin' THEN 0 WHEN 'viewer' THEN 2 ELSE 1 END, discord_name",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "users": rows })),
        Err(e) => {
            error!(?e, "failed to list users");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Deserialize)]
pub struct RoleBody {
    pub role: String,
}

/// `POST /api/users/{discord_id}/role` — set a user's role.
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
        "INSERT INTO users (discord_id, role) VALUES ($1, $2) \
         ON CONFLICT (discord_id) DO UPDATE SET role = EXCLUDED.role, updated_at = now()",
    )
    .bind(id.into_inner())
    .bind(&body.role)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to set user role");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}
