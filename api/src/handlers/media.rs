//! Media (Sonarr / Radarr) endpoints for the web GUI. Thin wrappers over the
//! shared `common::arr::Arr` connector.

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::state::AppState;

fn client<'a>(state: &'a AppState, service: &str) -> Option<&'a common::arr::Arr> {
    match service {
        "sonarr" => state.sonarr.as_ref(),
        "radarr" => state.radarr.as_ref(),
        _ => None,
    }
}

fn not_configured(service: &str) -> HttpResponse {
    HttpResponse::ServiceUnavailable().json(json!({
        "error": "not_configured",
        "message": format!("{service} is not configured on the server."),
    }))
}

fn arr_error(e: anyhow::Error) -> HttpResponse {
    HttpResponse::BadGateway().json(json!({ "error": "arr_error", "message": format!("{e:#}") }))
}

fn library_path(service: &str) -> &'static str {
    if service == "radarr" {
        "movie"
    } else {
        "series"
    }
}

fn lookup_path(service: &str) -> &'static str {
    if service == "radarr" {
        "movie/lookup"
    } else {
        "series/lookup"
    }
}

/// First poster's remote URL from a lookup object, if present.
fn poster(v: &Value) -> Option<String> {
    v["images"]
        .as_array()?
        .iter()
        .find(|i| i["coverType"] == "poster")
        .and_then(|i| i["remoteUrl"].as_str())
        .map(str::to_string)
}

/// `GET /api/media/{service}/status`
pub async fn status(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
) -> HttpResponse {
    let service = path.into_inner();
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };

    let version = arr
        .get::<Value>("system/status")
        .await
        .ok()
        .and_then(|v| v["version"].as_str().map(str::to_string))
        .unwrap_or_else(|| "?".to_string());
    let library = arr
        .get::<Value>(library_path(&service))
        .await
        .ok()
        .and_then(|v| v.as_array().map(|a| a.len()))
        .unwrap_or(0);
    let downloading = arr
        .get_q::<Value>("queue", &[("pageSize", "1")])
        .await
        .ok()
        .and_then(|v| v["totalRecords"].as_u64())
        .unwrap_or(0);

    HttpResponse::Ok().json(json!({
        "service": service,
        "version": version,
        "library": library,
        "downloading": downloading,
    }))
}

/// `GET /api/media/{service}/queue`
pub async fn queue(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
) -> HttpResponse {
    let service = path.into_inner();
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };

    match arr
        .get_q::<Value>("queue", &[("pageSize", "20"), ("sortKey", "timeleft")])
        .await
    {
        Ok(data) => {
            let records: Vec<Value> = data["records"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|r| {
                    let progress = r["size"]
                        .as_f64()
                        .zip(r["sizeleft"].as_f64())
                        .map(|(t, l)| if t > 0.0 { (t - l) / t * 100.0 } else { 0.0 });
                    json!({
                        "title": r["title"],
                        "status": r["status"],
                        "timeleft": r["timeleft"],
                        "progress": progress,
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "records": records }))
        }
        Err(e) => arr_error(e),
    }
}

/// `GET /api/media/{service}/calendar`
pub async fn calendar(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
) -> HttpResponse {
    let service = path.into_inner();
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };

    let days = if service == "radarr" { 30 } else { 7 };
    let now = chrono::Utc::now();
    let end = now + chrono::Duration::days(days);
    let start_s = now.format("%Y-%m-%d").to_string();
    let end_s = end.format("%Y-%m-%d").to_string();

    match arr
        .get_q::<Value>(
            "calendar",
            &[
                ("start", &start_s),
                ("end", &end_s),
                ("includeSeries", "true"),
            ],
        )
        .await
    {
        Ok(data) => {
            let items: Vec<Value> = data
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .take(30)
                .map(|e| {
                    let series = e["series"]["title"].as_str().unwrap_or("");
                    let date = e["airDateUtc"]
                        .as_str()
                        .or_else(|| e["digitalRelease"].as_str())
                        .or_else(|| e["physicalRelease"].as_str())
                        .or_else(|| e["inCinemas"].as_str())
                        .and_then(|d| d.split('T').next())
                        .unwrap_or("TBA");
                    json!({
                        "title": e["title"],
                        "series": series,
                        "season": e["seasonNumber"],
                        "episode": e["episodeNumber"],
                        "year": e["year"],
                        "date": date,
                        "poster": poster(&e),
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "items": items }))
        }
        Err(e) => arr_error(e),
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub term: String,
}

/// `GET /api/media/{service}/search?term=`
pub async fn search(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
    query: web::Query<SearchQuery>,
) -> HttpResponse {
    let service = path.into_inner();
    let term = query.term.trim();
    if term.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "missing_term" }));
    }
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };

    match arr
        .get_q::<Value>(lookup_path(&service), &[("term", term)])
        .await
    {
        Ok(data) => {
            let results: Vec<Value> = data
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .take(12)
                .map(|m| {
                    json!({
                        "id": m["id"],
                        "title": m["title"],
                        "year": m["year"],
                        "overview": m["overview"],
                        "in_library": m["id"].as_i64().unwrap_or(0) > 0,
                        "poster": poster(&m),
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "results": results }))
        }
        Err(e) => arr_error(e),
    }
}

#[derive(Deserialize)]
pub struct AddBody {
    pub term: String,
}

/// `POST /api/media/{service}/add` — add the top lookup match, monitored, and
/// trigger a search (same behaviour as the Discord commands).
pub async fn add(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
    body: web::Json<AddBody>,
) -> HttpResponse {
    let service = path.into_inner();
    let term = body.term.trim();
    if term.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "missing_term" }));
    }
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };

    let results: Value = match arr.get_q(lookup_path(&service), &[("term", term)]).await {
        Ok(v) => v,
        Err(e) => return arr_error(e),
    };
    let Some(mut item) = results.as_array().and_then(|a| a.first()).cloned() else {
        return HttpResponse::NotFound().json(json!({ "error": "no_match", "term": term }));
    };

    let title = item["title"].as_str().unwrap_or(term).to_string();
    if item["id"].as_i64().unwrap_or(0) > 0 {
        return HttpResponse::Ok().json(json!({ "added": false, "already": true, "title": title }));
    }

    let quality_profile_id = match arr.get::<Value>("qualityprofile").await {
        Ok(p) => p
            .as_array()
            .and_then(|a| a.first())
            .and_then(|p| p["id"].as_i64()),
        Err(_) => None,
    };
    let root_folder = match arr.get::<Value>("rootfolder").await {
        Ok(f) => f
            .as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["path"].as_str())
            .map(str::to_string),
        Err(_) => None,
    };
    let (Some(quality_profile_id), Some(root_folder)) = (quality_profile_id, root_folder) else {
        return HttpResponse::BadGateway()
            .json(json!({ "error": "no_profile", "message": "No quality profile / root folder configured." }));
    };

    item["qualityProfileId"] = json!(quality_profile_id);
    item["rootFolderPath"] = json!(root_folder);
    item["monitored"] = json!(true);

    let post_path = if service == "radarr" {
        item["minimumAvailability"] = json!("released");
        item["addOptions"] = json!({ "searchForMovie": true });
        "movie"
    } else {
        item["seasonFolder"] = json!(true);
        item["addOptions"] = json!({ "searchForMissingEpisodes": true, "monitor": "all" });
        // Sonarr v3 needs a language profile; v4 dropped them (best effort).
        if let Ok(p) = arr.get::<Value>("languageprofile").await {
            if let Some(lp) = p
                .as_array()
                .and_then(|a| a.first())
                .and_then(|p| p["id"].as_i64())
            {
                item["languageProfileId"] = json!(lp);
            }
        }
        "series"
    };

    match arr.post::<Value, Value>(post_path, &item).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "added": true, "title": title })),
        Err(e) => arr_error(e),
    }
}

// ── Manual import (link a finished download to the library) ─────────────────

fn import_path(state: &AppState, service: &str) -> String {
    if service == "radarr" {
        state.config.radarr_import_path.clone()
    } else {
        state.config.sonarr_import_path.clone()
    }
}

#[derive(Deserialize)]
pub struct FileQuery {
    /// File name relative to the shared downloads folder.
    pub file: String,
}

/// `GET /api/media/{service}/manual-import?file=<name>` — Sonarr/Radarr's parsed
/// candidates for a finished download, so it can be matched/imported.
pub async fn manual_import(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
    query: web::Query<FileQuery>,
) -> HttpResponse {
    let service = path.into_inner();
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };
    let import_path = import_path(&state, &service);
    let file_path = format!("{}/{}", import_path.trim_end_matches('/'), query.file);

    match arr
        .get_q::<Value>(
            "manualimport",
            &[
                ("folder", import_path.as_str()),
                ("filterExistingFiles", "false"),
            ],
        )
        .await
    {
        Ok(data) => {
            let all: Vec<Value> = data.as_array().cloned().unwrap_or_default();
            // Prefer the exact file; if nothing matches, return the whole folder.
            let exact: Vec<Value> = all
                .iter()
                .filter(|c| {
                    c["path"].as_str() == Some(file_path.as_str())
                        || c["name"].as_str() == Some(query.file.as_str())
                })
                .cloned()
                .collect();
            let candidates = if exact.is_empty() { all } else { exact };
            HttpResponse::Ok().json(json!({
                "import_path": import_path,
                "file_path": file_path,
                "candidates": candidates,
            }))
        }
        Err(e) => arr_error(e),
    }
}

#[derive(Deserialize)]
pub struct ImportBody {
    #[serde(default)]
    pub import_mode: Option<String>,
    /// Pre-built ManualImport file entries (seriesId+episodeIds, or movieId).
    pub files: Value,
}

/// `POST /api/media/{service}/manual-import` — run Sonarr/Radarr's import, which
/// renames/moves per the app's configured naming scheme.
pub async fn manual_import_confirm(
    state: web::Data<AppState>,
    _user: AuthUser,
    path: web::Path<String>,
    body: web::Json<ImportBody>,
) -> HttpResponse {
    let service = path.into_inner();
    let Some(arr) = client(&state, &service) else {
        return not_configured(&service);
    };
    let payload = json!({
        "name": "ManualImport",
        "importMode": body.import_mode.clone().unwrap_or_else(|| "move".to_string()),
        "files": body.files,
    });
    match arr.post::<Value, Value>("command", &payload).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => arr_error(e),
    }
}

#[derive(Deserialize)]
pub struct EpisodesQuery {
    pub series_id: i64,
}

/// `GET /api/media/sonarr/episodes?series_id=N` — episodes of a series, so a
/// download can be linked to a specific episode.
pub async fn sonarr_episodes(
    state: web::Data<AppState>,
    _user: AuthUser,
    query: web::Query<EpisodesQuery>,
) -> HttpResponse {
    let Some(arr) = state.sonarr.as_ref() else {
        return not_configured("sonarr");
    };
    let series_id = query.series_id.to_string();
    match arr
        .get_q::<Value>("episode", &[("seriesId", series_id.as_str())])
        .await
    {
        Ok(v) => HttpResponse::Ok().json(json!({ "episodes": v })),
        Err(e) => arr_error(e),
    }
}
