-- Telegram identity directory. Mirrors the bot's `users` model so the Telegram
-- surface can resolve roles the same way. Unifying this with `users` into one
-- cross-surface identity model is Track 4 in docs/ROADMAP.md.
CREATE TABLE IF NOT EXISTS telegram_users (
    telegram_id BIGINT PRIMARY KEY,
    username    TEXT,
    role        TEXT NOT NULL DEFAULT 'user',  -- 'admin' | 'user' | 'viewer'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
