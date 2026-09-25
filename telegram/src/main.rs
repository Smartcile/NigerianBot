//! Standalone Telegram binary (`nigerian-telegram`). The logic lives in
//! `telegram`'s library; the all-in-one `nigerianbot` binary calls
//! `telegram::run()` instead.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("telegram");
    telegram::run().await
}
