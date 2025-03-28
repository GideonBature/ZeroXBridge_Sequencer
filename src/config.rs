use sqlx::postgres::PgPoolOptions;
use std::env;

pub async fn get_db_pool() -> sqlx::Result<sqlx::PgPool> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
}