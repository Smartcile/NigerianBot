//! Telegram commands. A thin surface over the same core (Postgres + the worker's
//! download queue) that the Discord bot and web GUI use.

use std::sync::Arc;

use sqlx::PgPool;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::AppState;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "NigerianBot controls:")]
pub enum Command {
    #[command(description = "show this help")]
    Start,
    #[command(description = "show this help")]
    Help,
    #[command(description = "service and download status")]
    Status,
    #[command(description = "queue a video URL (Vimeo/YouTube) to download")]
    Download(String),
    #[command(description = "list recent downloads")]
    Downloads,
    #[command(description = "show your role")]
    Whoami,
}

pub fn help_text() -> String {
    format!(
        "🤖 NigerianBot (Telegram)\n\n{}\n\nDownloads are fetched by the worker; \
         finished files are linked from the web dashboard.",
        Command::descriptions()
    )
}

pub async fn handle(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    let chat = msg.chat.id;
    let user = msg.from.clone();
    let user_id = user.as_ref().map(|u| u.id.0 as i64);
    let username = user.as_ref().and_then(|u| u.username.clone());

    // Best-effort identity directory.
    if let (Some(db), Some(id)) = (&state.db, user_id) {
        touch(db, id, username.as_deref()).await;
    }

    match cmd {
        Command::Start | Command::Help => {
            bot.send_message(chat, help_text()).await?;
        }
        Command::Whoami => {
            let role = match (&state.db, user_id) {
                (Some(db), Some(id)) => role_of(db, id, &state.admin_ids).await,
                (_, Some(id)) if state.admin_ids.contains(&(id as u64)) => "admin".to_string(),
                _ => "user".to_string(),
            };
            bot.send_message(chat, format!("You are **{role}**."))
                .await?;
        }
        Command::Status => {
            let text = match &state.db {
                Some(db) => match counts(db).await {
                    Ok((queued, downloading, done, failed)) => format!(
                        "📊 Downloads — queued {queued} · downloading {downloading} · \
                         done {done} · failed {failed}"
                    ),
                    Err(e) => format!("⚠️ Could not read status: {e}"),
                },
                None => "⚠️ Database not configured.".to_string(),
            };
            bot.send_message(chat, text).await?;
        }
        Command::Download(url) => {
            let url = url.trim();
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                bot.send_message(chat, "Provide an http(s) URL.").await?;
                return Ok(());
            }
            let Some(db) = &state.db else {
                bot.send_message(chat, "Database not configured.").await?;
                return Ok(());
            };
            let provider = common::media::provider_of(url);
            let requested_by = user_id
                .map(|id| format!("telegram:{id}"))
                .unwrap_or_else(|| "telegram".to_string());
            let inserted: Result<i64, sqlx::Error> = sqlx::query_scalar(
                "INSERT INTO downloads (url, provider, requested_by) VALUES ($1, $2, $3) RETURNING id",
            )
            .bind(url)
            .bind(provider)
            .bind(&requested_by)
            .fetch_one(db)
            .await;
            let text = match inserted {
                Ok(id) => format!("📥 Queued download **#{id}** ({provider})."),
                Err(e) => format!("⚠️ Could not queue that: {e}"),
            };
            bot.send_message(chat, text).await?;
        }
        Command::Downloads => {
            let text = match &state.db {
                Some(db) => match recent(db).await {
                    Ok(rows) if rows.is_empty() => "No downloads yet.".to_string(),
                    Ok(rows) => {
                        let mut out = String::from("🗂 Recent downloads:\n");
                        for (id, title, status, file_path, size) in rows {
                            let name = title.unwrap_or_else(|| format!("#{id}"));
                            let sz = size.map(fmt_size).unwrap_or_else(|| "—".to_string());
                            let link = match (&state.public_base_url, &file_path) {
                                (Some(base), Some(file)) if status == "done" => {
                                    format!("\n  {}/media/{}", base.trim_end_matches('/'), file)
                                }
                                _ => String::new(),
                            };
                            out.push_str(&format!("• #{id} {name} — {status} ({sz}){link}\n"));
                        }
                        out
                    }
                    Err(e) => format!("⚠️ Could not list downloads: {e}"),
                },
                None => "⚠️ Database not configured.".to_string(),
            };
            bot.send_message(chat, text).await?;
        }
    }
    Ok(())
}

async fn touch(db: &PgPool, id: i64, username: Option<&str>) {
    let _ = sqlx::query(
        "INSERT INTO telegram_users (telegram_id, username) VALUES ($1, $2) \
         ON CONFLICT (telegram_id) DO UPDATE SET username = EXCLUDED.username, updated_at = now()",
    )
    .bind(id)
    .bind(username)
    .execute(db)
    .await;
}

async fn role_of(db: &PgPool, id: i64, admins: &[u64]) -> String {
    if admins.contains(&(id as u64)) {
        return "admin".to_string();
    }
    sqlx::query_as::<_, (String,)>("SELECT role FROM telegram_users WHERE telegram_id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|(role,)| role)
        .unwrap_or_else(|| "user".to_string())
}

async fn counts(db: &PgPool) -> Result<(i64, i64, i64, i64), sqlx::Error> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status, count(*)::bigint FROM downloads GROUP BY status")
            .fetch_all(db)
            .await?;
    let get = |s: &str| {
        rows.iter()
            .find(|(k, _)| k == s)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    };
    Ok((
        get("queued"),
        get("downloading"),
        get("done"),
        get("failed"),
    ))
}

async fn recent(
    db: &PgPool,
) -> Result<Vec<(i64, Option<String>, String, Option<String>, Option<i64>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, status, file_path, size_bytes FROM downloads \
         ORDER BY created_at DESC LIMIT 10",
    )
    .fetch_all(db)
    .await
}

fn fmt_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
