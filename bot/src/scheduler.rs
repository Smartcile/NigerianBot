//! Background scheduler — runs inside the bot process (it already has the
//! Discord connection, database, and Sonarr/Radarr clients). Every minute it
//! fires any due `schedules` rows (recurring messages, reminders, media
//! digests) and prunes old log rows.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use serenity::all::{ChannelId, Http};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::services::arr::Arr;
use crate::state::BotState;

#[derive(sqlx::FromRow)]
struct Schedule {
    id: i64,
    channel_id: i64,
    kind: String,
    message: Option<String>,
    interval_seconds: Option<i64>,
}

/// Spawn the scheduler loop. No-op effect until a database is configured.
pub fn spawn(http: Arc<Http>, state: Arc<BotState>) {
    tokio::spawn(async move {
        info!("scheduler started");
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let Some(db) = state.db.clone() else {
                continue;
            };
            if let Err(e) = tick(&http, &state, &db).await {
                warn!(?e, "scheduler tick failed");
            }
        }
    });
}

async fn tick(http: &Http, state: &BotState, db: &PgPool) -> anyhow::Result<()> {
    let due: Vec<Schedule> = sqlx::query_as(
        "SELECT id, channel_id, kind, message, interval_seconds FROM schedules \
         WHERE enabled AND next_run_at <= now() ORDER BY next_run_at LIMIT 25",
    )
    .fetch_all(db)
    .await?;

    for s in &due {
        if let Err(e) = fire(http, state, s).await {
            warn!(schedule = s.id, ?e, "schedule failed to fire");
        }
        // Advance recurring schedules; disable one-offs.
        match s.interval_seconds {
            Some(iv) if iv > 0 => {
                let _ = sqlx::query(
                    "UPDATE schedules SET next_run_at = now() + ($1 * interval '1 second') WHERE id = $2",
                )
                .bind(iv)
                .bind(s.id)
                .execute(db)
                .await;
            }
            _ => {
                let _ = sqlx::query("UPDATE schedules SET enabled = false WHERE id = $1")
                    .bind(s.id)
                    .execute(db)
                    .await;
            }
        }
    }

    // Housekeeping: prune audit/log rows older than 30 days (cheap no-op when clean).
    let _ = sqlx::query("DELETE FROM audit_log WHERE created_at < now() - interval '30 days'")
        .execute(db)
        .await;
    let _ = sqlx::query("DELETE FROM logs WHERE created_at < now() - interval '30 days'")
        .execute(db)
        .await;

    Ok(())
}

async fn fire(http: &Http, state: &BotState, s: &Schedule) -> anyhow::Result<()> {
    let channel = ChannelId::new(s.channel_id as u64);
    let content = match s.kind.as_str() {
        "message" => s.message.clone().unwrap_or_default(),
        "digest_sonarr" => digest(&state.sonarr, "📺 **Sonarr**").await,
        "digest_radarr" => digest(&state.radarr, "🎬 **Radarr**").await,
        other => format!("(unknown schedule kind: {other})"),
    };
    if content.trim().is_empty() {
        return Ok(());
    }
    channel.say(http, content).await?;
    Ok(())
}

/// A short "currently downloading" digest from a Servarr queue.
async fn digest(arr: &Option<Arr>, label: &str) -> String {
    let Some(arr) = arr else {
        return format!("{label}: not configured.");
    };
    match arr
        .get_q::<Value>("queue", &[("pageSize", "15"), ("sortKey", "timeleft")])
        .await
    {
        Ok(data) => {
            let records = data["records"].as_array().cloned().unwrap_or_default();
            if records.is_empty() {
                return format!("{label} — nothing downloading right now. ✅");
            }
            let mut out = format!("{label} — downloading:\n");
            for r in records.iter().take(10) {
                let title = r["title"].as_str().unwrap_or("?");
                let timeleft = r["timeleft"].as_str().unwrap_or("");
                out.push_str(&format!("• {title} {timeleft}\n"));
            }
            out
        }
        Err(e) => format!("{label}: couldn't reach the server ({e})"),
    }
}
