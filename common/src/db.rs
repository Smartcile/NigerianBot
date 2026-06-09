//! PostgreSQL connection pool shared by services that touch the database.

use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Create a connection pool against the given `DATABASE_URL`.
pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")
}
