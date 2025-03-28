// use axum::{Json, Extension, http::StatusCode};
// use serde::{Deserialize, Serialize};
// use sqlx::PgPool;
// use sha2::{Sha256, Digest};
// use crate::api::database::{insert_withdrawal, Withdrawal};

// #[derive(Deserialize)]
// pub struct WithdrawalRequest {
//     pub stark_pub_key: String,
//     pub amount: i64,
// }

// #[derive(Serialize)]
// pub struct WithdrawalResponse {
//     pub commitment_hash: String,
// }

// pub async fn handle_withdrawal_post(
//     Extension(pool): Extension<PgPool>,
//     Json(payload): Json<WithdrawalRequest>,
// ) -> Result<Json<WithdrawalResponse>, (StatusCode, String)> {
//     if payload.amount <= 0 || payload.stark_pub_key.trim().is_empty() {
//         return Err((StatusCode::BAD_REQUEST, "Invalid input".to_string()));
//     }

//     // Generate commitment hash
//     let mut hasher = Sha256::new();
//     hasher.update(format!("{}{}", payload.stark_pub_key, payload.amount));
//     let commitment_hash = format!("{:x}", hasher.finalize());

//     insert_withdrawal(&pool, &payload.stark_pub_key, payload.amount, &commitment_hash)
//         .await
//         .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

//     Ok(Json(WithdrawalResponse { commitment_hash }))
// }

// pub async fn handle_get_pending_withdrawals(
//     Extension(pool): Extension<PgPool>,
// ) -> Result<Json<Vec<Withdrawal>>, (StatusCode, String)> {
//     let withdrawals = sqlx::query_as!(
//         Withdrawal,
//         r#"SELECT * FROM withdrawals WHERE status = 'pending' ORDER BY created_at DESC"#
//     )
//     .fetch_all(&pool)
//     .await
//     .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

//     Ok(Json(withdrawals))
// }
use axum::{extract::State, Json};
use sqlx::PgPool;
use crate::api::models::{Withdrawal, CreateWithdrawalRequest};
use uuid::Uuid;
use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: String,
    pub commitment_hash: String,
}

pub async fn create_withdrawal(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<Withdrawal>, (axum::http::StatusCode, String)> {
    let withdrawal = sqlx::query_as!(
        Withdrawal,
        r#"
        INSERT INTO withdrawals (id, stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
        Uuid::new_v4(), 
        payload.stark_pub_key, 
        payload.amount, 
        payload.commitment_hash
    )
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", err))
    })?;

    Ok(Json(withdrawal))
}

pub async fn get_pending_withdrawals(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Withdrawal>>, (axum::http::StatusCode, String)> {
    let withdrawals = sqlx::query_as!(
        Withdrawal,
        r#"SELECT * FROM withdrawals WHERE status = 'pending' ORDER BY created_at DESC"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|err| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", err))
    })?;

    Ok(Json(withdrawals))
}
