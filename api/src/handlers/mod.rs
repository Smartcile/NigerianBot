//! HTTP request handlers.

pub mod downloads;
pub mod media;
pub mod schedules;
pub mod setup;
pub mod telegram;
pub mod users;

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
    /// Dashboard PIN.
    pub pin: Option<String>,
    /// Optional API key (for scripts/clients).
    pub api_key: Option<String>,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    /// True when the PIN is still the default and must be changed.
    pub must_change_pin: bool,
}

fn token_response(
    cfg: &crate::config::ApiConfig,
    sub: &str,
    must_change_pin: bool,
) -> HttpResponse {
    match auth::issue_token(&cfg.jwt_secret, sub, "admin", cfg.token_ttl_secs) {
        Ok(token) => HttpResponse::Ok().json(TokenResponse {
            token,
            token_type: "Bearer",
            expires_in: cfg.token_ttl_secs,
            must_change_pin,
        }),
        Err(e) => {
            error!(?e, "failed to issue token");
            HttpResponse::InternalServerError().json(json!({ "error": "token_error" }))
        }
    }
}

/// `POST /api/auth/login` — sign in with the dashboard PIN (or the API key).
pub async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> impl Responder {
    let cfg = &state.config;

    // API-key path (optional; lets scripts/clients keep working).
    if let Some(key) = body.api_key.as_deref() {
        if !cfg.api_key.is_empty() && constant_time_eq(key.as_bytes(), cfg.api_key.as_bytes()) {
            return token_response(cfg, "api", false);
        }
    }

    // PIN path (the dashboard).
    let Some(pin) = body.pin.as_deref() else {
        return HttpResponse::BadRequest().json(json!({
            "error": "pin_required",
            "message": "Provide your PIN.",
        }));
    };

    let hash = match auth::stored_pin_hash(&state.db).await {
        Ok(Some(hash)) => hash,
        Ok(None) => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "pin_uninitialized",
                "message": "No PIN set yet.",
            }))
        }
        Err(e) => {
            error!(?e, "failed to read PIN");
            return HttpResponse::InternalServerError().json(json!({ "error": "db_error" }));
        }
    };

    if !auth::verify_pin(pin, &hash) {
        return HttpResponse::Unauthorized().json(json!({ "error": "invalid_pin" }));
    }

    let must_change_pin = auth::pin_is_default(&state.db).await;
    token_response(cfg, "dashboard", must_change_pin)
}

#[derive(Deserialize)]
pub struct ChangePinRequest {
    pub current_pin: String,
    pub new_pin: String,
}

/// `POST /api/auth/change-pin` — set a new dashboard PIN.
pub async fn change_pin(
    state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Json<ChangePinRequest>,
) -> impl Responder {
    let hash = match auth::stored_pin_hash(&state.db).await {
        Ok(Some(hash)) => hash,
        _ => {
            return HttpResponse::ServiceUnavailable().json(json!({ "error": "pin_uninitialized" }))
        }
    };

    if !auth::verify_pin(&body.current_pin, &hash) {
        return HttpResponse::Unauthorized().json(json!({ "error": "invalid_pin" }));
    }
    if !auth::valid_pin(&body.new_pin) {
        return HttpResponse::BadRequest().json(json!({
            "error": "weak_pin",
            "message": "PIN must be 4-8 digits.",
        }));
    }

    match auth::set_pin(&state.db, &body.new_pin, false).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to set PIN");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
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
            must_change_pin: auth::pin_is_default(&state.db).await,
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
