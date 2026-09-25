//! Download-queue endpoints for the web GUI. Rows are queued here; the worker
//! service performs the actual fetch.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct DownloadRow {
    pub id: i64,
    pub url: String,
    pub provider: String,
    pub title: Option<String>,
    pub status: String,
    pub file_path: Option<String>,
    pub size_bytes: Option<i64>,
    pub error: Option<String>,
    pub requested_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

const COLUMNS: &str = "id, url, provider, title, status, file_path, size_bytes, error, \
                       requested_by, created_at, updated_at";

/// `GET /api/downloads?limit=N` — recent download jobs, newest first.
pub async fn list(
    state: web::Data<AppState>,
    _user: AuthUser,
    query: web::Query<ListQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query_as::<_, DownloadRow>(&format!(
        "SELECT {COLUMNS} FROM downloads ORDER BY created_at DESC LIMIT $1"
    ))
    .bind(limit)
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "downloads": rows })),
        Err(e) => {
            error!(?e, "failed to query downloads");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

#[derive(Deserialize)]
pub struct CreateDownload {
    pub url: String,
}

/// `POST /api/downloads` — queue a URL for download.
pub async fn create(
    state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Json<CreateDownload>,
) -> impl Responder {
    let url = body.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return HttpResponse::BadRequest()
            .json(json!({ "error": "invalid_url", "message": "Provide an http(s) URL." }));
    }

    let provider = common::media::provider_of(url);
    let row = sqlx::query_as::<_, DownloadRow>(&format!(
        "INSERT INTO downloads (url, provider, requested_by) VALUES ($1, $2, 'web') \
         RETURNING {COLUMNS}"
    ))
    .bind(url)
    .bind(provider)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(row) => HttpResponse::Created().json(row),
        Err(e) => {
            error!(?e, "failed to queue download");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

/// `DELETE /api/downloads/{id}` — remove a job and (best effort) its file.
pub async fn delete(
    state: web::Data<AppState>,
    _user: AuthUser,
    id: web::Path<i64>,
) -> impl Responder {
    let deleted: Result<Option<(Option<String>,)>, _> =
        sqlx::query_as("DELETE FROM downloads WHERE id = $1 RETURNING file_path")
            .bind(id.into_inner())
            .fetch_optional(&state.db)
            .await;

    match deleted {
        Ok(Some((Some(file),))) => {
            let path = std::path::Path::new(&state.config.downloads_path).join(file);
            let _ = std::fs::remove_file(path);
            HttpResponse::NoContent().finish()
        }
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to delete download");
            HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
        }
    }
}

// ── yt-dlp cookies (for login-gated sites, e.g. Vimeo) ──────────────────────

/// `GET /api/downloads/cookies` — whether a cookies file is stored.
pub async fn cookies_status(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    match std::fs::metadata(&state.config.cookies_path) {
        Ok(m) => {
            let modified = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            HttpResponse::Ok().json(json!({
                "present": true,
                "size": m.len(),
                "modified": modified,
            }))
        }
        Err(_) => HttpResponse::Ok().json(json!({ "present": false })),
    }
}

/// `POST /api/downloads/cookies` — store an uploaded Netscape `cookies.txt`.
pub async fn upload_cookies(
    state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Bytes,
) -> impl Responder {
    if body.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "empty" }));
    }
    if body.len() > 2 * 1024 * 1024 {
        return HttpResponse::PayloadTooLarge()
            .json(json!({ "error": "too_large", "message": "Max 2 MB." }));
    }
    // Light validation: Netscape cookie files are tab-separated.
    let text = String::from_utf8_lossy(&body);
    if !text.contains("Netscape") && !text.contains('\t') {
        return HttpResponse::BadRequest().json(json!({
            "error": "not_cookies",
            "message": "That doesn't look like a Netscape cookies.txt file.",
        }));
    }

    let path = std::path::Path::new(&state.config.cookies_path);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            error!(?e, "failed to create cookies directory");
            return HttpResponse::InternalServerError().json(json!({ "error": "io_error" }));
        }
    }

    match std::fs::write(path, &body) {
        Ok(_) => HttpResponse::Ok().json(json!({ "present": true, "size": body.len() })),
        Err(e) => {
            error!(?e, "failed to write cookies file");
            HttpResponse::InternalServerError().json(json!({ "error": "io_error" }))
        }
    }
}

/// `DELETE /api/downloads/cookies` — remove the stored cookies.
pub async fn delete_cookies(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    match std::fs::remove_file(&state.config.cookies_path) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(?e, "failed to remove cookies file");
            HttpResponse::InternalServerError().json(json!({ "error": "io_error" }))
        }
    }
}
