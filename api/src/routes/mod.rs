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
                // --- Dashboard ---
                .route("/stats", web::get().to(handlers::stats))
                // --- Downloads ---
                .route("/downloads", web::get().to(handlers::downloads::list))
                .route("/downloads", web::post().to(handlers::downloads::create))
                .route(
                    "/downloads/{id}",
                    web::delete().to(handlers::downloads::delete),
                )
                // --- Media (Sonarr / Radarr) ---
                .route(
                    "/media/{service}/status",
                    web::get().to(handlers::media::status),
                )
                .route(
                    "/media/{service}/queue",
                    web::get().to(handlers::media::queue),
                )
                .route(
                    "/media/{service}/calendar",
                    web::get().to(handlers::media::calendar),
                )
                .route(
                    "/media/{service}/search",
                    web::get().to(handlers::media::search),
                )
                .route("/media/{service}/add", web::post().to(handlers::media::add))
                // --- Schedules ---
                .route("/schedules", web::get().to(handlers::schedules::list))
                .route("/schedules", web::post().to(handlers::schedules::create))
                .route(
                    "/schedules/{id}",
                    web::delete().to(handlers::schedules::delete),
                )
                .route(
                    "/schedules/{id}/enabled",
                    web::post().to(handlers::schedules::set_enabled),
                )
                // --- Users & roles ---
                .route("/users", web::get().to(handlers::users::list))
                .route(
                    "/users/{id}/role",
                    web::post().to(handlers::users::set_role),
                )
                // --- Telegram ---
                .route(
                    "/telegram/status",
                    web::get().to(handlers::telegram::status),
                )
                .route(
                    "/telegram/users",
                    web::get().to(handlers::telegram::list_users),
                )
                .route(
                    "/telegram/users/{id}/role",
                    web::post().to(handlers::telegram::set_role),
                )
                // --- Setup / integration status ---
                .route("/setup/status", web::get().to(handlers::setup::status))
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
