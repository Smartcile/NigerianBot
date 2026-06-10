-- Scheduled jobs: recurring or one-off posts to a channel, and media digests.
-- The bot's background scheduler fires anything due, then advances or disables it.
CREATE TABLE IF NOT EXISTS schedules (
    id               BIGSERIAL PRIMARY KEY,
    guild_id         BIGINT NOT NULL,
    channel_id       BIGINT NOT NULL,
    kind             TEXT NOT NULL,             -- 'message' | 'digest_sonarr' | 'digest_radarr'
    message          TEXT,                      -- used by the 'message' kind
    interval_seconds BIGINT,                    -- recurring interval; NULL = one-off
    next_run_at      TIMESTAMPTZ NOT NULL,
    enabled          BOOLEAN NOT NULL DEFAULT TRUE,
    created_by       BIGINT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_schedules_due ON schedules (enabled, next_run_at);
