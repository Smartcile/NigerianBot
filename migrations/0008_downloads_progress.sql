-- Live download progress (0-100), updated by the worker while yt-dlp runs.
ALTER TABLE downloads ADD COLUMN IF NOT EXISTS progress INTEGER NOT NULL DEFAULT 0;
