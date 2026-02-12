use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::errors::InfraError;

pub type DbPool = PgPool;

pub async fn create_pool(db_url: &str) -> Result<DbPool, InfraError> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(db_url)
        .await
        .map_err(|e| InfraError::Pool(e.to_string()))?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), InfraError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| InfraError::Migration(e.to_string()))?;

    Ok(())
}
