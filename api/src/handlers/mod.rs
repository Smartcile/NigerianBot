//! HTTP request handlers.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::{self, AuthUser};
use crate::state::AppState;

// ── Public ─────────────────────────────────────────────────────────────────

/// Liveness/health probe used by Docker and the deploy pipeline.
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Placeholder for routes that exist in the API surface but aren't built yet.
pub async fn not_implemented() -> impl Responder {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "This endpoint is planned for a later phase.",
    }))
}

// ── Auth ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

/// `POST /api/auth/login` — exchange the shared API key for a JWT.
pub async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> impl Responder {
    let cfg = &state.config;

    if cfg.api_key.is_empty() {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "auth_disabled",
            "message": "API_KEY is not configured on the server.",
        }));
    }

    if !constant_time_eq(body.api_key.as_bytes(), cfg.api_key.as_bytes()) {
        return HttpResponse::Unauthorized().json(json!({ "error": "invalid_api_key" }));
    }

    match auth::issue_token(&cfg.jwt_secret, "api", "admin", cfg.token_ttl_secs) {
        Ok(token) => HttpResponse::Ok().json(TokenResponse {
            token,
            token_type: "Bearer",
            expires_in: cfg.token_ttl_secs,
        }),
        Err(e) => {
            error!(?e, "failed to issue token");
            HttpResponse::InternalServerError().json(json!({ "error": "token_error" }))
        }
    }
}

/// `POST /api/auth/refresh` — issue a fresh token for an already-authenticated caller.
pub async fn refresh(state: web::Data<AppState>, user: AuthUser) -> impl Responder {
    match auth::issue_token(
        &state.config.jwt_secret,
        &user.claims.sub,
        &user.claims.role,
        state.config.token_ttl_secs,
    ) {
        Ok(token) => HttpResponse::Ok().json(TokenResponse {
            token,
            token_type: "Bearer",
            expires_in: state.config.token_ttl_secs,
        }),
        Err(e) => {
            error!(?e, "failed to refresh token");
            HttpResponse::InternalServerError().json(json!({ "error": "token_error" }))
        }
    }
}

// ── Bot (protected) ─────────────────────────────────────────────────────────

/// `GET /api/bot/status` — service + database stats.
pub async fn bot_status(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let commands_logged: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    HttpResponse::Ok().json(json!({
        "service": "api",
        "status": "ok",
        "database": "connected",
        "commands_logged": commands_logged,
    }))
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AuditRow {
    pub id: i64,
    pub user_id: i64,
    pub user_name: Option<String>,
    pub guild_id: Option<i64>,
    pub command: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/bot/logs?limit=N` — recent audit-log entries written by the bot.
pub async fn bot_logs(
    state: web::Data<AppState>,
    _user: AuthUser,
    query: web::Query<LogQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, user_id, user_name, guild_id, command, created_at \
         FROM audit_log ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "logs": rows })),
        Err(e) => {
            error!(?e, "failed to query audit log");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CommandCount {
    pub command: String,
    pub count: i64,
}

/// `GET /api/stats` — aggregate command stats for the dashboard.
pub async fn stats(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let last_24h: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE created_at > now() - interval '24 hours'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let top: Vec<CommandCount> = sqlx::query_as(
        "SELECT command, count(*)::bigint AS count FROM audit_log \
         GROUP BY command ORDER BY count DESC LIMIT 6",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(json!({ "total": total, "last_24h": last_24h, "top": top }))
}

/// Length-checked constant-time byte comparison to avoid timing leaks on the key.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
