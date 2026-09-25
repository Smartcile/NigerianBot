//! Standalone API binary (`nigerian-api`). The logic lives in `api`'s library;
//! the all-in-one `nigerianbot` binary calls `api::run()` instead.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("api");
    api::run().await
}
