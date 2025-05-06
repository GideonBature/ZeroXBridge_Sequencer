use axum::{http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::db::database::{
    fetch_pending_deposits, fetch_pending_withdrawals, insert_deposit, insert_withdrawal, Deposit,
    Withdrawal,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct DepositRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct DepositResponse {
    pub deposit_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct WithrawalResponse {
    pub withdrawal_id: i32,
}

pub async fn handle_deposit_post(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, (StatusCode, String)> {
    if payload.amount <= 0 || payload.stark_pub_key.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Invalid input".to_string()));
    }
    let deposit_id = insert_deposit(
        &pool,
        &payload.stark_pub_key,
        payload.amount,
        &payload.commitment_hash,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DepositResponse { deposit_id }))
}

pub async fn handle_get_pending_deposits(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Deposit>>, (StatusCode, String)> {
    let deposit = fetch_pending_deposits(&pool, 5)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(deposit))
}

pub async fn create_withdrawal(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<WithrawalResponse>, (StatusCode, String)> {
    let withdrawal_id = insert_withdrawal(
        &pool,
        &payload.stark_pub_key,
        payload.amount,
        &payload.commitment_hash,
    )
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(WithrawalResponse { withdrawal_id }))
}

pub async fn get_pending_withdrawals(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Withdrawal>>, (StatusCode, String)> {
    let withdrawals = fetch_pending_withdrawals(&pool, 5).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(withdrawals))
}
