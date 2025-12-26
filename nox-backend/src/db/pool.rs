// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! Database Connection Pool

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

pub type DbPool = PgPool;
pub async fn init_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    info!("Connecting to PostgreSQL database...");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await?;

    info!("PostgreSQL connection pool established");
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    info!("Running database migrations...");
    let migration_sql = include_str!("../../migrations/001_initial.sql");
    sqlx::raw_sql(migration_sql).execute(pool).await?;
    info!("Database migrations completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pool_type() {
        // Type alias test
    }
}
