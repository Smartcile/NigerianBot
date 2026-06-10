-- Platform identity & roles. Keyed on Discord user id; the foundation for
-- account linking and privilege management across services (see docs/VISION.md).
CREATE TABLE IF NOT EXISTS users (
    discord_id   BIGINT PRIMARY KEY,
    discord_name TEXT,
    role         TEXT NOT NULL DEFAULT 'user',  -- 'admin' | 'user' | 'viewer'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
