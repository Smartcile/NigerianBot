//! Audit logging — records each slash-command invocation to the database.
//!
//! Uses runtime-checked `sqlx::query` (not the compile-time `query!` macro) so
//! the build doesn't require a live database — important for CI.

use serenity::all::CommandInteraction;
use sqlx::PgPool;

/// Record a command invocation. Best-effort: callers log and ignore errors so a
/// database hiccup never breaks the user's command.
pub async fn record(pool: &PgPool, command: &CommandInteraction) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (user_id, user_name, guild_id, command) VALUES ($1, $2, $3, $4)",
    )
    .bind(command.user.id.get() as i64)
    .bind(&command.user.name)
    .bind(command.guild_id.map(|g| g.get() as i64))
    .bind(&command.data.name)
    .execute(pool)
    .await?;
    Ok(())
}

/// Total number of recorded command invocations.
pub async fn count(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
        .fetch_one(pool)
        .await
}
