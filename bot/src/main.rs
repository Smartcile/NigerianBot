//! Standalone Discord bot binary (`nigerian-bot`). The logic lives in `bot`'s
//! library; the all-in-one `nigerianbot` binary calls `bot::run()` instead.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("bot");
    bot::run().await
}
