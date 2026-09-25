-- Download queue (Vimeo / YouTube / any yt-dlp-supported URL).
-- Any surface (Discord, Telegram, the web GUI) inserts a 'queued' row; the worker
-- claims it, downloads the file into DOWNLOADS_PATH, and records the outcome.
CREATE TABLE IF NOT EXISTS downloads (
    id           BIGSERIAL PRIMARY KEY,
    url          TEXT NOT NULL,
    provider     TEXT NOT NULL DEFAULT 'url',      -- 'vimeo' | 'youtube' | 'url'
    title        TEXT,
    status       TEXT NOT NULL DEFAULT 'queued',   -- queued|downloading|done|failed
    -- Path of the finished file RELATIVE to the downloads directory (e.g.
    -- "707452756.mp4"); the API serves that directory at /media.
    file_path    TEXT,
    size_bytes   BIGINT,
    error        TEXT,
    requested_by TEXT,                              -- 'discord:123' | 'telegram:456' | 'web'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads (status, id);
CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads (created_at DESC);
