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
