//! Shared building blocks for every NigerianBot service.
//!
//! Keeping config loading, telemetry, and the database pool here means the
//! `bot`, `api`, `scheduler`, and `worker` crates don't each reimplement them.

pub mod config;
pub mod db;
pub mod telemetry;
