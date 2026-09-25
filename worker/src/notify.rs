//! Outbound notifications for finished downloads. Best-effort: a failure to
//! notify is logged, never fatal. Discord uses an incoming webhook; Telegram
//! uses the Bot API directly (no gateway / bot loop required).

use tracing::{info, warn};

use crate::config::WorkerConfig;

pub async fn download_done(
    config: &WorkerConfig,
    title: &str,
    file_path: &str,
    size: i64,
    url: &str,
) {
    let link = match &config.public_base_url {
        Some(base) => format!(" ({}/media/{})", base.trim_end_matches('/'), file_path),
        None => String::new(),
    };
    let text = format!(
        "✅ Download complete: {title}\n{url}\n{} · saved as {file_path}{link}",
        human_size(size)
    );
    send_all(config, &text).await;
}

pub async fn download_failed(
    config: &WorkerConfig,
    url: &str,
    error: &str,
    requested_by: Option<&str>,
) {
    let who = requested_by
        .map(|w| format!("\nrequested by {w}"))
        .unwrap_or_default();
    let text = format!("❌ Download failed: {url}\n{error}{who}");
    send_all(config, &text).await;
}

async fn send_all(config: &WorkerConfig, text: &str) {
    if let Some(webhook) = &config.discord_webhook {
        if let Err(e) = send_discord(webhook, text).await {
            warn!(error = %e, "failed to send Discord notification");
        }
    }
    if let (Some(token), Some(chat)) = (&config.telegram_bot_token, &config.telegram_chat_id) {
        if let Err(e) = send_telegram(token, chat, text).await {
            warn!(error = %e, "failed to send Telegram notification");
        }
    }
}

async fn send_discord(webhook: &str, text: &str) -> anyhow::Result<()> {
    reqwest::Client::new()
        .post(webhook)
        .json(&serde_json::json!({ "content": text }))
        .send()
        .await?
        .error_for_status()?;
    info!("sent Discord download notification");
    Ok(())
}

async fn send_telegram(token: &str, chat_id: &str, text: &str) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
        .send()
        .await?
        .error_for_status()?;
    info!("sent Telegram download notification");
    Ok(())
}

/// Render a byte count as `1.2 GB`, `34.5 MB`, etc.
fn human_size(bytes: i64) -> String {
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
