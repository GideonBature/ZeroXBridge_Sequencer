use axum::{routing::post, Extension, Router};
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use zeroxbridge_sequencer::api::{
    handlers::{handle_deposit_post, handle_get_pending_deposits},
    routes::AppState,
};

pub async fn create_test_app() -> Router {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let state = Arc::new(AppState { db: pool.clone() });

    Router::new()
        .route(
            "/deposit",
            post(handle_deposit_post).get(handle_get_pending_deposits),
        )
        .layer(Extension(state))
}
