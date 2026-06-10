//! `/radarr` — query and control Radarr (movies) over its v3 API.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

use crate::services::arr::Arr;

pub fn definition() -> CreateCommand {
    let query = || {
        CreateCommandOption::new(CommandOptionType::String, "query", "Movie name").required(true)
    };
    CreateCommand::new("radarr")
        .description("Radarr — movie management")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "status",
            "Radarr health and library size",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "queue",
            "What's currently downloading",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "upcoming",
            "Movies releasing in the next 30 days",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "search", "Look up a movie")
                .add_sub_option(query()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "add",
                "Add a movie and start downloading",
            )
            .add_sub_option(query()),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    command.defer(&ctx.http).await?;
    let state = super::state(ctx).await;
    let text = match &state.radarr {
        None => "🎬 Radarr isn't configured. Set `RADARR_URL` and `RADARR_API_KEY`.".to_string(),
        Some(arr) => match run(arr, command).await {
            Ok(t) => t,
            Err(e) => format!("⚠️ Radarr: {e}"),
        },
    };
    super::respond_edit(ctx, command, text).await
}

async fn run(arr: &Arr, command: &CommandInteraction) -> Result<String> {
    match super::subcommand_name(command) {
        "status" => status(arr).await,
        "queue" => queue(arr).await,
        "upcoming" => upcoming(arr).await,
        "search" => search(arr, super::sub_option_str(command, "query").unwrap_or("")).await,
        "add" => add(arr, super::sub_option_str(command, "query").unwrap_or("")).await,
        other => Ok(format!("Unknown action: `{other}`")),
    }
}

async fn status(arr: &Arr) -> Result<String> {
    let status: Value = arr.get("system/status").await?;
    let version = status["version"].as_str().unwrap_or("?");
    let movies: Value = arr.get("movie").await?;
    let count = movies.as_array().map(|a| a.len()).unwrap_or(0);
    let queue: Value = arr.get_q("queue", &[("pageSize", "1")]).await?;
    let downloading = queue["totalRecords"].as_u64().unwrap_or(0);
    Ok(format!(
        "🎬 **Radarr** v{version}\n• Movies: **{count}**\n• Downloading: **{downloading}**"
    ))
}

async fn queue(arr: &Arr) -> Result<String> {
    let data: Value = arr
        .get_q("queue", &[("pageSize", "20"), ("sortKey", "timeleft")])
        .await?;
    let records = data["records"].as_array().cloned().unwrap_or_default();
    if records.is_empty() {
        return Ok("🎬 The Radarr queue is empty.".to_string());
    }
    let mut out = String::from("🎬 **Radarr — downloading:**\n");
    for r in records.iter().take(15) {
        let title = r["title"].as_str().unwrap_or("?");
        let status = r["status"].as_str().unwrap_or("");
        let timeleft = r["timeleft"].as_str().unwrap_or("");
        out.push_str(&format!("• {title} — {status} {timeleft}\n"));
    }
    Ok(cap(out))
}

async fn upcoming(arr: &Arr) -> Result<String> {
    let now = chrono::Utc::now();
    let end = now + chrono::Duration::days(30);
    let start_s = now.format("%Y-%m-%d").to_string();
    let end_s = end.format("%Y-%m-%d").to_string();
    let movies: Value = arr
        .get_q("calendar", &[("start", &start_s), ("end", &end_s)])
        .await?;
    let list = movies.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Ok("🎬 No releases in the next 30 days.".to_string());
    }
    let mut out = String::from("🎬 **Releasing soon:**\n");
    for m in list.iter().take(15) {
        let title = m["title"].as_str().unwrap_or("?");
        let year = m["year"].as_u64().unwrap_or(0);
        let date = m["digitalRelease"]
            .as_str()
            .or_else(|| m["physicalRelease"].as_str())
            .or_else(|| m["inCinemas"].as_str())
            .and_then(|d| d.split('T').next())
            .unwrap_or("TBA");
        out.push_str(&format!("• **{title}** ({year}) — {date}\n"));
    }
    Ok(cap(out))
}

async fn search(arr: &Arr, term: &str) -> Result<String> {
    if term.is_empty() {
        return Err(anyhow!("give me a movie name to search for."));
    }
    let results: Value = arr.get_q("movie/lookup", &[("term", term)]).await?;
    let list = results.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Ok(format!("🎬 No movie results for **{term}**."));
    }
    let mut out = format!("🎬 **Results for \"{term}\":**\n");
    for m in list.iter().take(5) {
        let title = m["title"].as_str().unwrap_or("?");
        let year = m["year"].as_u64().unwrap_or(0);
        let snippet: String = m["overview"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(150)
            .collect();
        let in_lib = m["id"].as_i64().unwrap_or(0) > 0;
        let mark = if in_lib { " ✅ (in library)" } else { "" };
        out.push_str(&format!("**{title}** ({year}){mark}\n{snippet}…\n\n"));
    }
    out.push_str("_`/radarr add <name>` adds the top match._");
    Ok(cap(out))
}

async fn add(arr: &Arr, term: &str) -> Result<String> {
    if term.is_empty() {
        return Err(anyhow!("give me a movie name to add."));
    }
    let results: Value = arr.get_q("movie/lookup", &[("term", term)]).await?;
    let mut movie = results
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| anyhow!("no movie match for \"{term}\"."))?;

    let title = movie["title"].as_str().unwrap_or(term).to_string();
    if movie["id"].as_i64().unwrap_or(0) > 0 {
        return Ok(format!("🎬 **{title}** is already in Radarr."));
    }

    let (quality_profile_id, root_folder) = profile_and_root(arr).await?;
    movie["qualityProfileId"] = json!(quality_profile_id);
    movie["rootFolderPath"] = json!(root_folder);
    movie["monitored"] = json!(true);
    movie["minimumAvailability"] = json!("released");
    movie["addOptions"] = json!({ "searchForMovie": true });

    let _created: Value = arr.post("movie", &movie).await?;
    Ok(format!("🎬 Added **{title}** and started searching. 🍿"))
}

async fn profile_and_root(arr: &Arr) -> Result<(i64, String)> {
    let profiles: Value = arr.get("qualityprofile").await?;
    let quality = profiles
        .as_array()
        .and_then(|a| a.first())
        .and_then(|p| p["id"].as_i64())
        .ok_or_else(|| anyhow!("no quality profile configured in Radarr."))?;
    let folders: Value = arr.get("rootfolder").await?;
    let root = folders
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r["path"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no root folder configured in Radarr."))?;
    Ok((quality, root))
}

/// Keep replies under Discord's 2000-character limit.
fn cap(s: String) -> String {
    if s.chars().count() > 1900 {
        s.chars().take(1900).collect::<String>() + "…"
    } else {
        s
    }
}
