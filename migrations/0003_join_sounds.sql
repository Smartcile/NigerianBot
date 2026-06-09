-- Join sounds: when a user joins a configured voice channel, the bot joins,
-- plays the configured source once, and then leaves.
CREATE TABLE IF NOT EXISTS join_sounds (
    guild_id   BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    source     TEXT NOT NULL,
    title      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, channel_id)
);
