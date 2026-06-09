# NigerianBot — Claude Code Project Context

Production-grade Discord bot system in Rust, built as a microservices monorepo
with a React dashboard, PostgreSQL, Docker, and GitHub Actions CI/CD.

## Workspace layout

A single Cargo workspace (`Cargo.toml` at the root) with these member crates:

| Crate       | Binary                | Role                                                       |
|-------------|-----------------------|------------------------------------------------------------|
| `common`    | (lib)                 | Shared config, telemetry, and DB pool helpers              |
| `bot`       | `nigerian-bot`        | Discord bot (serenity) — slash commands & events           |
| `api`       | `nigerian-api`        | REST API (actix-web) for the dashboard & control plane     |
| `scheduler` | `nigerian-scheduler`  | Cron-style scheduled workflows (tokio-cron-scheduler)      |
| `worker`    | `nigerian-worker`     | Async task / external-API processing                       |

Non-Rust pieces: `dashboard/` (React, Phase 7), `migrations/` (SQL, Phase 3),
`docker-compose.yml` (Phase 9), `.github/workflows/` (Phase 10).

## Conventions

- **Edition 2021.** Dependency versions are centralized in
  `[workspace.dependencies]`; member crates use `dep.workspace = true`. Add or
  bump versions in the root `Cargo.toml`, not in member crates.
- **`common` crate first.** Don't duplicate env-loading, tracing init, or the
  Postgres pool in a service — add/extend it in `common` and call it.
- **Config from env** via `common::config` (`require`, `optional`,
  `optional_or`). Each service has its own typed `Config::from_env()`.
- **Logging** with `tracing`; initialize once per process with
  `common::telemetry::init("<service>")`. Honors `RUST_LOG`.
- **Errors:** `anyhow::Result` at boundaries; prefer `.context(...)`.
- Slash commands: one module per feature in `bot/src/commands/`, each exposing
  `definition() -> CreateCommand` and `handle(...)`. Register in
  `commands::all_definitions()` and route in `commands::dispatch()`.

## Build & run

```bash
cargo check                       # type-check the whole workspace
cargo clippy --all-targets        # lint
cargo fmt                         # format
cargo run -p bot                  # run a single service (needs DISCORD_TOKEN)
```

Copy `.env.example` to `.env` and fill in secrets before running locally.

## Deliberate deviations from the original spec

- Added a shared **`common`** crate (spec had per-service `config.rs` only) to
  avoid copy-pasting infrastructure across four services.
- Uses **`dotenvy`** (maintained) instead of the unmaintained `dotenv`.
- **`songbird`** (music) is commented out in `bot/Cargo.toml` until Phase 6 — its
  default `driver` feature needs libopus/native deps that would otherwise weigh
  down the very first build.

## Phase status

- **Phase 1 — DONE:** workspace, four service skeletons, `common` crate, bot
  connects and registers slash commands. Verified live on the user's server.
- **Phase 2 — DONE:** command framework — shared `BotState` in `ctx.data`, embed/
  ephemeral helpers, error-catching dispatcher, live `/bot status` & `/server info`.
  Reused-bot fix: clears stale global commands when a guild is configured.
- **Phase 3 — DONE:** PostgreSQL. `migrations/0001_init.sql` (7 tables). Bot
  connects on startup (retry) and runs `sqlx::migrate!` automatically; pool lives
  in `BotState.db`. Every command is audit-logged; `/bot status` shows DB health +
  logged count. Postgres added to the Portainer stack (compose). Note: queries use
  runtime `sqlx::query` (not the `query!` macro) so CI builds need no live DB; the
  Dockerfile copies `migrations/` so `migrate!` can embed them.
- **Phase 4 — DONE:** API backend (actix-web). Connects to the same Postgres
  (+ runs migrations), `POST /api/auth/login` exchanges `API_KEY` for a JWT,
  `AuthUser` extractor guards `/api/*`, `POST /api/auth/refresh`,
  `GET /api/bot/status` and `GET /api/bot/logs` (reads `audit_log`). New
  `Dockerfile.api` + CI matrix builds both `nigerianbot-bot` and `nigerianbot-api`
  on GHCR. `api` service added to the stack, exposed on port 8000. The
  `nigerianbot-api` GHCR package must be made public once (like the bot did).
- Phases 5–11: Sonar/Radar, music, dashboard, scheduler/worker logic, Docker
  hardening, CI/CD polish, docs.

## Deploy / CI cheatsheet

Push to `main` → `.github/workflows/build.yml` builds `Dockerfile.bot` → pushes
`ghcr.io/smartcile/nigerianbot-bot:latest` (GHCR package is public) → user does
"Pull and redeploy" in Portainer. Never use compose `build:` in Portainer (its
builder mis-resolves paths) and never use `cache-to: type=gha` (504s). Use the
full path to `gh`: `C:\Program Files\GitHub CLI\gh.exe`.
