# 🎵 NigerianBot

A **production-grade Discord bot system written in Rust** — a microservices
monorepo with a full-featured music engine, Sonarr/Radarr media requests, a
React dashboard, PostgreSQL persistence, a JWT-secured REST API, and a
push-to-deploy CI/CD pipeline. Built to run 24/7 in Docker.

![Build](https://github.com/Smartcile/NigerianBot/actions/workflows/build.yml/badge.svg)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![React](https://img.shields.io/badge/React-18-61DAFB?logo=react)
![Postgres](https://img.shields.io/badge/PostgreSQL-16-blue?logo=postgresql)
![Docker](https://img.shields.io/badge/Docker-ready-2496ED?logo=docker)
![License](https://img.shields.io/badge/license-MIT-green)

---

## ✨ Highlights

- 🎧 **Full music engine** — play from a mounted library *or* YouTube/URLs, with a
  live queue, volume, and **Pause / Skip / Stop buttons** on the now-playing
  message.
- 🔎 **Smart autocomplete** — start typing `/music play` and your local library is
  searched live, subfolders and all.
- 🔐 **Discord DAVE / E2EE voice** — speaks Discord's end-to-end-encrypted voice
  protocol (mandatory since March 2026), so voice actually connects.
- 🤖 **Multi-bot voice pool** — run several bot identities from one process so
  **multiple channels play audio at once** (Discord allows one voice channel per
  bot, so NigerianBot runs a self-healing pool).
- 🎯 **`/autoplay`** & 🔔 **`/joinsound`** — auto-play when someone joins a channel
  (stay), or one-shot entrance sounds (join → play → leave). ⏳ Idle auto-leave.
- 🎬 **Media requests** — `/sonarr` (TV) & `/radarr` (movies): status, queue,
  upcoming, search, and **one-command requests** that add to your library and
  start downloading.
- 🗓️ **Scheduler** — `/schedule` reminders, recurring announcements, and
  Sonarr/Radarr download digests; auto-prunes old logs.
- 📊 **React dashboard** — log in with your API key for live stats, top commands,
  and recent activity (served by the API — bring your own HTTPS).
- 🗄️ **Persistence & audit** — every command is logged to PostgreSQL; settings,
  queues, triggers, and schedules survive restarts.
- 🚀 **Push-to-deploy** — `git push` → GitHub Actions builds images → your server
  pulls them. Code-only builds are cached down to **~90 seconds** with `cargo-chef`.

---

## 🏗️ Architecture

A Cargo workspace of services that share a `common` library and one database:

```
                         ┌──────────────────────────────┐
        Discord  ◀──────▶│  bot  (serenity + songbird)  │
        (gateway,        │  · voice-bot pool            │
         voice, DAVE)    │  · background scheduler      │◀──▶ Sonarr / Radarr
                         └───────────────┬──────────────┘
                                         │
   Browser ──▶ React dashboard ──▶ ┌─────┴────────┐   ┌────────────────────┐
                                   │  api          │   │  PostgreSQL 16     │
                                   │  (actix-web,  │◀─▶│  audit · settings  │
                                   │   JWT, serves │   │  queue · triggers  │
                                   │   the SPA)    │   │  schedules         │
                                   └───────────────┘   └────────────────────┘
```

| Service       | Stack                | Role                                          |
|---------------|----------------------|-----------------------------------------------|
| **bot**       | serenity, songbird   | Discord bot, voice, media, in-process scheduler |
| **api**       | actix-web + React    | REST API (JWT) + serves the dashboard SPA     |
| **common**    | (library)            | Shared config, telemetry, DB pool             |
| scheduler / worker | tokio           | Workspace scaffolding (scheduling runs in the bot) |

---

## 🎛️ Commands

| Command | What it does |
|---------|--------------|
| `/music play <song\|url>` | Play from your library (autocompletes) or a URL — with buttons |
| `/music pause · stop · queue · volume` | Playback controls |
| `/autoplay set·clear·list` | Auto-play when someone joins a channel (stays) |
| `/joinsound set·clear·list` | Entrance sound when someone joins, then leave |
| `/sonarr status·queue·upcoming·search·add` | Sonarr (TV): browse & request shows |
| `/radarr status·queue·upcoming·search·add` | Radarr (movies): browse & request films |
| `/schedule reminder·recurring·digest·list·delete` | Schedule messages & media digests |
| `/server info` · `/bot status` · `/ping` | Server stats · bot health · health check |

---

## 🧰 Tech stack

**Rust (2021)** · Bot: [serenity](https://github.com/serenity-rs/serenity) +
[songbird](https://github.com/serenity-rs/songbird) · API: [actix-web](https://actix.rs/) ·
Dashboard: **React + Vite** · DB: PostgreSQL + [sqlx](https://github.com/launchbadge/sqlx) ·
Audio: ffmpeg + yt-dlp · Auth: JWT · CI/CD: GitHub Actions → GHCR · Deploy: Docker
Compose / Portainer.

---

## ⚙️ How it's deployed

1. Push to `main`.
2. **GitHub Actions** builds the `bot` and `api` images (cargo-chef + registry
   cache keep it fast; the API image also builds the React dashboard) and pushes
   them to the **GitHub Container Registry**.
3. The server (e.g. **Portainer**) pulls the prebuilt images — no compiling on the
   host. The dashboard is at `http://<server>:8000/`.

The whole stack is defined in [`docker-compose.yml`](docker-compose.yml). Secrets
are injected as environment variables and never committed. The app serves **plain
HTTP** — bring your own reverse proxy / TLS (see
**[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)**). The Discord bot is outbound-only and
needs no inbound ports.

```bash
# Local development
cp .env.example .env        # fill in DISCORD_TOKEN, etc.
cargo run -p bot            # or -p api
```

---

## 🗺️ Status

| Area | Status |
|------|:------:|
| Workspace, slash-command framework, embeds | ✅ |
| PostgreSQL schema, auto-migrations, audit log | ✅ |
| REST API with JWT auth | ✅ |
| Music engine (DAVE, queue, buttons, autoplay, joinsound, voice pool) | ✅ |
| Sonarr + Radarr integrations (incl. requests) | ✅ |
| React dashboard | ✅ |
| Scheduler (reminders, recurring, media digests, housekeeping) | ✅ |
| CI/CD with cargo-chef caching | ✅ |
| HTTPS | bring your own reverse proxy ([docs](docs/DEPLOYMENT.md)) |

---

## 🔒 Engineering notes

- **Zero unsafe**, type-safe end to end. Errors carry context via `anyhow`.
- Migrations embedded into the binaries and applied on startup behind an advisory
  lock (safe for concurrent services); services retry the DB while it warms up.
- Local files are decoded via **ffmpeg → raw PCM**, so any format/codec and messy
  metadata tags "just work" without crashing the decoder.
- The voice pool is **self-healing**: a bot that fails to join is freed instead of
  getting stuck, and requests try every free bot.

---

## 📄 License

MIT — see [`Cargo.toml`](Cargo.toml).

> Built with Rust 🦀 and a lot of Discord voice-protocol spelunking.
