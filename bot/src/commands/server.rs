//! `/server` — guild management.
//!
//! `/server info` is live (read-only). The mutating actions (`members`, `roles`,
//! `channels`) are destructive, so they're gated until role-based access control
//! lands in Phase 4.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
};

fn action_option() -> CreateCommandOption {
    CreateCommandOption::new(CommandOptionType::String, "action", "Action to perform")
        .required(true)
}

pub fn definition() -> CreateCommand {
    CreateCommand::new("server")
        .description("Server management")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "members",
                "Add/remove members",
            )
            .add_sub_option(action_option()),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "roles", "Manage roles")
                .add_sub_option(action_option()),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "channels",
                "Create/delete channels",
            )
            .add_sub_option(action_option()),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "info",
            "Show server information",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    match super::subcommand_name(command) {
        "info" => info(ctx, command).await,
        other => {
            super::respond_ephemeral(
                ctx,
                command,
                format!(
                    "🛠️ `/server {other}` changes the server, so it's gated until \
                     role-based access control lands (Phase 4)."
                ),
            )
            .await
        }
    }
}

/// `/server info` — read-only snapshot of the current guild.
async fn info(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "This command only works inside a server.")
            .await;
    };

    // Fetch fresh from the API (the cache feature is off): counts + channels.
    let guild = guild_id.to_partial_guild_with_counts(&ctx.http).await?;
    let channels = guild_id.channels(&ctx.http).await?;

    let members = guild
        .approximate_member_count
        .map(|c| c.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let created = guild_id.created_at().unix_timestamp();

    let mut embed = CreateEmbed::new()
        .title(format!("ℹ️ {}", guild.name))
        .colour(0x5865F2_u32)
        .field("Server ID", guild_id.to_string(), true)
        .field("Owner", format!("<@{}>", guild.owner_id), true)
        .field("Members", members, true)
        .field("Roles", guild.roles.len().to_string(), true)
        .field("Channels", channels.len().to_string(), true)
        // Discord renders <t:UNIX:D> as a localized date for each viewer.
        .field("Created", format!("<t:{created}:D>"), true);

    if let Some(icon) = guild.icon_url() {
        embed = embed.thumbnail(icon);
    }

    super::respond_embed(ctx, command, embed).await
}
