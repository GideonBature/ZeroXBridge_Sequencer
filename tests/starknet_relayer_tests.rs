#[cfg(test)]
mod tests {
    use crate::relayer::starknet_relayer::{StarknetRelayer, StarknetRelayerConfig};
    use crate::queue::l2_queue::L2Transaction;
    use mockall::predicate::*;
    use mockall::mock;
    use sqlx::{Pool, Postgres};
    use std::sync::Arc;
    
    // Mock the Starknet provider
    mock! {
        pub StarknetProvider {}
        
        impl Clone for StarknetProvider {
            fn clone(&self) -> Self;
        }
        
        impl StarknetProvider {
            fn execute_transaction(&self, tx_hash: String) -> Result<String, String>;
            fn get_transaction_receipt(&self, tx_hash: String) -> Result<bool, String>;
        }
    }
    
    // Helper function to create a test database pool
    async fn create_test_db_pool() -> Pool<Postgres> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/testdb".to_string());
        
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database")
    }
    
    // Helper function to create sample L2Transaction
    fn create_sample_l2_transaction() -> L2Transaction {
        L2Transaction {
            id: 1,
            user_address: "0x1234567890".to_string(),
            amount: "1000000000000000000".to_string(),
            token_address: "0xabcdef1234567890".to_string(),
            status: "ready_for_relay".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tx_hash: None,
            error: None,
            proof_data: Some(r#"{
                "proof_array": ["0x1", "0x2", "0x3"],
                "merkle_root": "0xabcdef123456789"
            }"#.to_string()),
        }
    }
    
    #[tokio::test]
    async fn test_fetch_ready_transactions() {
        // Create database pool with a transaction
        let pool = create_test_db_pool().await;
        let mut tx = pool.begin().await.expect("Failed to begin transaction");
        
        // Insert a test transaction
        let test_tx = create_sample_l2_transaction();
        sqlx::query!(
            r#"
            INSERT INTO l2_transactions 
            (id, user_address, amount, token_address, status, created_at, updated_at, proof_data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            test_tx.id,
            test_tx.user_address,
            test_tx.amount,
            test_tx.token_address,
            test_tx.status,
            test_tx.created_at,
            test_tx.updated_at,
            test_tx.proof_data
        )
        .execute(&mut *tx)
        .await
        .expect("Failed to insert test transaction");
        
        // Create relayer config
        let config = StarknetRelayerConfig {
            bridge_contract_address: "0x1234567890abcdef".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            transaction_timeout_ms: 30000,
        };
        
        // Create a mock relayer with a customized fetch_ready_transactions method
        let mock_relayer = MockStarknetRelayer::new(pool.clone(), config);
        
        // Execute the test
        let transactions = mock_relayer.fetch_ready_transactions().await.unwrap();
        
        // Verify results
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].id, test_tx.id);
        assert_eq!(transactions[0].status, "ready_for_relay");
        
        // Rollback the transaction to clean up
        tx.rollback().await.expect("Failed to rollback transaction");
    }
    
    #[tokio::test]
    async fn test_process_transaction_success() {
        // Create database pool
        let pool = create_test_db_pool().await;
        
        // Create a mock provider
        let mut mock_provider = MockStarknetProvider::new();
        mock_provider
            .expect_execute_transaction()
            .returning(|_| Ok("0xsuccesstxhash".to_string()));
        mock_provider
            .expect_get_transaction_receipt()
            .returning(|_| Ok(true));
        
        // Create relayer config
        let config = StarknetRelayerConfig {
            bridge_contract_address: "0x1234567890abcdef".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            max_retries: 3,
            retry_delay_ms: 100,
            transaction_timeout_ms: 1000,
        };
        
        // Create test transaction
        let test_tx = create_sample_l2_transaction();
        
        // Create a mock relayer with customized methods
        let mock_relayer = MockStarknetRelayer::new_with_provider(pool.clone(), config, Arc::new(mock_provider));
        
        // Execute the test
        let result = mock_relayer.process_transaction(test_tx.clone()).await;
        
        // Verify results
        assert!(result.is_ok());
        
        // Verify the transaction was marked as completed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");
        
        assert_eq!(updated_tx.status, "completed");
        assert_eq!(updated_tx.tx_hash, Some("0xsuccesstxhash".to_string()));
    }
    
    #[tokio::test]
    async fn test_process_transaction_with_retries() {
        // Create database pool
        let pool = create_test_db_pool().await;
        
        // Create a mock provider that fails twice then succeeds
        let mut mock_provider = MockStarknetProvider::new();
        mock_provider
            .expect_execute_transaction()
            .times(3)
            .returning(|call_count| {
                static mut CALL_COUNT: usize = 0;
                unsafe {
                    CALL_COUNT += 1;
                    if CALL_COUNT <= 2 {
                        Err("Temporary failure".to_string())
                    } else {
                        Ok("0xsuccesstxhash".to_string())
                    }
                }
            });
        mock_provider
            .expect_get_transaction_receipt()
            .returning(|_| Ok(true));
        
        // Create relayer config with retries
        let config = StarknetRelayerConfig {
            bridge_contract_address: "0x1234567890abcdef".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            max_retries: 3,
            retry_delay_ms: 100,
            transaction_timeout_ms: 1000,
        };
        
        // Create test transaction
        let test_tx = create_sample_l2_transaction();
        
        // Create a mock relayer
        let mock_relayer = MockStarknetRelayer::new_with_provider(pool.clone(), config, Arc::new(mock_provider));
        
        // Execute the test
        let result = mock_relayer.process_transaction(test_tx.clone()).await;
        
        // Verify results
        assert!(result.is_ok());
        
        // Verify the transaction was marked as completed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");
        
        assert_eq!(updated_tx.status, "completed");
    }
    
    #[tokio::test]
    async fn test_process_transaction_failure() {
        // Create database pool
        let pool = create_test_db_pool().await;
        
        // Create a mock provider that always fails
        let mut mock_provider = MockStarknetProvider::new();
        mock_provider
            .expect_execute_transaction()
            .returning(|_| Err("Critical failure".to_string()));
        
        // Create relayer config
        let config = StarknetRelayerConfig {
            bridge_contract_address: "0x1234567890abcdef".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            max_retries: 3,
            retry_delay_ms: 100,
            transaction_timeout_ms: 1000,
        };
        
        // Create test transaction
        let test_tx = create_sample_l2_transaction();
        
        // Create a mock relayer
        let mock_relayer = MockStarknetRelayer::new_with_provider(pool.clone(), config, Arc::new(mock_provider));
        
        // Execute the test
        let result = mock_relayer.process_transaction(test_tx.clone()).await;
        
        // Verify results
        assert!(result.is_err());
        
        // Verify the transaction was marked as failed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");
        
        assert_eq!(updated_tx.status, "failed");
        assert!(updated_tx.error.is_some());
    }
}