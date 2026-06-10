# Ideas & backlog

A running list of enhancements. Numbers are **stable references** so we can say
"build Music #1" without ambiguity. See [`VISION.md`](VISION.md) for the bigger
platform direction these should build toward.

## 🎵 Music

1. **`/music search` — local-first, then YouTube.** Search the local library
   (filename + metadata); show matches with **➕ Add** buttons. If nothing local,
   fall back to `yt-dlp "ytsearch5:<query>"` and show YouTube results, also with
   Add buttons. Reuses the existing component-interaction (button) system.
2. **Music metadata.** Read tags from local files (`lofty`: title, artist, album,
   length, cover art). Powers richer now-playing embeds and search-by-artist/album.
   Best indexed into a `tracks` table via a background library scan so search is
   instant and doesn't hit disk per keystroke.
3. **Queue management.** `/music skip`, `remove <n>`, `clear`, `shuffle`, `loop`,
   `nowplaying` (with progress); plus **persistent queue restore** on restart
   (the `music_queue` table already exists for it).
4. **Playlists / bulk add.** `/music album <name>` / `artist <name>` to queue a
   whole album/artist (falls out of metadata indexing); user-saved playlists.

## 🛠️ Admin & control

1. **Control dashboard.** Make the web UI *do* things, not just show stats:
   playback control, manage schedules / autoplay / joinsound, see & manage the
   voice pool, filterable log viewer.
2. **RBAC + identity.** Admin/viewer roles in the JWT (`role` claim already
   exists); a `users` model linking Discord accounts. Foundation for everything
   account-related (see VISION.md).
3. **Media-request approvals.** `/radarr add` / `/sonarr add` become *pending
   requests* an admin approves from the dashboard before downloading. Pairs with a
   **quality-profile / root-folder picker** (currently uses the first one).
4. **Per-guild settings.** Default volume, allowed channels, request limits —
   editable from the dashboard.
5. **Ops.** Prometheus `/metrics`, error alerts posted to an admin channel,
   configurable log retention.

## 🌐 Platform

These extend the project from "a bot" toward an all-in-one self-hosted control
platform — identity/account provisioning, Authentik SSO, Seer (Jellyseerr →
Overseerr), and managing service accounts with privileges. **See
[`VISION.md`](VISION.md).**
