//! Download queue processor.
//!
//! Claims one `queued` row at a time (safe against multiple workers via
//! `FOR UPDATE SKIP LOCKED`), runs `yt-dlp` to fetch the video into
//! `DOWNLOADS_PATH`, then records the outcome and fires a notification.

use std::process::Stdio;

use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn};

use crate::config::WorkerConfig;
use crate::notify;

/// Claim and process the next queued download. Returns `true` if a job was
/// handled, `false` when the queue is empty.
pub async fn process_next(pool: &PgPool, config: &WorkerConfig) -> Result<bool> {
    let job: Option<(i64, String, Option<String>, Option<String>)> = sqlx::query_as(
        "UPDATE downloads SET status = 'downloading', progress = 0, updated_at = now() \
         WHERE id = (SELECT id FROM downloads WHERE status = 'queued' \
                     ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED) \
         RETURNING id, url, requested_by, save_as",
    )
    .fetch_optional(pool)
    .await
    .context("claiming a download job")?;

    let Some((id, url, requested_by, save_as)) = job else {
        return Ok(false);
    };

    info!(id, %url, "downloading");

    match run_ytdlp(config, pool, id, &url, save_as.as_deref()).await {
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

/// Run yt-dlp for one URL, streaming progress into the row as it downloads.
/// Returns `(title, relative_file_name, size_bytes)`.
async fn run_ytdlp(
    config: &WorkerConfig,
    pool: &PgPool,
    id: i64,
    url: &str,
    save_as: Option<&str>,
) -> Result<(String, String, i64)> {
    let output_template = save_as
        .map(sanitize_output_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "%(id)s.%(ext)s".to_string());

    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.args([
        "--no-playlist",
        "--no-warnings",
        "--newline",
        "--progress",
        "-f",
        "bv*+ba/b",
        "--merge-output-format",
        "mp4",
        "-P",
        &config.downloads_path,
        "-o",
        &output_template,
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

    let mut child = cmd
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run yt-dlp (is it installed?)")?;

    let stdout = child.stdout.take().context("no yt-dlp stdout")?;
    let stderr = child.stderr.take().context("no yt-dlp stderr")?;

    // yt-dlp writes progress to stderr; parse it and update the row (throttled),
    // remembering the last ERROR line for the failure message.
    let progress_pool = pool.clone();
    let progress_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut last_saved = -5i32;
        let mut last_error = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.contains("ERROR:") {
                last_error = line.trim().to_string();
            }
            if let Some(pct) = parse_percent(&line) {
                if pct >= last_saved + 3 || pct == 100 {
                    last_saved = pct;
                    let _ = sqlx::query(
                        "UPDATE downloads SET progress = $2, updated_at = now() WHERE id = $1",
                    )
                    .bind(id)
                    .bind(pct)
                    .execute(&progress_pool)
                    .await;
                }
            }
        }
        last_error
    });

    // The `--print` values (title, final path) arrive on stdout.
    let mut out_lines: Vec<String> = Vec::new();
    let mut out = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = out.next_line().await {
        let line = line.trim();
        if !line.is_empty() {
            out_lines.push(line.to_string());
        }
    }

    let status = child.wait().await.context("yt-dlp did not finish")?;
    let last_error = progress_task.await.unwrap_or_default();

    if !status.success() {
        let msg = if last_error.is_empty() {
            "unknown yt-dlp error".to_string()
        } else {
            last_error
        };
        anyhow::bail!("yt-dlp failed: {msg}");
    }

    let lines: Vec<&str> = out_lines.iter().map(String::as_str).collect();
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

/// Make a user-supplied filename safe (no path separators / traversal) and give
/// it an extension template if none was provided.
fn sanitize_output_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || (c as u32) < 0x20 {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.replace("..", "_").trim().to_string();
    if cleaned.is_empty() {
        return cleaned;
    }
    if cleaned.contains('.') {
        cleaned
    } else {
        format!("{cleaned}.%(ext)s")
    }
}

/// Parse a yt-dlp progress line like `[download]  42.3% of 1.2GiB ...`.
fn parse_percent(line: &str) -> Option<i32> {
    if !line.contains("[download]") {
        return None;
    }
    let idx = line.find('%')?;
    let digits: String = line[..idx]
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let value: f64 = digits.chars().rev().collect::<String>().parse().ok()?;
    Some(value.clamp(0.0, 100.0) as i32)
}
