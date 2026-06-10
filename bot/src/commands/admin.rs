//! `/admin` — role management. The server owner and `ADMIN_DISCORD_IDS` are
//! always Admin; admins can promote/demote others. Everyone defaults to `user`.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
};

use crate::identity::{self, Role};

pub fn definition() -> CreateCommand {
    CreateCommand::new("admin")
        .description("Manage user roles")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "whoami",
            "Show your role",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "role",
                "Set a user's role (admin only)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "The user")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "level", "Role to assign")
                    .required(true)
                    .add_string_choice("Admin", "admin")
                    .add_string_choice("User", "user")
                    .add_string_choice("Viewer", "viewer"),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List users with elevated roles (admin only)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };
    let state = super::state(ctx).await;
    let owner = ctx.cache.guild(guild_id).map(|g| g.owner_id);
    let my_role =
        identity::role_of(state.db.as_ref(), command.user.id, owner, &state.admin_ids).await;

    match super::subcommand_name(command) {
        "whoami" => {
            super::respond_ephemeral(ctx, command, format!("You are **{}**.", my_role.as_str()))
                .await
        }
        "role" => {
            if !my_role.is_admin() {
                return super::respond_ephemeral(ctx, command, "🔒 Admins only.").await;
            }
            let Some(db) = &state.db else {
                return super::respond_ephemeral(ctx, command, "Database not available.").await;
            };
            let Some(target) = super::sub_option_user(command, "user") else {
                return super::respond_ephemeral(ctx, command, "Pick a user.").await;
            };
            let role = Role::parse(super::sub_option_str(command, "level").unwrap_or("user"));
            let name = command
                .data
                .resolved
                .users
                .get(&target)
                .map(|u| u.name.as_str());
            identity::set_role(db, target, name, role).await?;
            super::respond(
                ctx,
                command,
                format!("✅ <@{}> is now **{}**.", target.get(), role.as_str()),
            )
            .await
        }
        "list" => {
            if !my_role.is_admin() {
                return super::respond_ephemeral(ctx, command, "🔒 Admins only.").await;
            }
            let Some(db) = &state.db else {
                return super::respond_ephemeral(ctx, command, "Database not available.").await;
            };
            let rows: Vec<(i64, Option<String>, String)> = sqlx::query_as(
                "SELECT discord_id, discord_name, role FROM users WHERE role <> 'user' \
                 ORDER BY role DESC, discord_name",
            )
            .fetch_all(db)
            .await?;
            if rows.is_empty() {
                return super::respond(
                    ctx,
                    command,
                    "No users with elevated/restricted roles. Everyone defaults to **user**; the server owner is always **admin**.",
                )
                .await;
            }
            let mut out = String::from("**Roles:**\n");
            for (id, _name, role) in rows {
                out.push_str(&format!("• <@{id}> — **{role}**\n"));
            }
            super::respond(ctx, command, out).await
        }
        other => super::respond_ephemeral(ctx, command, format!("Unknown action: `{other}`")).await,
    }
}
