-- Compose builder: saved projects (canonical JSON model + rendered YAML) and
-- reusable palette defaults (bind mounts, networks, env-file key sets).
--
-- Secret env values are never stored; only key names + non-secret defaults live
-- in the model. The real values belong in the host-side <project>.env file.
CREATE TABLE IF NOT EXISTS compose_projects (
    id             BIGSERIAL PRIMARY KEY,
    name           TEXT NOT NULL UNIQUE,        -- slug, also the published filename
    description    TEXT,
    model          JSONB NOT NULL DEFAULT '{}'::jsonb,
    rendered_yaml  TEXT,
    published_path TEXT,
    published_at   TIMESTAMPTZ,
    created_by     TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS compose_defaults (
    id         BIGSERIAL PRIMARY KEY,
    kind       TEXT NOT NULL,                   -- 'bind_mount' | 'network' | 'env_file'
    name       TEXT NOT NULL,
    definition JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (kind, name)
);
