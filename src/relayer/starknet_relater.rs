use crate::queue::l2_queue::L2Transaction;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
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

impl StarknetRelayer {
    // Initialize a new StarknetRelayer
    pub async fn new(
        db_pool: Pool<Postgres>,
        config: StarknetRelayerConfig,
    ) -> Result<Self, StarknetRelayerError> {
        // Create JsonRpcClient provider
        let provider = JsonRpcClient::new(HttpTransport::new(config.rpc_url.clone()));

        // Parse private key to create a signer
        let private_key = FieldElement::from_hex_be(&config.private_key)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        // Parse contract address
        let contract_address = FieldElement::from_hex_be(&config.bridge_contract_address)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;

        // Create Starknet account abstraction
        let chain_id = provider
            .chain_id()
            .await
            .map_err(StarknetRelayerError::Provider)?;
        
        let account = SingleOwnerAccount::new(
            provider,
            signer,
            contract_address,
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
        
        // Prepare the call parameters 
        // NOTE: This is a simplified example - adjust according to your actual contract
        let withdrawal_id = FieldElement::from_dec_str(&tx.id.to_string())
            .map_err(|_| StarknetRelayerError::TransactionFailed("Failed to convert ID".to_string()))?;
        
        // Convert proof data to felt array (simplified example)
        let proof_array = match proof.get("proof_array") {
            Some(serde_json::Value::Array(array)) => {
                let mut felt_array = Vec::new();
                for value in array {
                    if let Some(s) = value.as_str() {
                        let felt = FieldElement::from_hex_be(s)
                            .map_err(|_| StarknetRelayerError::TransactionFailed("Invalid proof element".to_string()))?;
                        felt_array.push(felt);
                    }
                }
                felt_array
            },
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };
        
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
                // Add the proof array elements
                proof_array.clone().into_iter()
                    .collect::<Vec<FieldElement>>(),
                merkle_root,
            ].into_iter().flatten().collect(),
        }];
        
        // Execute the transaction
        let result = self.account
            .execute(calls)
            .send()
            .await
            .map_err(|e| StarknetRelayerError::Provider(e))?;
        
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
            // Check if transaction has been confirmed
            match self.account.provider().get_transaction_receipt(tx_hash).await {
                Ok(receipt) => {
                    // Transaction is confirmed
                    return Ok(());
                }
                Err(e) => {
                    // Check if we've exceeded the timeout
                    if start_time.elapsed() > timeout {
                        return Err(StarknetRelayerError::TransactionTimeout);
                    }
                    
                    // If error is not just "transaction not found", propagate it
                    if !matches!(e, ProviderError::StarknetError(MaybeUnknownErrorCode::Known(StarknetErrorCode::TransactionHashNotFound))) {
                        return Err(StarknetRelayerError::Provider(e));
                    }
                    
                    // Sleep before checking again
                    sleep(Duration::from_secs(2)).await;
                }
            }
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

// HTTP Transport implementation for JsonRpcClient
struct HttpTransport {
    url: String,
}

impl HttpTransport {
    fn new(url: String) -> Self {
        Self { url }
    }
}

// Implement required traits for HttpTransport to work with JsonRpcClient
impl Transport for HttpTransport {
    fn post<T: serde::de::DeserializeOwned>(
        &self,
        request: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ProviderError>> + Send>> {
        let url = self.url.clone();
        Box::pin(async move {
            let client = reqwest::Client::new();
            let response = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| ProviderError::RateLimited(e.to_string()))?;
            
            let status = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|e| ProviderError::RateLimited(e.to_string()))?;
            
            if !status.is_success() {
                return Err(ProviderError::StarknetError(
                    MaybeUnknownErrorCode::Unknown(response_text),
                ));
            }
            
            serde_json::from_str(&response_text)
                .map_err(|e| ProviderError::SerdeError(e.to_string()))
        })
    }
}

// Missing trait implementation for Transport
pub trait Transport {
    fn post<T: serde::de::DeserializeOwned>(
        &self,
        request: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ProviderError>> + Send>>;
}