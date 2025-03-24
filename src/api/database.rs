use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use sqlx::PgPool;


#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Withdrawal {
    pub id: i32,
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub status: String, // "pending", "processed", etc.
    pub created_at: chrono::NaiveDateTime,
}

pub async fn insert_withdrawal(
    pool: &PgPool,
    stark_pub_key: &str,
    amount: i64,
    commitment_hash: &str,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query_scalar!(
        r#"
        INSERT INTO withdrawals (stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
        stark_pub_key,
        amount,
        commitment_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}

