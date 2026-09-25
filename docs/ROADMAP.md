# Roadmap — the "toolbelt"

NigerianBot is growing into a personal, self-hosted **toolbelt**: one core (Rust +
Postgres) with several faces you can operate from. This doc is the running plan
for that — it complements [`VISION.md`](VISION.md) (the long-term platform goal)
and [`IDEAS.md`](IDEAS.md) (loose backlog). Items are marked:

- ✅ **shipped** — in the codebase now
- 🚧 **in progress** — being built
- ⬜ **planned** — not started

## The toolbelt at a glance

```
                 ┌──────────────── one core (Postgres + service clients) ───────────────┐
   Discord  ──▶  │  bot        commands · voice · scheduler                          │
   Telegram ──▶  │  telegram   commands · notifications                              │
   Web GUI  ──▶  │  api        REST + React dashboard (admin/control)                │
                 │  worker     background jobs: downloads, transcode, notify         │
                 │  postgres   identity · audit · queues · history                   │
                 └───────────────────────────────────────────────────────────────────┘
```

Everything the toolbelt *does* is queued in Postgres and executed by the
**worker**, so any face (Discord, Telegram, web) can trigger it and every face can
see the result.

## Track 1 — Downloader (Vimeo, YouTube, any yt-dlp URL)

Save a URL's video to server storage; link it from Discord/Telegram/web.

- ✅ **Download queue + worker pipeline.** `downloads` table (migration 0006);
  the `worker` service claims queued rows, runs `yt-dlp` into `DOWNLOADS_PATH`,
  records title/size/path or the error.
- ✅ **Web GUI.** Dashboard → Downloads: paste a URL, watch status, open the file.
- ✅ **Discord `/download <url>`** and **Telegram `/download <url>`**.
- ✅ **Notifications** when a download finishes (Discord webhook + Telegram chat).
- ⬜ **Audio extraction** into the music library (toggle: video / audio / both).
- ⬜ **Format & quality chooser** (max height, container, subtitles) from the GUI.
- ⬜ **Playlists / channels**, per-item rows, and download history/retention.
- ⬜ **Vimeo specifics**: password'd / embed-only videos via cookies file.

## Track 2 — Telegram surface

Use the same toolbelt from Telegram.

- ✅ **Commands**: `/start`, `/help`, `/status`, `/download <url>`, `/downloads`,
  `/whoami`.
- ✅ **Notifications**: download completed/failed pushed to `TELEGRAM_CHAT_ID`.
- ⬜ **Account linking**: bind a Telegram id to a Discord/platform `users` row so a
  person has one role/identity across faces (needs the unified identity model).
- ⬜ **More commands**: media requests (`/sonarr`, `/radarr`), schedules, music
  links, admin role management.
- ⬜ **Inline confirmations** (buttons) instead of plain text replies.

## Track 3 — Web control plane (admin GUI)

The dashboard is the "settings and control" face. Today it exposes the whole
toolbelt, wrapped in a parody **"HONOURABLE BUSINESS" 419 skin** (cosmetic only —
all data/actions are real).

- ✅ **Downloads page** (list, add, cancel/remove, open file).
- ✅ **Media panel** — pick Sonarr or Radarr; see version/library/queue, browse the
  calendar, search the catalog, and **request** a title (adds + searches).
- ✅ **Schedules manager** — list, create, pause/resume, and delete `schedules`
  rows from the browser.
- ✅ **Users & roles** — the Discord `users` directory and the Telegram
  `telegram_users` directory, with role editing.
- ✅ **Telegram panel** — config status, notify chat id, and subscribers.
- ✅ **Setup page** — per-service env vars, connection steps, and usage, with live
  status pills read from `GET /api/setup/status` (presence only; no secrets).
- ⬜ **Media-request approvals** — pending `/sonarr`/`/radarr` requests an admin
  approves (currently adds immediately).
- ⬜ **Voice control** — see the voice pool, who's playing what, transport controls.
- ⬜ **Settings** — per-guild defaults (volume, allowed channels, request limits),
  managed service credentials, log retention.
- ⬜ **Real RBAC in the UI** — viewer vs admin, using the JWT `role` claim.
- ⬜ **Live updates** — SSE/WebSocket instead of polling.

## Track 4 — Platform foundation

The generalising work from [`VISION.md`](VISION.md).

- ⬜ **Unified identity** — one `identities` model linking Discord ↔ Telegram ↔
  future SSO (Authentik), with roles, replacing per-surface id tables.
- ✅ **Shared service layer** — the `Arr` (Sonarr/Radarr) connector now lives in
  `common::arr`; the bot re-exports it and the API uses it directly.
- ⬜ **Generic job queue** — generalise `downloads` into a `jobs` table so any
  surface can enqueue any background task.
- ⬜ **Auth**: Authentik OIDC alongside the API-key path.
- ⬜ **Ops**: Prometheus `/metrics`, error alerts to an admin channel.

## Track 5 — Testing & quality

Today there are **no tests anywhere** and CI only builds images. For each language
the runner is the obvious one — they cover different code, not each other:

- **Rust → `cargo test`.** Built into the toolchain (no config). Unit tests inline
  in `#[cfg(test)]` modules; integration tests in each crate's `tests/`. Add
  crates only as needed: `insta` (snapshots), `mockall` (mocks), `wiremock` /
  `httpmock` (fake HTTP services), `rstest` (fixtures/parametrisation),
  `proptest` (properties), `cargo-llvm-cov` (coverage), `cargo-nextest` (runner).
- **Dashboard → Vitest.** JS/React only; pointless unless the UI grows real logic
  (validation, state) beyond the current fetch-and-render. Add a `test` script and
  `vitest` dep together with the first actual test.

Cheap, high-value starting points (pure functions, no DB/network needed):

- ⬜ `common::media::provider_of` (URL → provider).
- ⬜ Size formatting (`human_size` in `worker/src/notify.rs`, `fmt_size` in
  `telegram`).
- ⬜ `/download` option parsing and `list_music_files` filtering.
- ⬜ CI: run `cargo test` (and `vitest run` once it exists) in
  `.github/workflows/build.yml` — currently there's no test step.

## How to use this doc

Pick a track, build the smallest useful slice, and keep the shared core
(`common`, migrations, the worker) the place where capability lives. A feature
that only makes sense in one face is usually a bug in the design — it should be a
core action with more than one way to trigger it.
