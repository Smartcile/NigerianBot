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
  connects and registers all slash command definitions; non-`ping` commands
  reply with a "not implemented (Phase N)" placeholder.
- Phases 2–11: see the project plan (commands, migrations, API auth, Sonar/Radar,
  music, dashboard, scheduler/worker logic, Docker, CI/CD, docs).
