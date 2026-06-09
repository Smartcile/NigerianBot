//! Route table for the API. Mirrors the endpoint plan from the project spec so
//! the dashboard contract is visible from day one; handlers fill in per phase.

use actix_web::web;

use crate::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(handlers::health))
        .service(
            web::scope("/api")
                // --- Authentication ---
                .route("/auth/login", web::post().to(handlers::login))
                .route("/auth/refresh", web::post().to(handlers::refresh))
                // --- Bot control ---
                .route("/bot/status", web::get().to(handlers::bot_status))
                .route("/bot/logs", web::get().to(handlers::bot_logs))
                .route(
                    "/bot/command/{command}",
                    web::post().to(handlers::not_implemented),
                )
                .route("/bot/settings", web::post().to(handlers::not_implemented))
                // --- Workflows (Phase 8) ---
                .route("/workflows", web::get().to(handlers::not_implemented))
                .route(
                    "/workflows/{id}/execute",
                    web::post().to(handlers::not_implemented),
                )
                .route(
                    "/workflows/{id}/status",
                    web::get().to(handlers::not_implemented),
                )
                // --- Service proxies (Phase 5/6) ---
                .route(
                    "/services/sonar/{project}",
                    web::get().to(handlers::not_implemented),
                )
                .route(
                    "/services/radar/{service}",
                    web::get().to(handlers::not_implemented),
                )
                .route(
                    "/services/music/queue",
                    web::get().to(handlers::not_implemented),
                )
                .route(
                    "/services/music/play",
                    web::post().to(handlers::not_implemented),
                ),
        );
}
