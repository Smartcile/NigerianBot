-- Auto-play triggers: when a user joins a configured voice channel, the bot
-- joins and plays the configured source (a local file path or a URL).
CREATE TABLE IF NOT EXISTS voice_triggers (
    guild_id   BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    source     TEXT NOT NULL,
    title      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, channel_id)
);
