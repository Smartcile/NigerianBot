//! Identity & role-based access. The foundation for the wider platform vision
//! (see `docs/VISION.md`): a Discord-keyed `users` model with roles, which later
//! extends to account linking and provisioning across services.

use serenity::all::UserId;
use sqlx::PgPool;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Admin,
    User,
    Viewer,
}

impl Role {
    pub fn parse(s: &str) -> Role {
        match s {
            "admin" => Role::Admin,
            "viewer" => Role::Viewer,
            _ => Role::User,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
            Role::Viewer => "viewer",
        }
    }

    /// Higher rank = more privilege. Use for `role >= Role::User` style checks.
    /// Reserved for gating commands by role (see docs/IDEAS.md, docs/VISION.md).
    #[allow(dead_code)]
    pub fn rank(self) -> u8 {
        match self {
            Role::Viewer => 0,
            Role::User => 1,
            Role::Admin => 2,
        }
    }

    pub fn is_admin(self) -> bool {
        self == Role::Admin
    }
}

/// Ensure a user row exists and refresh their name. Best-effort; called when a
/// command runs so the directory stays current.
pub async fn touch(db: &PgPool, user_id: UserId, name: &str) {
    let _ = sqlx::query(
        "INSERT INTO users (discord_id, discord_name) VALUES ($1, $2) \
         ON CONFLICT (discord_id) DO UPDATE SET discord_name = EXCLUDED.discord_name, updated_at = now()",
    )
    .bind(user_id.get() as i64)
    .bind(name)
    .execute(db)
    .await;
}

/// Resolve a user's effective role. The guild owner and any `ADMIN_DISCORD_IDS`
/// are always Admin (bootstrap), so there's always a way in without pre-seeding.
pub async fn role_of(
    db: Option<&PgPool>,
    user_id: UserId,
    guild_owner: Option<UserId>,
    env_admins: &[u64],
) -> Role {
    if guild_owner == Some(user_id) || env_admins.contains(&user_id.get()) {
        return Role::Admin;
    }
    if let Some(db) = db {
        if let Ok(Some((role,))) =
            sqlx::query_as::<_, (String,)>("SELECT role FROM users WHERE discord_id = $1")
                .bind(user_id.get() as i64)
                .fetch_optional(db)
                .await
        {
            return Role::parse(&role);
        }
    }
    Role::User
}

/// Set (and upsert) a user's role.
pub async fn set_role(
    db: &PgPool,
    user_id: UserId,
    name: Option<&str>,
    role: Role,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO users (discord_id, discord_name, role) VALUES ($1, $2, $3) \
         ON CONFLICT (discord_id) DO UPDATE SET role = EXCLUDED.role, \
         discord_name = COALESCE(EXCLUDED.discord_name, users.discord_name), updated_at = now()",
    )
    .bind(user_id.get() as i64)
    .bind(name)
    .bind(role.as_str())
    .execute(db)
    .await?;
    Ok(())
}
