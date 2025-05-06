use crate::queue::l2_queue::L2Transaction;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use starknet::core::chain_id::MAINNET; // or TESTNET
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::jsonrpc::JsonRpcClient;
use url::Url;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use starknet::accounts::{SingleOwnerAccount, LocalWallet};
use starknet::core::types::{ChainId, FieldElement};
use starknet::core::types::{TransactionReceipt, ExecutionStatus, MaybePendingTransactionReceipt};
use starknet::providers::ProviderError;
use starknet::core::types::ExecutionStatus::Succeeded;
use starknet::signers::SigningKey;
use tracing::{debug, error, info, warn};
use starknet::{
    core::{
        types::{BlockId, BlockTag, FieldElement, FunctionCall},
        utils::cairo_short_string_to_felt,
    },
    providers::{
        Provider, ProviderError, JsonRpcClient,
        MaybeUnknownErrorCode, StarknetErrorCode,
    },
    signers::{LocalWallet, SigningKey},
    accounts::{Account, Call, SingleOwnerAccount},
};

// Define custom error types for the Starknet Relayer
#[derive(Error, Debug)]
pub enum StarknetRelayerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Transaction not found")]
    TransactionNotFound,

    #[error("Proof data missing")]
    ProofDataMissing,

    #[error("Invalid contract address")]
    InvalidContractAddress,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Transaction timeout")]
    TransactionTimeout,

    // ✅ Add these if they're used
    #[error("Selector parse failed")]
    SelectorParseFailed,

    #[error("Request timed out")]
    Timeout,

    #[error("Timeout error: {0}")]
    TimeoutError(String),
}


// Configuration for the Starknet Relayer
#[derive(Debug, Clone)]
pub struct StarknetRelayerConfig {
    pub bridge_contract_address: String,
    pub rpc_url: String,
    pub private_key: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub transaction_timeout_ms: u64,
}

// The main Starknet Relayer struct
pub struct StarknetRelayer {
    db_pool: Pool<Postgres>,
    config: StarknetRelayerConfig,
    account: SingleOwnerAccount<JsonRpcClient, LocalWallet>,
}

pub async fn new(
    db_pool: Pool<Postgres>,
    config: StarknetRelayerConfig,
) -> Result<Self, StarknetRelayerError> {
    let provider = JsonRpcClient::new(HttpTransport::new(config.rpc_url.clone()));

    let private_key = FieldElement::from_hex_be(&config.private_key)
        .map_err(|_| StarknetRelayerError::TransactionFailed("Invalid private key".into()))?;
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

    let account_address = signer.address();
    let chain_id = MAINNET; // Set according to your environment

    let account = SingleOwnerAccount::new(
        provider,
        signer,
        account_address,
        chain_id,
    );

    Ok(Self {
        db_pool,
        config,
        account,
    })
}


    // Main function to start the relayer process
    pub async fn start(&self) -> Result<(), StarknetRelayerError> {
        info!("Starting Starknet Relayer service");
        
        loop {
            match self.process_pending_transactions().await {
                Ok(processed) => {
                    if processed > 0 {
                        info!("Successfully processed {} Starknet transactions", processed);
                    } else {
                        debug!("No pending Starknet transactions to process");
                    }
                }
                Err(e) => {
                    error!("Error processing Starknet transactions: {:?}", e);
                }
            }
            
            // Sleep before the next iteration
            sleep(Duration::from_secs(10)).await;
        }
    }

    // Process all pending transactions
    async fn process_pending_transactions(&self) -> Result<usize, StarknetRelayerError> {
        let mut processed_count = 0;
        
        // Fetch all transactions marked as "ready for relay"
        let transactions = self.fetch_ready_transactions().await?;
        
        for tx in transactions {
            match self.process_transaction(tx).await {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(e) => {
                    error!("Failed to process transaction {}: {:?}", tx.id, e);
                    self.mark_transaction_failed(&tx, &e.to_string()).await?;
                }
            }
        }
        
        Ok(processed_count)
    }

    // Fetch transactions marked as "ready for relay"
    async fn fetch_ready_transactions(&self) -> Result<Vec<L2Transaction>, StarknetRelayerError> {
        let transactions = sqlx::query_as!(
            L2Transaction,
            r#"
            SELECT * FROM l2_transactions
            WHERE status = 'ready_for_relay'
            ORDER BY created_at ASC
            LIMIT 10
            "#
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;
        
        Ok(transactions)
    }

    // Process a single transaction
    async fn process_transaction(&self, tx: L2Transaction) -> Result<(), StarknetRelayerError> {
        info!("Processing L2 transaction {}", tx.id);
        
        // Mark transaction as processing
        self.mark_transaction_processing(&tx).await?;
        
        // Extract proof data from the transaction
        let proof_data = tx.proof_data.ok_or(StarknetRelayerError::ProofDataMissing)?;
        
        // Attempt to relay the transaction with retries
        let mut attempts = 0;
        let max_retries = self.config.max_retries;
        
        loop {
            attempts += 1;
            
            match self.relay_to_starknet(&tx, &proof_data).await {
                Ok(tx_hash) => {
                    // Wait for transaction confirmation
                    match self.wait_for_transaction_confirmation(tx_hash).await {
                        Ok(_) => {
                            // Mark transaction as completed
                            self.mark_transaction_completed(&tx, &tx_hash.to_string()).await?;
                            info!("Transaction {} successfully processed on Starknet (hash: {})", tx.id, tx_hash);
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("Transaction {} submitted but confirmation failed: {:?}", tx.id, e);
                            
                            if attempts >= max_retries {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to relay transaction {} (attempt {}/{}): {:?}", 
                        tx.id, attempts, max_retries, e);
                    
                    if attempts >= max_retries {
                        return Err(e);
                    }
                }
            }
            
            // Delay before retry
            let retry_delay = Duration::from_millis(self.config.retry_delay_ms);
            sleep(retry_delay).await;
        }
    }

    // Relay transaction to Starknet
    async fn relay_to_starknet(
        &self, 
        tx: &L2Transaction,
        proof_data: &str,
    ) -> Result<FieldElement, StarknetRelayerError> {
        // Parse proof data from JSON
        let proof: serde_json::Value = serde_json::from_str(proof_data)
            .map_err(|e| StarknetRelayerError::TransactionFailed(format!("Invalid proof data: {}", e)))?;
        
        // Extract components needed for the call
        // Note: The exact structure depends on your proof format and contract interface
        
        // Example call assuming your L2 contract has a function like:
        // func process_withdrawal(
        //     withdrawal_id: felt,  
        //     proof_data: Array<felt>,
        //     merkle_root: felt
        // )
        
        let mut calldata = vec![
            withdrawal_id,
            FieldElement::from_dec_str(&proof_array.len().to_string())
                .map_err(|_| StarknetRelayerError::TransactionFailed("Invalid proof length".to_string()))?,
        ];
        
        calldata.extend(proof_array.clone());
        calldata.push(merkle_root);
        
        let calls = vec![Call {
            to: FieldElement::from_hex_be(&self.config.bridge_contract_address)
                .map_err(|_| StarknetRelayerError::InvalidContractAddress)?,
            selector: cairo_short_string_to_felt("process_withdrawal")
                .map_err(|_| StarknetRelayerError::SelectorParseFailed)?,
            calldata,
        }];
        
        
        let merkle_root = match proof.get("merkle_root") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    FieldElement::from_hex_be(s)
                        .map_err(|_| StarknetRelayerError::TransactionFailed("Invalid merkle root".to_string()))?
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            },
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };
        
        // Create call to the L2 bridge contract
        let calls = vec![Call {
            to: self.account.address(), // Address of the bridge contract
            selector: cairo_short_string_to_felt("process_withdrawal").unwrap(),
            calldata: vec![
                withdrawal_id,
                FieldElement::from_dec_str(&proof_array.len().to_string()).unwrap(),
                
                // Initialize calldata with basic fields
let mut calldata = vec![
    withdrawal_id,
    FieldElement::from_dec_str(&proof_array.len().to_string()).unwrap(),
];

// Extend calldata with proof array elements
calldata.extend(proof_array.clone());

// Add merkle root at the end
calldata.push(merkle_root);

// Create call to the L2 bridge contract
let calls = vec![Call {
    to: self.account.address(), // Correctly use account address or contract address if needed
    selector: cairo_short_string_to_felt("process_withdrawal").unwrap(),
    calldata,
}];

        
        Ok(result.transaction_hash)
    }
    
    // Wait for transaction confirmation

    async fn wait_for_transaction_confirmation(
        &self,
        tx_hash: FieldElement,
    ) -> Result<(), StarknetRelayerError> {
        let timeout = Duration::from_millis(self.config.transaction_timeout_ms);
        let start_time = std::time::Instant::now();
    
        loop {
            // Timeout check
            if start_time.elapsed() > timeout {
                return Err(StarknetRelayerError::Timeout(
                    "Transaction confirmation timed out.".to_string(),
                ));
            }
    
            match self.account.provider().get_transaction_receipt(tx_hash).await {
                Ok(MaybePendingTransactionReceipt::Receipt(receipt)) => {
                    match receipt.execution_status {
                        Some(ExecutionStatus::Succeeded) => return Ok(()),
                        Some(ExecutionStatus::Reverted) => {
                            return Err(StarknetRelayerError::TransactionFailed(
                                "Transaction reverted".to_string(),
                            ));
                        }
                        _ => {
                            // Still pending or failed in an unknown way — keep polling
                        }
                    }
                }
                Ok(MaybePendingTransactionReceipt::PendingReceipt(_)) => {
                    // Still pending — retry
                }
                Err(ProviderError::StarknetError(
                    starknet::providers::jsonrpc::error::StarknetError::TransactionHashNotFound,
                )) => {
                    // Hash not found yet — retry
                }
                Err(e) => return Err(StarknetRelayerError::Provider(e.into())),
            }
    
            // Sleep for a short duration before retrying
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    
        // Timeout check
        if start_time.elapsed() > timeout {
            return Err(StarknetRelayerError::TimeoutError(
                "Transaction confirmation timed out.".to_string(),
            ));
        }

        // Sleep for a short duration before retrying
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

    // Mark transaction as processing in the database
    async fn mark_transaction_processing(&self, tx: &L2Transaction) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
            UPDATE l2_transactions
            SET status = 'processing', updated_at = NOW()
            WHERE id = $1
            "#,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;
        
        Ok(())
    }

    // Mark transaction as completed in the database
    async fn mark_transaction_completed(
        &self,
        tx: &L2Transaction,
        tx_hash: &str,
    ) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
            UPDATE l2_transactions
            SET status = 'completed', tx_hash = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            tx_hash,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;
        
        Ok(())
    }

    // Mark transaction as failed in the database
    async fn mark_transaction_failed(
        &self,
        tx: &L2Transaction,
        error_message: &str,
    ) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
            UPDATE l2_transactions
            SET status = 'failed', error = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            error_message,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;
        
        Ok(())
    }
}

impl StarknetRelayer {
    pub async fn new(
        db_pool: Pool<Postgres>,
        config: StarknetRelayerConfig,
    ) -> Result<Self, StarknetRelayerError> {
        let transport = HttpTransport::new(Url::parse(&config.rpc_url)?);
        let provider = JsonRpcClient::new(transport);

        let private_key = FieldElement::from_hex_be(&config.private_key)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));
        let account_address = signer.address();

        let chain_id = ChainId::Mainnet; // or Testnet as per your setup

        let account = SingleOwnerAccount::new(
            provider,
            signer,
            account_address,
            chain_id,
        );

        Ok(Self {
            db_pool,
            config,
            account,
        })
    }
}

