//! PostgreSQL pool and migration runner.
//!
//! Schema changes happen only through files in `migrations/`. Deployed migrations
//! are immutable: fix mistakes with a new migration.

use sqlx::{PgPool, migrate::MigrateError, postgres::PgPoolOptions};

use crate::config::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    pool_options(config).connect(config.url.expose()).await
}

/// Creates the pool without opening a connection. Useful for tests and tooling.
pub fn connect_lazy(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    pool_options(config).connect_lazy(config.url.expose())
}

fn pool_options(config: &DatabaseConfig) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), MigrateError> {
    tracing::info!("applying database migrations");
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}
