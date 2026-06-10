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
- **Phase 6 — DONE (music, before Phase 5 at user request):** songbird voice
  playback. `/music play|pause|stop|queue|volume`; sources are local files under
  `MUSIC_MOUNT_PATH` or URLs/YouTube via yt-dlp. Needs serenity `voice`+`cache`
  features and **reqwest 0.11** (songbird 0.4.6's `YoutubeDl` takes a 0.11
  Client — do NOT bump to 0.12). `Dockerfile.bot` installs cmake+libopus to build
  and ffmpeg+yt-dlp+libopus0 at runtime. Compose mounts `MUSIC_HOST_PATH`→`/music`
  read-only. Verify the bot build locally with Docker (songbird won't build on
  Windows — no cmake): `docker run --rm -v "<repo>:/app" -v nb_target:/app/target
  -v nb_cargo:/usr/local/cargo/registry -w /app rust:1-bookworm bash -c
  "apt-get update -qq && apt-get install -y -qq cmake libopus-dev pkg-config
  libssl-dev && cargo check -p bot"` (use `bash -c`, NOT `-lc` — login shell drops
  cargo from PATH).
- **Phase 6 follow-ups (DONE):** music UX — Pause/Skip/Stop buttons on the
  now-playing message (component interactions, custom_id `music_*`), `/music play`
  song autocomplete from the local library, 3-min idle auto-leave (songbird
  `Event::Periodic` watchdog), and `/autoplay` (pick voice channel + song; fires
  on `voice_state_update`, stored in `voice_triggers`). Local files are decoded
  via **ffmpeg → f32 PCM → `RawAdapter`** (songbird's symphonia ships with NO
  codecs enabled by default — the bot enables them via a direct `symphonia`
  dep with `features=["all"]`, which provides the PCM decoder).
- **Voice-bot pool (DONE):** one process runs the primary bot + extra bots from
  `DISCORD_TOKEN_2..9`. Each is its own serenity `Client` + `Songbird` (workers
  started via `register_songbird_with` on spawned tasks; only the primary
  registers commands). `BotState.pool` (`VoicePool`) picks a free bot per voice
  request so channels play simultaneously. A Discord bot can only be in ONE voice
  channel per guild — multiple bots is the only way around it. Button custom_ids
  encode the bot pool index (`music_skip:2`); control commands target the bot in
  the user's channel. Extra bots need only `bot` scope + Connect/Speak.
- **CI caching (DONE):** Dockerfiles use **cargo-chef** + GHCR registry cache
  (`cache-from/to type=registry ...:buildcache`). Dependency-only changes are
  slow once; code-only changes build in ~90s (was 20+ min). Verify Docker builds
  locally with the cached volumes `nb_target` / `nb_cargo` (see Phase 6 note).
- **Phase 5 — DONE:** `/sonarr` (TV) + `/radarr` (movies) wired to the Servarr v3
  REST API (`services/arr.rs`: base URL + `X-Api-Key`). Subcommands: status, queue,
  upcoming (calendar), search, and add (request → adds top lookup match with the
  first quality profile + root folder, monitored, triggers a search). Configured
  via `SONARR_URL`/`SONARR_API_KEY`/`RADARR_URL`/`RADARR_API_KEY`; clients live in
  `BotState.sonarr`/`.radarr`. URLs must be reachable from the bot container (LAN
  IP, not localhost). Old SonarQube-style `/sonar` `/radar` stubs removed.
- **Phase 7 — DONE (lightweight dashboard):** a self-contained single-page
  dashboard (`dashboard/index.html`, vanilla JS) **served by the API** at `/` via
  `include_str!` (no separate React app/container/build). Login with `API_KEY` →
  JWT in localStorage → shows live stats + recent activity. Added `GET /api/stats`
  (total, last_24h, top commands from `audit_log`). NOTE: `.dockerignore` must NOT
  exclude `dashboard/` (only its build artifacts) or `include_str!` fails in CI.
  Deploy = just redeploy the api image; reachable at `http://server:API_PORT/`.
- **Phase 8 — DONE (scheduling folded into the bot):** rather than separate
  scheduler/worker containers (which would duplicate the Discord token, DB, and
  arr config), the scheduling runs as a background tokio task in the bot
  (`bot/src/scheduler.rs`): every 60s it fires due `schedules` rows and prunes
  audit/log rows >30 days. `/schedule reminder|recurring|digest|list|delete`
  creates them (`schedules` table, migration 0004). Kinds: `message`,
  `digest_sonarr`, `digest_radarr`. The worker posts via `client.http` (REST).
  The `scheduler`/`worker` crates remain as workspace scaffolding. Reverse-proxy
  guidance lives in `docs/DEPLOYMENT.md` (BYO proxy — none bundled, by design).
- Phases remaining: React dashboard (upgrade the lightweight one), Docker
  hardening, CI/CD polish, docs.

## Deploy / CI cheatsheet

Push to `main` → `.github/workflows/build.yml` builds `Dockerfile.bot` → pushes
`ghcr.io/smartcile/nigerianbot-bot:latest` (GHCR package is public) → user does
"Pull and redeploy" in Portainer. Never use compose `build:` in Portainer (its
builder mis-resolves paths) and never use `cache-to: type=gha` (504s). Use the
full path to `gh`: `C:\Program Files\GitHub CLI\gh.exe`.
