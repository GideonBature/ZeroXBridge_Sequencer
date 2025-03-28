use axum::routing::get;
use axum::{Router, extract::State};
use sqlx::PgPool;
use std::sync::Arc;

pub fn withdrawal_routes() -> Router {
    Router::new()
        .route("/withdrawals", get(get_pending_withdrawals))
        .route("/withdrawals/create", get(create_withdrawal))
}

async fn get_pending_withdrawals(
    State(pool): State<Arc<PgPool>>,
) -> Result<Json<Vec<Withdrawal>>, (axum::http::StatusCode, String)> {
    // Your handler logic here
}

async fn create_withdrawal(
    State(pool): State<Arc<PgPool>>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<Withdrawal>, (axum::http::StatusCode, String)> {
    // Your handler logic here
}