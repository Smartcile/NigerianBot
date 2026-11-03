//! Compose builder: CRUD for saved projects + palette defaults, server-side YAML
//! rendering, and publishing to the directory Dockhand watches.

use std::path::Path;

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::auth::{AdminUser, AuthUser};
use crate::state::AppState;
use common::compose::{self, ProjectModel};

#[derive(Serialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub model: serde_json::Value,
    pub rendered_yaml: Option<String>,
    pub published_path: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

const PROJECT_COLUMNS: &str = "id, name, description, model, rendered_yaml, published_path, \
                               published_at, created_by, created_at, updated_at";

#[derive(Serialize)]
pub struct ComposeFileEntry {
    pub name: String,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// `GET /api/compose/files` — read-only list of `.yml`/`.yaml` files already in
/// `COMPOSE_DIR` (e.g. files Dockhand is watching).
pub async fn list_files(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let dir = Path::new(&state.config.compose_dir);
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return HttpResponse::Ok().json(json!({
                "dir": state.config.compose_dir,
                "count": 0,
                "files": [],
            }))
        }
        Err(e) => {
            error!(?e, "failed to read compose dir");
            return HttpResponse::InternalServerError().json(json!({ "error": "read_error" }));
        }
    };

    let mut files = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if !entry
            .file_type()
            .await
            .map(|t| t.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(ext, "yml" | "yaml") {
            continue;
        }
        let meta = entry.metadata().await.ok();
        files.push(ComposeFileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: meta
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<chrono::Utc>::from),
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    HttpResponse::Ok().json(json!({
        "dir": state.config.compose_dir,
        "count": files.len(),
        "files": files,
    }))
}

#[derive(Deserialize)]
pub struct SaveProject {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: ProjectModel,
}

#[derive(Deserialize)]
pub struct RenderRequest {
    pub name: String,
    #[serde(default)]
    pub model: ProjectModel,
}

/// `GET /api/compose/projects` — saved projects.
pub async fn list_projects(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let rows = sqlx::query_as::<_, ProjectRow>(&format!(
        "SELECT {PROJECT_COLUMNS} FROM compose_projects ORDER BY name"
    ))
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "projects": rows })),
        Err(e) => db_error(e),
    }
}

/// `GET /api/compose/projects/{id}`
pub async fn get_project(
    state: web::Data<AppState>,
    _user: AuthUser,
    id: web::Path<i64>,
) -> impl Responder {
    let row = sqlx::query_as::<_, ProjectRow>(&format!(
        "SELECT {PROJECT_COLUMNS} FROM compose_projects WHERE id = $1"
    ))
    .bind(id.into_inner())
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => not_found(),
        Err(e) => db_error(e),
    }
}

/// `POST /api/compose/projects` — create a project.
pub async fn create_project(
    state: web::Data<AppState>,
    admin: AdminUser,
    body: web::Json<SaveProject>,
) -> impl Responder {
    let mut model = body.model.clone();
    compose::sanitize(&mut model);
    let yaml = match compose::render(&body.name, &model) {
        Ok(yaml) => yaml,
        Err(e) => return invalid_model(&e),
    };
    let model_json = match serde_json::to_value(&model) {
        Ok(value) => value,
        Err(e) => {
            error!(?e, "failed to serialize compose model");
            return HttpResponse::InternalServerError().json(json!({ "error": "serialize_error" }));
        }
    };

    let row = sqlx::query_as::<_, ProjectRow>(&format!(
        "INSERT INTO compose_projects (name, description, model, rendered_yaml, created_by) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {PROJECT_COLUMNS}"
    ))
    .bind(&body.name)
    .bind(&body.description)
    .bind(model_json)
    .bind(&yaml)
    .bind(&admin.claims.sub)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(row) => HttpResponse::Created().json(row),
        Err(e) => db_error(e),
    }
}

/// `PUT /api/compose/projects/{id}` — replace a project's name/description/model.
pub async fn update_project(
    state: web::Data<AppState>,
    _admin: AdminUser,
    id: web::Path<i64>,
    body: web::Json<SaveProject>,
) -> impl Responder {
    let mut model = body.model.clone();
    compose::sanitize(&mut model);
    let yaml = match compose::render(&body.name, &model) {
        Ok(yaml) => yaml,
        Err(e) => return invalid_model(&e),
    };
    let model_json = match serde_json::to_value(&model) {
        Ok(value) => value,
        Err(e) => {
            error!(?e, "failed to serialize compose model");
            return HttpResponse::InternalServerError().json(json!({ "error": "serialize_error" }));
        }
    };

    let row = sqlx::query_as::<_, ProjectRow>(&format!(
        "UPDATE compose_projects \
         SET name = $2, description = $3, model = $4, rendered_yaml = $5, updated_at = now() \
         WHERE id = $1 RETURNING {PROJECT_COLUMNS}"
    ))
    .bind(id.into_inner())
    .bind(&body.name)
    .bind(&body.description)
    .bind(model_json)
    .bind(&yaml)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => not_found(),
        Err(e) => db_error(e),
    }
}

/// `DELETE /api/compose/projects/{id}`
pub async fn delete_project(
    state: web::Data<AppState>,
    _admin: AdminUser,
    id: web::Path<i64>,
) -> impl Responder {
    match sqlx::query("DELETE FROM compose_projects WHERE id = $1")
        .bind(id.into_inner())
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => not_found(),
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => db_error(e),
    }
}

/// `POST /api/compose/projects/{id}/publish` — write the YAML (and a blank
/// `.env` scaffold for secret keys) into `COMPOSE_DIR`.
pub async fn publish_project(
    state: web::Data<AppState>,
    _admin: AdminUser,
    id: web::Path<i64>,
) -> impl Responder {
    let id = id.into_inner();
    let row = match sqlx::query_as::<_, ProjectRow>(&format!(
        "SELECT {PROJECT_COLUMNS} FROM compose_projects WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return not_found(),
        Err(e) => return db_error(e),
    };

    let model: ProjectModel = match serde_json::from_value(row.model.clone()) {
        Ok(model) => model,
        Err(e) => {
            error!(?e, "stored compose model is invalid");
            return HttpResponse::InternalServerError()
                .json(json!({ "error": "stored_model_invalid" }));
        }
    };
    let yaml = match compose::render(&row.name, &model) {
        Ok(yaml) => yaml,
        Err(e) => return invalid_model(&e),
    };

    let dir = Path::new(&state.config.compose_dir);
    if let Err(e) = tokio::fs::create_dir_all(dir).await {
        error!(?e, "failed to create compose dir");
        return HttpResponse::InternalServerError().json(json!({ "error": "compose_dir_error" }));
    }

    let path = dir.join(format!("{}.yml", row.name));
    let tmp = dir.join(format!(".{}.yml.tmp", row.name));
    if let Err(e) = tokio::fs::write(&tmp, yaml.as_bytes()).await {
        error!(?e, "failed to write compose file");
        return HttpResponse::InternalServerError().json(json!({
            "error": "write_error",
            "message": "Could not write to the compose folder. On the server run: \
                        chown 10001:10001 <COMPOSE_HOST_PATH>",
        }));
    }
    if let Err(e) = tokio::fs::rename(&tmp, &path).await {
        error!(?e, "failed to move compose file into place");
        let _ = tokio::fs::remove_file(&tmp).await;
        return HttpResponse::InternalServerError().json(json!({
            "error": "write_error",
            "message": "Could not write to the compose folder. On the server run: \
                        chown 10001:10001 <COMPOSE_HOST_PATH>",
        }));
    }

    // Never overwrite an existing .env — it holds the real secret values.
    let env_path = dir.join(format!("{}.env", row.name));
    if !env_path.exists() {
        let keys = compose::secret_keys(&model);
        if !keys.is_empty() {
            let mut scaffold = String::new();
            for key in keys {
                scaffold.push_str(&key);
                scaffold.push_str("=\n");
            }
            if let Err(e) = tokio::fs::write(&env_path, scaffold).await {
                error!(?e, "failed to write env scaffold");
            }
        }
    }

    let published_path = path.to_string_lossy().to_string();
    let updated = sqlx::query_as::<_, ProjectRow>(&format!(
        "UPDATE compose_projects \
         SET published_path = $2, published_at = now(), updated_at = now() \
         WHERE id = $1 RETURNING {PROJECT_COLUMNS}"
    ))
    .bind(id)
    .bind(&published_path)
    .fetch_one(&state.db)
    .await;

    match updated {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => db_error(e),
    }
}

/// `POST /api/compose/render` — render YAML for the live preview without saving.
pub async fn render_preview(
    _state: web::Data<AppState>,
    _user: AuthUser,
    body: web::Json<RenderRequest>,
) -> impl Responder {
    match compose::render(&body.name, &body.model) {
        Ok(yaml) => HttpResponse::Ok().json(json!({ "yaml": yaml })),
        Err(e) => invalid_model(&e),
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DefaultRow {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub definition: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

const DEFAULT_COLUMNS: &str = "id, kind, name, definition, created_at, updated_at";

#[derive(Deserialize)]
pub struct SaveDefault {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub definition: serde_json::Value,
}

/// `GET /api/compose/defaults` — reusable palette entries.
pub async fn list_defaults(state: web::Data<AppState>, _user: AuthUser) -> impl Responder {
    let rows = sqlx::query_as::<_, DefaultRow>(&format!(
        "SELECT {DEFAULT_COLUMNS} FROM compose_defaults ORDER BY kind, name"
    ))
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "count": rows.len(), "defaults": rows })),
        Err(e) => db_error(e),
    }
}

/// `POST /api/compose/defaults` — create/update a palette entry.
pub async fn create_default(
    state: web::Data<AppState>,
    _admin: AdminUser,
    body: web::Json<SaveDefault>,
) -> impl Responder {
    if !matches!(body.kind.as_str(), "bind_mount" | "network" | "env_file") {
        return HttpResponse::BadRequest().json(json!({ "error": "invalid_kind" }));
    }
    let name = body.name.trim();
    if name.is_empty() || name.len() > 80 || name.contains('\n') || name.contains('\r') {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_name",
            "message": "Name must be 1-80 characters on one line.",
        }));
    }
    if !body.definition.is_object() {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_definition",
            "message": "Definition must be a JSON object.",
        }));
    }

    let definition_ok = match body.kind.as_str() {
        "bind_mount" => ["source", "target"].iter().all(|key| {
            body.definition
                .get(*key)
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.trim().is_empty())
        }),
        "env_file" => body
            .definition
            .get("keys")
            .and_then(|v| v.as_array())
            .is_some_and(|keys| {
                !keys.is_empty()
                    && keys
                        .iter()
                        .all(|v| v.as_str().is_some_and(|k| !k.trim().is_empty()))
            }),
        _ => true,
    };
    if !definition_ok {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_definition",
            "message": "Bind mounts need both paths; env-file defaults need at least one key.",
        }));
    }

    let row = sqlx::query_as::<_, DefaultRow>(&format!(
        "INSERT INTO compose_defaults (kind, name, definition) VALUES ($1, $2, $3) \
         ON CONFLICT (kind, name) DO UPDATE \
         SET definition = EXCLUDED.definition, updated_at = now() \
         RETURNING {DEFAULT_COLUMNS}"
    ))
    .bind(&body.kind)
    .bind(name)
    .bind(&body.definition)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(row) => HttpResponse::Ok().json(row),
        Err(e) => db_error(e),
    }
}

/// `DELETE /api/compose/defaults/{id}`
pub async fn delete_default(
    state: web::Data<AppState>,
    _admin: AdminUser,
    id: web::Path<i64>,
) -> impl Responder {
    match sqlx::query("DELETE FROM compose_defaults WHERE id = $1")
        .bind(id.into_inner())
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => not_found(),
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => db_error(e),
    }
}

fn not_found() -> HttpResponse {
    HttpResponse::NotFound().json(json!({ "error": "not_found" }))
}

fn invalid_model(e: &anyhow::Error) -> HttpResponse {
    HttpResponse::BadRequest().json(json!({
        "error": "invalid_model",
        "message": e.to_string(),
    }))
}

fn db_error(e: sqlx::Error) -> HttpResponse {
    let unique = e
        .as_database_error()
        .map(|d| d.is_unique_violation())
        .unwrap_or(false);
    if unique {
        return HttpResponse::Conflict().json(json!({
            "error": "duplicate",
            "message": "A record with that name already exists.",
        }));
    }
    error!(?e, "compose database error");
    HttpResponse::InternalServerError().json(json!({ "error": "db_error" }))
}
