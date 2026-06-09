# NigerianBot

A production-grade Discord bot system in Rust, built as a microservices monorepo:
Discord bot, REST API, scheduler, and worker — backed by PostgreSQL and Redis,
with a React dashboard and a full Docker / GitHub Actions pipeline.

> **Status:** Phase 1 complete — project structure and a working bot skeleton.
> Features land phase by phase (see [Roadmap](#roadmap)).

## Architecture

```
┌───────────┐   ┌───────────┐   ┌─────────────┐   ┌──────────┐
│   bot     │   │    api    │   │  scheduler  │   │  worker  │
│ serenity  │   │ actix-web │   │ tokio-cron  │   │  tasks   │
└─────┬─────┘   └─────┬─────┘   └──────┬──────┘   └────┬─────┘
      └───────────────┴────────────────┴───────────────┘
                          │
              ┌───────────┴───────────┐
              │  PostgreSQL  +  Redis  │
              └────────────────────────┘
```

All four Rust services share a `common` library crate (config, telemetry, DB).

## Project layout

```
nigerian-bot/
├── Cargo.toml          # workspace + centralized dependency versions
├── common/             # shared config / telemetry / db
├── bot/                # Discord bot service        (bin: nigerian-bot)
├── api/                # REST API backend           (bin: nigerian-api)
├── scheduler/          # scheduled workflows        (bin: nigerian-scheduler)
├── worker/             # async task processing      (bin: nigerian-worker)
├── dashboard/          # React control panel        (Phase 7)
├── migrations/         # PostgreSQL migrations      (Phase 3)
├── docker-compose.yml  # local/prod stack           (Phase 9)
└── .github/workflows/  # CI/CD                       (Phase 10)
```

## Getting started

Requires Rust 1.75+ (`rustup`), and later Docker + Node 18+ for the full stack.

```bash
# 1. Configure
cp .env.example .env        # then fill in DISCORD_TOKEN (+ DISCORD_GUILD_ID for fast dev registration)

# 2. Verify the workspace builds
cargo check

# 3. Run the bot
cargo run -p bot
```

On startup the bot connects to Discord and registers its slash commands
(`/ping`, `/music`, `/sonar`, `/radar`, `/server`, `/bot`). `/ping` works today;
the rest respond with a placeholder naming the phase that implements them.

Run other services with `cargo run -p api` / `-p scheduler` / `-p worker`.

## Configuration

All configuration comes from environment variables (see `.env.example`).
Key ones: `DISCORD_TOKEN`, `DISCORD_GUILD_ID`, `DATABASE_URL`, `REDIS_URL`,
`SONAR_URL`/`SONAR_TOKEN`, `RADAR_URL`/`RADAR_TOKEN`, `JWT_SECRET`, `API_PORT`,
`RUST_LOG`.

## Roadmap

| Phase | Scope                                             | Status |
|-------|---------------------------------------------------|--------|
| 1     | Project structure + bot skeleton                  | ✅ done |
| 2     | Slash command framework & handlers                | ⬜      |
| 3     | PostgreSQL schema & migrations                    | ⬜      |
| 4     | API backend with auth (JWT + API key, RBAC)       | ⬜      |
| 5     | Sonar & Radar integrations                        | ⬜      |
| 6     | Music playback (songbird)                         | ⬜      |
| 7     | React dashboard                                   | ⬜      |
| 8     | Scheduler & worker logic                          | ⬜      |
| 9     | Docker & docker-compose                           | ⬜      |
| 10    | GitHub Actions CI/CD                              | ⬜      |
| 11    | Deployment guide & docs                           | ⬜      |

## License

MIT
