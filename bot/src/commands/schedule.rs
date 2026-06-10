//! `/schedule` — create recurring messages, one-off reminders, and recurring
//! media digests. Stored in `schedules`; the bot's background scheduler fires them.

use anyhow::Result;
use serenity::all::{
    ChannelType, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};
use sqlx::PgPool;

pub fn definition() -> CreateCommand {
    let text_channel = || {
        CreateCommandOption::new(CommandOptionType::Channel, "channel", "Text channel")
            .required(true)
            .channel_types(vec![ChannelType::Text])
    };
    let minutes = |desc: &str| {
        CreateCommandOption::new(CommandOptionType::Integer, "minutes", desc)
            .required(true)
            .min_int_value(1)
    };

    CreateCommand::new("schedule")
        .description("Schedule messages, reminders, and media digests")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "reminder",
                "One-off reminder posted in this channel",
            )
            .add_sub_option(minutes("Minutes from now"))
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "text", "Reminder text")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "recurring",
                "Recurring message to a channel",
            )
            .add_sub_option(text_channel())
            .add_sub_option(minutes("Repeat every N minutes"))
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "text", "Message text")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "digest",
                "Recurring Sonarr/Radarr download digest",
            )
            .add_sub_option(text_channel())
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "service", "Which service")
                    .required(true)
                    .add_string_choice("Sonarr (TV)", "sonarr")
                    .add_string_choice("Radarr (movies)", "radarr"),
            )
            .add_sub_option(minutes("Repeat every N minutes")),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List this server's schedules",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Delete a schedule")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Integer, "id", "Schedule id")
                        .required(true),
                ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let Some(db) = &state.db else {
        return super::respond_ephemeral(ctx, command, "Database not available.").await;
    };

    match super::subcommand_name(command) {
        "reminder" => reminder(ctx, command, db, guild_id.get() as i64).await,
        "recurring" => recurring(ctx, command, db, guild_id.get() as i64).await,
        "digest" => digest(ctx, command, db, guild_id.get() as i64).await,
        "list" => list(ctx, command, db, guild_id.get() as i64).await,
        "delete" => delete(ctx, command, db, guild_id.get() as i64).await,
        other => super::respond_ephemeral(ctx, command, format!("Unknown action: `{other}`")).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn insert(
    db: &PgPool,
    guild_id: i64,
    channel_id: i64,
    kind: &str,
    message: Option<&str>,
    interval_seconds: Option<i64>,
    minutes: i64,
    created_by: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO schedules (guild_id, channel_id, kind, message, interval_seconds, next_run_at, created_by) \
         VALUES ($1, $2, $3, $4, $5, now() + ($6 * interval '1 minute'), $7)",
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(kind)
    .bind(message)
    .bind(interval_seconds)
    .bind(minutes)
    .bind(created_by)
    .execute(db)
    .await?;
    Ok(())
}

async fn reminder(
    ctx: &Context,
    command: &CommandInteraction,
    db: &PgPool,
    guild: i64,
) -> anyhow::Result<()> {
    let minutes = super::sub_option_i64(command, "minutes")
        .unwrap_or(1)
        .max(1);
    let text = super::sub_option_str(command, "text").unwrap_or("");
    if text.is_empty() {
        return super::respond_ephemeral(ctx, command, "Give me some reminder text.").await;
    }
    let channel = command.channel_id.get() as i64;
    insert(
        db,
        guild,
        channel,
        "message",
        Some(text),
        None,
        minutes,
        command.user.id.get() as i64,
    )
    .await?;
    super::respond(
        ctx,
        command,
        format!("⏰ Reminder set — I'll post that here in **{minutes} min**."),
    )
    .await
}

async fn recurring(
    ctx: &Context,
    command: &CommandInteraction,
    db: &PgPool,
    guild: i64,
) -> anyhow::Result<()> {
    let Some(channel) = super::sub_option_channel(command, "channel") else {
        return super::respond_ephemeral(ctx, command, "Pick a channel.").await;
    };
    let minutes = super::sub_option_i64(command, "minutes")
        .unwrap_or(60)
        .max(1);
    let text = super::sub_option_str(command, "text").unwrap_or("");
    if text.is_empty() {
        return super::respond_ephemeral(ctx, command, "Give me some message text.").await;
    }
    insert(
        db,
        guild,
        channel.get() as i64,
        "message",
        Some(text),
        Some(minutes * 60),
        minutes,
        command.user.id.get() as i64,
    )
    .await?;
    super::respond(
        ctx,
        command,
        format!(
            "🔁 I'll post that in <#{}> every **{minutes} min**.",
            channel.get()
        ),
    )
    .await
}

async fn digest(
    ctx: &Context,
    command: &CommandInteraction,
    db: &PgPool,
    guild: i64,
) -> anyhow::Result<()> {
    let Some(channel) = super::sub_option_channel(command, "channel") else {
        return super::respond_ephemeral(ctx, command, "Pick a channel.").await;
    };
    let service = super::sub_option_str(command, "service").unwrap_or("sonarr");
    let minutes = super::sub_option_i64(command, "minutes")
        .unwrap_or(720)
        .max(1);
    let kind = if service == "radarr" {
        "digest_radarr"
    } else {
        "digest_sonarr"
    };
    insert(
        db,
        guild,
        channel.get() as i64,
        kind,
        None,
        Some(minutes * 60),
        minutes,
        command.user.id.get() as i64,
    )
    .await?;
    super::respond(
        ctx,
        command,
        format!(
            "📡 I'll post the **{service}** digest in <#{}> every **{minutes} min**.",
            channel.get()
        ),
    )
    .await
}

#[derive(sqlx::FromRow)]
struct Row {
    id: i64,
    channel_id: i64,
    kind: String,
    interval_seconds: Option<i64>,
    next_unix: i64,
}

async fn list(
    ctx: &Context,
    command: &CommandInteraction,
    db: &PgPool,
    guild: i64,
) -> anyhow::Result<()> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, channel_id, kind, interval_seconds, \
         extract(epoch FROM next_run_at)::bigint AS next_unix \
         FROM schedules WHERE guild_id = $1 AND enabled ORDER BY next_run_at",
    )
    .bind(guild)
    .fetch_all(db)
    .await?;

    if rows.is_empty() {
        return super::respond(
            ctx,
            command,
            "No schedules set. Use `/schedule reminder`, `recurring`, or `digest`.",
        )
        .await;
    }
    let mut out = String::from("🗓️ **Schedules:**\n");
    for r in rows {
        let what = match r.kind.as_str() {
            "digest_sonarr" => "Sonarr digest".to_string(),
            "digest_radarr" => "Radarr digest".to_string(),
            _ => "message".to_string(),
        };
        let cadence = match r.interval_seconds {
            Some(iv) => format!("every {}m", iv / 60),
            None => "once".to_string(),
        };
        out.push_str(&format!(
            "**#{}** · <#{}> · {what} · {cadence} · next <t:{}:R>\n",
            r.id, r.channel_id, r.next_unix
        ));
    }
    super::respond(ctx, command, out).await
}

async fn delete(
    ctx: &Context,
    command: &CommandInteraction,
    db: &PgPool,
    guild: i64,
) -> anyhow::Result<()> {
    let id = super::sub_option_i64(command, "id").unwrap_or(0);
    let result = sqlx::query("DELETE FROM schedules WHERE id = $1 AND guild_id = $2")
        .bind(id)
        .bind(guild)
        .execute(db)
        .await?;
    let msg = if result.rows_affected() > 0 {
        format!("🗑️ Deleted schedule **#{id}**.")
    } else {
        format!("No schedule **#{id}** in this server.")
    };
    super::respond(ctx, command, msg).await
}
