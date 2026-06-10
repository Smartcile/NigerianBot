//! `/sonarr` — query and control Sonarr (TV) over its v3 API.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

use crate::services::arr::Arr;

pub fn definition() -> CreateCommand {
    let query =
        || CreateCommandOption::new(CommandOptionType::String, "query", "Show name").required(true);
    CreateCommand::new("sonarr")
        .description("Sonarr — TV management")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "status",
            "Sonarr health and library size",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "queue",
            "What's currently downloading",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "upcoming",
            "Episodes airing in the next 7 days",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "search", "Look up a show")
                .add_sub_option(query()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "add",
                "Add a show and start downloading",
            )
            .add_sub_option(query()),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    command.defer(&ctx.http).await?;
    let state = super::state(ctx).await;
    let text = match &state.sonarr {
        None => "📺 Sonarr isn't configured. Set `SONARR_URL` and `SONARR_API_KEY`.".to_string(),
        Some(arr) => match run(arr, command).await {
            Ok(t) => t,
            Err(e) => format!("⚠️ Sonarr: {e}"),
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
    let series: Value = arr.get("series").await?;
    let count = series.as_array().map(|a| a.len()).unwrap_or(0);
    let queue: Value = arr.get_q("queue", &[("pageSize", "1")]).await?;
    let downloading = queue["totalRecords"].as_u64().unwrap_or(0);
    Ok(format!(
        "📺 **Sonarr** v{version}\n• Series: **{count}**\n• Downloading: **{downloading}**"
    ))
}

async fn queue(arr: &Arr) -> Result<String> {
    let data: Value = arr
        .get_q("queue", &[("pageSize", "20"), ("sortKey", "timeleft")])
        .await?;
    let records = data["records"].as_array().cloned().unwrap_or_default();
    if records.is_empty() {
        return Ok("📺 The Sonarr queue is empty.".to_string());
    }
    let mut out = String::from("📺 **Sonarr — downloading:**\n");
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
    let end = now + chrono::Duration::days(7);
    let start_s = now.format("%Y-%m-%d").to_string();
    let end_s = end.format("%Y-%m-%d").to_string();
    let eps: Value = arr
        .get_q(
            "calendar",
            &[
                ("start", &start_s),
                ("end", &end_s),
                ("includeSeries", "true"),
            ],
        )
        .await?;
    let list = eps.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Ok("📺 Nothing airing in the next 7 days.".to_string());
    }
    let mut out = String::from("📺 **Airing soon:**\n");
    for e in list.iter().take(15) {
        let series = e["series"]["title"].as_str().unwrap_or("?");
        let s = e["seasonNumber"].as_u64().unwrap_or(0);
        let ep = e["episodeNumber"].as_u64().unwrap_or(0);
        let title = e["title"].as_str().unwrap_or("");
        let date = e["airDateUtc"]
            .as_str()
            .and_then(|d| d.split('T').next())
            .unwrap_or("");
        out.push_str(&format!(
            "• **{series}** S{s:02}E{ep:02} — {title} ({date})\n"
        ));
    }
    Ok(cap(out))
}

async fn search(arr: &Arr, term: &str) -> Result<String> {
    if term.is_empty() {
        return Err(anyhow!("give me a show name to search for."));
    }
    let results: Value = arr.get_q("series/lookup", &[("term", term)]).await?;
    let list = results.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Ok(format!("📺 No TV results for **{term}**."));
    }
    let mut out = format!("📺 **Results for \"{term}\":**\n");
    for s in list.iter().take(5) {
        let title = s["title"].as_str().unwrap_or("?");
        let year = s["year"].as_u64().unwrap_or(0);
        let snippet: String = s["overview"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(150)
            .collect();
        let in_lib = s["id"].as_i64().unwrap_or(0) > 0;
        let mark = if in_lib { " ✅ (in library)" } else { "" };
        out.push_str(&format!("**{title}** ({year}){mark}\n{snippet}…\n\n"));
    }
    out.push_str("_`/sonarr add <name>` adds the top match._");
    Ok(cap(out))
}

async fn add(arr: &Arr, term: &str) -> Result<String> {
    if term.is_empty() {
        return Err(anyhow!("give me a show name to add."));
    }
    let results: Value = arr.get_q("series/lookup", &[("term", term)]).await?;
    let mut series = results
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| anyhow!("no TV match for \"{term}\"."))?;

    let title = series["title"].as_str().unwrap_or(term).to_string();
    if series["id"].as_i64().unwrap_or(0) > 0 {
        return Ok(format!("📺 **{title}** is already in Sonarr."));
    }

    let (quality_profile_id, root_folder) = profile_and_root(arr).await?;
    series["qualityProfileId"] = json!(quality_profile_id);
    series["rootFolderPath"] = json!(root_folder);
    series["monitored"] = json!(true);
    series["seasonFolder"] = json!(true);
    series["addOptions"] = json!({ "searchForMissingEpisodes": true, "monitor": "all" });
    // Sonarr v3 requires a language profile; v4 dropped them.
    if let Some(lp) = first_language_profile(arr).await {
        series["languageProfileId"] = json!(lp);
    }

    let _created: Value = arr.post("series", &series).await?;
    Ok(format!("📺 Added **{title}** and started searching. 🎬"))
}

async fn profile_and_root(arr: &Arr) -> Result<(i64, String)> {
    let profiles: Value = arr.get("qualityprofile").await?;
    let quality = profiles
        .as_array()
        .and_then(|a| a.first())
        .and_then(|p| p["id"].as_i64())
        .ok_or_else(|| anyhow!("no quality profile configured in Sonarr."))?;
    let folders: Value = arr.get("rootfolder").await?;
    let root = folders
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r["path"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no root folder configured in Sonarr."))?;
    Ok((quality, root))
}

async fn first_language_profile(arr: &Arr) -> Option<i64> {
    let profiles: Value = arr.get("languageprofile").await.ok()?;
    profiles.as_array()?.first()?["id"].as_i64()
}

/// Keep replies under Discord's 2000-character limit.
fn cap(s: String) -> String {
    if s.chars().count() > 1900 {
        s.chars().take(1900).collect::<String>() + "…"
    } else {
        s
    }
}
