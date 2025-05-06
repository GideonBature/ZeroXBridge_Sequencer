use axum::{routing::post, Extension, Router};
use sqlx::PgPool;
use std::sync::Arc;

use crate::api::handlers::{
    create_withdrawal, get_pending_withdrawals, handle_deposit_post, handle_get_pending_deposits,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn create_router(pool: Arc<PgPool>) -> Router {
    Router::new()
        .route(
            "/deposit",
            post(handle_deposit_post).get(handle_get_pending_deposits),
        )
        .route(
            "/withdrawals",
            post(create_withdrawal).get(get_pending_withdrawals),
        )
        .layer(Extension(pool))
}
