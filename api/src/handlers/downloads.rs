//! Download-queue endpoints for the web GUI. Rows are queued here; the worker
//! service performs the actual fetch.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub progress: i32,
    pub save_as: Option<String>,
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

const COLUMNS: &str = "id, url, provider, title, status, progress, save_as, file_path, \
                       size_bytes, error, requested_by, created_at, updated_at";

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
    /// Optional filename to save as (so Sonarr/Radarr can parse it).
    #[serde(default)]
    pub save_as: Option<String>,
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
    let save_as = body
        .save_as
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let row = sqlx::query_as::<_, DownloadRow>(&format!(
        "INSERT INTO downloads (url, provider, requested_by, save_as) VALUES ($1, $2, 'web', $3) \
         RETURNING {COLUMNS}"
    ))
    .bind(url)
    .bind(provider)
    .bind(save_as)
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

// ── Bulk import (paste many URLs, or a series/playlist link) ────────────────

#[derive(Deserialize)]
pub struct BulkImport {
    /// One or more URLs, separated by whitespace/newlines.
    pub urls: String,
}

struct Episode {
    url: String,
    title: String,
}

/// `POST /api/downloads/bulk` — expand any playlist/series URL into episodes and
/// queue them all (skipping duplicates).
pub async fn bulk_import(
    state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Json<BulkImport>,
) -> impl Responder {
    let urls: Vec<String> = body
        .urls
        .split_whitespace()
        .map(str::to_string)
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
        .collect();
    if urls.is_empty() {
        return HttpResponse::BadRequest()
            .json(json!({ "error": "no_urls", "message": "Paste at least one http(s) URL." }));
    }

    let mut added = 0i64;
    let mut skipped = 0i64;
    let mut items: Vec<DownloadRow> = Vec::new();

    for url in urls {
        let episodes = expand_playlist(&state, &url).await;
        let targets: Vec<(String, Option<String>)> = if episodes.is_empty() {
            vec![(url.clone(), None)]
        } else {
            episodes
                .into_iter()
                .enumerate()
                .map(|(i, e)| {
                    (
                        e.url,
                        Some(format!("{:02} - {}", i + 1, sanitize_title(&e.title))),
                    )
                })
                .collect()
        };

        for (ep_url, save_as) in targets {
            let exists: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM downloads WHERE url = $1 AND status <> 'failed'",
            )
            .bind(&ep_url)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
            if exists > 0 {
                skipped += 1;
                continue;
            }

            let provider = common::media::provider_of(&ep_url);
            let row = sqlx::query_as::<_, DownloadRow>(&format!(
                "INSERT INTO downloads (url, provider, requested_by, save_as) \
                 VALUES ($1, $2, 'bulk', $3) RETURNING {COLUMNS}"
            ))
            .bind(&ep_url)
            .bind(provider)
            .bind(&save_as)
            .fetch_one(&state.db)
            .await;

            match row {
                Ok(row) => {
                    added += 1;
                    items.push(row);
                }
                Err(e) => {
                    error!(?e, "failed to queue bulk download");
                    skipped += 1;
                }
            }
        }
    }

    HttpResponse::Ok().json(json!({ "added": added, "skipped": skipped, "items": items }))
}

/// Use yt-dlp to list a series/playlist's episodes. Empty if it's a single video.
async fn expand_playlist(state: &AppState, url: &str) -> Vec<Episode> {
    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.args(["--flat-playlist", "--no-warnings", "-J"]);
    if std::path::Path::new(&state.config.cookies_path).is_file() {
        cmd.arg("--cookies").arg(&state.config.cookies_path);
    }

    let output =
        match tokio::time::timeout(std::time::Duration::from_secs(45), cmd.arg(url).output()).await
        {
            Ok(Ok(o)) if o.status.success() => o,
            _ => return Vec::new(),
        };

    let data: Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(entries) = data.get("entries").and_then(|e| e.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for e in entries {
        let resolved = e
            .get("url")
            .and_then(|u| u.as_str())
            .filter(|u| u.starts_with("http"))
            .or_else(|| e.get("webpage_url").and_then(|u| u.as_str()))
            .map(str::to_string)
            .or_else(|| {
                // Reconstruct from a known provider + id as a fallback.
                let key = e.get("ie_key").and_then(|k| k.as_str()).unwrap_or("");
                let id = e.get("id").and_then(|i| i.as_str())?;
                if key == "Youtube" {
                    Some(format!("https://www.youtube.com/watch?v={id}"))
                } else if key.starts_with("Tubi") {
                    Some(format!("https://tubitv.com/video/{id}"))
                } else {
                    None
                }
            });
        if let Some(url) = resolved {
            let title = e
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("episode")
                .to_string();
            out.push(Episode { url, title });
        }
    }
    out
}

fn sanitize_title(title: &str) -> String {
    title
        .replace(['/', '\\'], "_")
        .replace("..", "_")
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}
