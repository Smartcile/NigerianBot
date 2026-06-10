# 🎵 NigerianBot

A **production-grade Discord bot system written in Rust** — a microservices
monorepo with a full-featured music engine, PostgreSQL persistence, a JWT-secured
REST API, and a push-to-deploy CI/CD pipeline. Built to run 24/7 in Docker.

![Build](https://github.com/Smartcile/NigerianBot/actions/workflows/build.yml/badge.svg)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![Postgres](https://img.shields.io/badge/PostgreSQL-16-blue?logo=postgresql)
![Docker](https://img.shields.io/badge/Docker-ready-2496ED?logo=docker)
![License](https://img.shields.io/badge/license-MIT-green)

---

## ✨ Highlights

A serious music bot, plus the infrastructure to run it like a real service:

- 🎧 **Full music engine** — play from a mounted music library *or* YouTube/URLs,
  with a live queue, volume control, and **interactive Pause / Skip / Stop
  buttons** right on the now-playing message.
- 🔎 **Smart autocomplete** — start typing `/music play` and your local library
  is searched live, subfolders and all.
- 🔐 **Discord DAVE / E2EE voice** — speaks Discord's new end-to-end-encrypted
  voice protocol (mandatory since March 2026), so voice actually connects in 2026.
- 🤖 **Multi-bot voice pool** — run several bot identities from one process so
  **multiple channels can play audio simultaneously** (Discord only allows one
  voice channel per bot — so NigerianBot runs a pool).
- 🎯 **`/autoplay`** — when someone joins a chosen voice channel, the bot hops in
  and plays a track (and stays).
- 🔔 **`/joinsound`** — entrance sounds: join → play a clip → leave.
- ⏳ **Idle auto-leave** — frees a bot after 3 minutes of silence.
- 🗄️ **Persistence & audit** — every command is logged to PostgreSQL; settings,
  queues, and triggers survive restarts.
- 🚀 **Push-to-deploy** — `git push` → GitHub Actions builds images → your server
  pulls them. Builds are cached down to **~90 seconds** with `cargo-chef`.

---

## 🏗️ Architecture

A Cargo workspace of independent services that share a `common` library and one
database:

```
                         ┌──────────────────────────┐
        Discord  ◀──────▶│   bot  (serenity +       │
        (gateway,        │        songbird voice)   │
         voice, DAVE)    │   + voice-bot pool       │
                         └────────────┬─────────────┘
                                      │
   Dashboard / API  ◀───┐            │
   clients              ▼            ▼
                ┌───────────────┐  ┌──────────────────────┐
                │  api          │  │  PostgreSQL 16        │
                │  (actix-web,  │◀▶│  logs · settings ·    │
                │   JWT auth)   │  │  queue · audit ·      │
                └───────────────┘  │  triggers             │
                ┌───────────────┐  └──────────────────────┘
                │  scheduler /  │            ▲
                │  worker       │────────────┘
                └───────────────┘
```

| Service       | Stack            | Role                                            |
|---------------|------------------|-------------------------------------------------|
| **bot**       | serenity, songbird | Discord bot — slash commands, events, voice    |
| **api**       | actix-web        | REST API (JWT auth) for the dashboard/control   |
| **scheduler** | tokio-cron       | Scheduled workflows *(scaffolded)*              |
| **worker**    | tokio            | Async task processing *(scaffolded)*            |
| **common**    | (library)        | Shared config, telemetry, DB pool               |

---

## 🎛️ Commands

| Command | What it does |
|---------|--------------|
| `/music play <song\|url>` | Play from your library (autocompletes) or a URL — with control buttons |
| `/music pause · stop · queue · volume` | Playback controls |
| `/autoplay set·clear·list` | Auto-play a track when someone joins a chosen voice channel (stays) |
| `/joinsound set·clear·list` | Play an entrance sound when someone joins, then leave |
| `/server info` | Live server stats (members, roles, channels, owner…) |
| `/bot status` | Bot health: uptime, version, DB status, voice-bot count, commands logged |
| `/ping` | Health check |
| `/sonar` · `/radar` | Code-quality & monitoring integrations *(coming soon)* |

---

## 🧰 Tech stack

**Language:** Rust (2021) · **Bot:** [serenity](https://github.com/serenity-rs/serenity) +
[songbird](https://github.com/serenity-rs/songbird) · **API:** [actix-web](https://actix.rs/) ·
**DB:** PostgreSQL + [sqlx](https://github.com/launchbadge/sqlx) · **Audio:** ffmpeg + yt-dlp ·
**Auth:** JWT · **CI/CD:** GitHub Actions → GHCR · **Deploy:** Docker Compose / Portainer.

---

## ⚙️ How it's deployed

1. Push to `main`.
2. **GitHub Actions** builds the `bot` and `api` images (cargo-chef + registry
   cache keep it fast) and pushes them to the **GitHub Container Registry**.
3. The server (running **Portainer**) pulls the prebuilt images — no compiling on
   the host.

The whole stack — bot, API, and PostgreSQL — is defined in
[`docker-compose.yml`](docker-compose.yml). Secrets (Discord token, DB password,
API keys) are injected as environment variables and never committed.

The app serves **plain HTTP** and expects you to bring your own reverse proxy /
TLS (Nginx Proxy Manager, Caddy, Traefik, Cloudflare Tunnel…) — see
**[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)**. The Discord bot itself is
outbound-only and needs no inbound ports at all.

```bash
# Local development
cp .env.example .env        # fill in DISCORD_TOKEN, etc.
cargo run -p bot            # or -p api / -p scheduler / -p worker
```

---

## 🗺️ Roadmap

| Phase | Scope | Status |
|------:|-------|:------:|
| 1 | Workspace + bot skeleton | ✅ |
| 2 | Slash-command framework, embeds | ✅ |
| 3 | PostgreSQL schema + auto-migrations + audit log | ✅ |
| 4 | REST API with JWT auth | ✅ |
| 6 | Music engine (DAVE, queue, buttons, autoplay, joinsound, multi-bot pool) | ✅ |
| — | CI/CD with cargo-chef caching | ✅ |
| 5 | Sonar + Radar integrations | ⬜ |
| 7 | React dashboard | ⬜ |
| 8 | Scheduler & worker logic | ⬜ |
| 9 | Nginx reverse proxy + HTTPS | ⬜ |

---

## 🔒 A few engineering notes

- **Zero unsafe**, type-safe end to end. Errors carry context via `anyhow`.
- **One shared schema**, migrations embedded into the binaries and applied on
  startup behind an advisory lock (safe for concurrent services).
- **Resilient startup** — services retry the database while it warms up.
- **Local files decoded via ffmpeg → raw PCM**, so any format/codec and messy
  metadata tags "just work" without crashing the decoder.

---

## 📄 License

MIT — see [`Cargo.toml`](Cargo.toml).

> Built with Rust 🦀 and a lot of Discord voice-protocol spelunking.
