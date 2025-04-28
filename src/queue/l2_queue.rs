use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use crate::config::QueueConfig;
use crate::db::database::{
    fetch_pending_withdrawals, process_withdrawal_retry, update_withdrawal_status, Withdrawal,
};

#[derive(Debug, thiserror::Error)]
enum ValidationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Invalid commitment format")]
    InvalidCommitment,

    #[error("Commitment not yet found on L2")]
    CommitmentPending,

    #[error("Commitment not found after max retries")]
    MaxRetriesExceeded,
}

pub struct L2Queue {
    db_pool: PgPool,
    config: QueueConfig,
}

impl L2Queue {
    pub fn new(db_pool: PgPool, config: QueueConfig) -> Self {
        Self { db_pool, config }
    }

    pub async fn run(&self) {
        loop {
            match self.process_withdrawals().await {
                Ok(_) => info!("Completed processing cycle"),
                Err(e) => error!("Processing cycle failed: {:?}", e),
            }
            sleep(Duration::from_secs(self.config.process_interval_sec)).await;
        }
    }

    async fn process_withdrawals(&self) -> Result<(), sqlx::Error> {
        let withdrawals = fetch_pending_withdrawals(&self.db_pool, self.config.max_retries).await?;

        for withdrawal in withdrawals {
            let mut tx = self.db_pool.begin().await?;

            // Apply processing delay
            sleep(Duration::from_secs(self.config.initial_retry_delay_sec)).await;

            match self.validate_withdrawal(&withdrawal).await {
                Ok(()) => {
                    info!("Withdrawal {} validated successfully", withdrawal.id);
                    update_withdrawal_status(&mut tx, withdrawal.id, "processing").await?;
                    tx.commit().await?;
                }
                Err(ValidationError::CommitmentPending) => {
                    warn!(
                        "Withdrawal {} not yet found on L1. Will retry.",
                        withdrawal.id
                    );
                    process_withdrawal_retry(&mut tx, withdrawal.id).await?;
                    tx.commit().await?;
                }
                Err(ValidationError::MaxRetriesExceeded) => {
                    error!(
                        "Withdrawal {} failed after max retries. Marking as failed.",
                        withdrawal.id
                    );
                    update_withdrawal_status(&mut tx, withdrawal.id, "failed").await?;
                }
                Err(e) => {
                    warn!(
                        "Deposit {} hit an error: {:?}. Will retry.",
                        withdrawal.id, e
                    );
                    update_withdrawal_status(&mut tx, withdrawal.id, "failed").await?;
                    tx.commit().await?;
                }
            }
        }

        Ok(())
    }

    async fn validate_withdrawal(&self, withdrawal: &Withdrawal) -> Result<(), ValidationError> {
        // Check commitment exists on L2
        let commitment_exists = self
            .check_l2_commitment(withdrawal.commitment_hash.clone())
            .await?;

        if !commitment_exists {
            let max_retries_i32 = self.config.max_retries as i32;

            if withdrawal.retry_count + 1 >= max_retries_i32 {
                return Err(ValidationError::MaxRetriesExceeded);
            } else {
                return Err(ValidationError::CommitmentPending);
            }
        }

        Ok(())
    }

    async fn check_l2_commitment(&self, commitment_hash: String) -> Result<bool, ValidationError> {
        // Check l2 event logs for commitment hash
        // Assuming we have a function `get_event_logs` that fetches event logs from L2
        // we store last index so we don't have to fetch all logs every time

        trace!("Checking L2 commitment for hash: {}", commitment_hash);
        Ok(true)
    }
}
