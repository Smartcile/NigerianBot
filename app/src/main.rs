//! All-in-one NigerianBot: runs every surface — the Discord bot, the API +
//! dashboard, the download worker, and the Telegram bot — as concurrent tasks in
//! a single process/container.
//!
//! Each surface reads its own config from the environment; `TELEGRAM_BOT_TOKEN`
//! being unset simply disables the Telegram surface.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("nigerianbot");

    tracing::info!("starting all-in-one NigerianBot");

    tokio::try_join!(bot::run(), api::run(), worker::run(), telegram::run(),)?;

    Ok(())
}
