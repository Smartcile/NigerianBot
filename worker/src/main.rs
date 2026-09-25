//! Standalone worker binary (`nigerian-worker`). The logic lives in `worker`'s
//! library; the all-in-one `nigerianbot` binary calls `worker::run()` instead.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::config::load_dotenv();
    common::telemetry::init("worker");
    worker::run().await
}
