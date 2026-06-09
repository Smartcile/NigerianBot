-- NigerianBot initial schema (Phase 3).
-- Discord snowflake IDs are stored as BIGINT.

-- Workflow definitions (executed by the worker in Phase 8).
CREATE TABLE IF NOT EXISTS workflows (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    definition  JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Structured bot/service logs surfaced on the dashboard.
CREATE TABLE IF NOT EXISTS logs (
    id         BIGSERIAL PRIMARY KEY,
    service    TEXT NOT NULL,
    level      TEXT NOT NULL,
    message    TEXT NOT NULL,
    context    JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_logs_created_at ON logs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_logs_service ON logs (service);

-- Key/value bot configuration.
CREATE TABLE IF NOT EXISTS bot_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Cron-style scheduled tasks (driven by the scheduler in Phase 8).
CREATE TABLE IF NOT EXISTS scheduled_tasks (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    cron        TEXT NOT NULL,
    workflow_id BIGINT REFERENCES workflows (id) ON DELETE CASCADE,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- External service credentials (Sonar, Radar — Phase 5).
CREATE TABLE IF NOT EXISTS external_service_configs (
    id         BIGSERIAL PRIMARY KEY,
    service    TEXT NOT NULL UNIQUE,
    base_url   TEXT NOT NULL,
    token      TEXT,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Persistent per-guild music queue (Phase 6).
CREATE TABLE IF NOT EXISTS music_queue (
    id           BIGSERIAL PRIMARY KEY,
    guild_id     BIGINT NOT NULL,
    position     INTEGER NOT NULL,
    title        TEXT,
    source       TEXT NOT NULL,
    requested_by BIGINT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_music_queue_guild ON music_queue (guild_id, position);

-- Audit trail of user actions (security).
CREATE TABLE IF NOT EXISTS audit_log (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL,
    user_name  TEXT,
    guild_id   BIGINT,
    command    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_log (created_at DESC);
