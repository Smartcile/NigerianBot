//! Download queue processor.
//!
//! Claims one `queued` row at a time (safe against multiple workers via
//! `FOR UPDATE SKIP LOCKED`), runs `yt-dlp` to fetch the video into
//! `DOWNLOADS_PATH`, then records the outcome and fires a notification.

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::config::WorkerConfig;
use crate::notify;

/// Claim and process the next queued download. Returns `true` if a job was
/// handled, `false` when the queue is empty.
pub async fn process_next(pool: &PgPool, config: &WorkerConfig) -> Result<bool> {
    let job: Option<(i64, String, Option<String>)> = sqlx::query_as(
        "UPDATE downloads SET status = 'downloading', updated_at = now() \
         WHERE id = (SELECT id FROM downloads WHERE status = 'queued' \
                     ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED) \
         RETURNING id, url, requested_by",
    )
    .fetch_optional(pool)
    .await
    .context("claiming a download job")?;

    let Some((id, url, requested_by)) = job else {
        return Ok(false);
    };

    info!(id, %url, "downloading");

    match run_ytdlp(config, &url).await {
        Ok((title, file_path, size)) => {
            sqlx::query(
                "UPDATE downloads SET status = 'done', title = $2, file_path = $3, \
                 size_bytes = $4, error = NULL, updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .bind(&title)
            .bind(&file_path)
            .bind(size)
            .execute(pool)
            .await
            .context("recording a completed download")?;
            info!(id, %title, size, "download complete");
            notify::download_done(config, &title, &file_path, size, &url).await;
        }
        Err(e) => {
            let msg = format!("{e:#}");
            sqlx::query("UPDATE downloads SET status = 'failed', error = $2, updated_at = now() WHERE id = $1")
                .bind(id)
                .bind(&msg)
                .execute(pool)
                .await
                .context("recording a failed download")?;
            warn!(id, error = %msg, "download failed");
            notify::download_failed(config, &url, &msg, requested_by.as_deref()).await;
        }
    }

    Ok(true)
}

/// Run yt-dlp for one URL. Returns `(title, relative_file_name, size_bytes)`.
async fn run_ytdlp(config: &WorkerConfig, url: &str) -> Result<(String, String, i64)> {
    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.args([
        "--no-playlist",
        "--no-warnings",
        "--newline",
        "-f",
        "bv*+ba/b",
        "--merge-output-format",
        "mp4",
        "-P",
        &config.downloads_path,
        "-o",
        "%(id)s.%(ext)s",
        "--print",
        "%(title)s",
        "--print",
        "after_move:filepath",
    ]);

    // Some sites (e.g. Vimeo) only serve the web client to a logged-in session;
    // pass the stored cookies file when it exists (uploaded via the web GUI).
    if std::path::Path::new(&config.cookies_file).is_file() {
        cmd.arg("--cookies").arg(&config.cookies_file);
    }

    let output = cmd
        .arg(url)
        .output()
        .await
        .context("failed to run yt-dlp (is it installed?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr
            .trim()
            .lines()
            .last()
            .unwrap_or("unknown yt-dlp error");
        anyhow::bail!("yt-dlp failed: {last}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let (title, abs) = match lines.as_slice() {
        [path] => ("Untitled".to_string(), (*path).to_string()),
        [title, .., path] => ((*title).to_string(), (*path).to_string()),
        [] => anyhow::bail!("yt-dlp produced no output"),
    };

    let path = std::path::Path::new(&abs);
    let size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
    let file_path = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&abs)
        .to_string();

    Ok((title, file_path, size))
}
