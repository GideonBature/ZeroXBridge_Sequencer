use std::sync::Arc;
use axum::{
    Router,
    routing::{post, get},
    Extension,
};
use sqlx::PgPool;

use crate::api::handlers::{handle_withdrawal_post, handle_get_pending_withdrawals};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn configure_routes() -> Router {
    Router::new()
        .route("/new", post(new_withdrawal))
        .route("/list", get(get_withdrawals))
}

pub fn create_router(pool: Arc<PgPool>) -> Router {
    Router::new()
        .route(
            "/withdraw",
            post(handle_withdrawal_post)
                .get(handle_get_pending_withdrawals),
        )
        .layer(Extension(pool))
}

#[cfg(test)]
pub async fn create_test_app() -> Router {
    use sqlx::postgres::PgPoolOptions;
    use std::env;

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let state = Arc::new(AppState { db: pool.clone() });

    Router::new()
        .route("/withdraw", post(handle_withdrawal_post).get(handle_get_pending_withdrawals))
        .with_state(state)
}
