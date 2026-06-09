//! HTTP request handlers.

use actix_web::{HttpResponse, Responder};
use serde_json::json;

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
