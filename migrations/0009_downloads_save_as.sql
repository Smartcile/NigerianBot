-- Optional user-chosen filename for a download, so Sonarr/Radarr can parse it
-- (e.g. "Show - S01E05 - Title.mp4") before import.
ALTER TABLE downloads ADD COLUMN IF NOT EXISTS save_as TEXT;
