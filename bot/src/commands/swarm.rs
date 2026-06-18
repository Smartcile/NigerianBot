//! `/swarm` — all free bots in the pool join the user's current (or specified)
//! voice channel and play a specified local sound, then leave.

use serenity::all::{
    ChannelId, ChannelType, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption,
    GuildId,
};
use tracing::warn;
use std::sync::Arc;
use songbird::Songbird;

pub fn definition() -> CreateCommand {
    CreateCommand::new("swarm")
        .description("Make all free bots join a channel and play a sound once")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "song", "File from your library")
                .required(true)
                .set_autocomplete(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Channel, "channel", "Voice channel (optional)")
                .channel_types(vec![ChannelType::Voice]),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    let Some(guild_id) = command.guild_id else {
        return super::respond_ephemeral(ctx, command, "Use this in a server.").await;
    };

    let state = super::state(ctx).await;
    let Some(song) = super::sub_option_str(command, "song") else {
        return super::respond_ephemeral(ctx, command, "Missing song.").await;
    };

    // Determine target channel
    let target_channel = super::sub_option_channel(command, "channel").or_else(|| {
        command.member.as_ref().and_then(|m| {
            ctx.cache.guild(guild_id).and_then(|g| {
                g.voice_states.get(&m.user.id).and_then(|vs| vs.channel_id)
            })
        })
    });

    let Some(channel_id) = target_channel else {
        return super::respond_ephemeral(ctx, command, "Please specify a channel or join one.").await;
    };

    let free_bots = state.pool.free_bots(guild_id).await;
    if free_bots.is_empty() {
        return super::respond_ephemeral(ctx, command, "All bots are busy!").await;
    }

    super::respond(ctx, command, format!("🚀 Swarming <#{}> with {} bots...", channel_id, free_bots.len())).await?;

    for (_index, songbird) in free_bots {
        if let Err(e) = play_once(ctx, guild_id, channel_id, songbird, song).await {
            warn!(error = %e, "swarm bot failed to play");
        }
    }
    
    Ok(())
}

async fn play_once(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    songbird: Arc<Songbird>,
    source: &str,
) -> anyhow::Result<()> {
    let state = super::state(ctx).await;
    let (input, _title) = super::music::build_source(&state, source).await?;

    songbird.join(guild_id, channel_id).await?;
    
    let call_lock = songbird
        .get(guild_id)
        .ok_or_else(|| anyhow::anyhow!("the player just left"))?;

    let handle = {
        let mut call = call_lock.lock().await;
        call.enqueue_input(input).await
    };
    
    let _ = handle.add_event(
        songbird::Event::Track(songbird::TrackEvent::End),
        super::music::LeaveOnEnd {
            manager: songbird.clone(),
            guild_id,
        },
    );
    
    Ok(())
}

pub async fn handle_autocomplete(
    ctx: &Context,
    command: &CommandInteraction,
) -> anyhow::Result<()> {
    super::joinsound::handle_autocomplete(ctx, command).await
}
