use anyhow::Result;
use mockall::predicate::*;
use mockall::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use starknet::core::types::{EmittedEvent, EventFilter, EventsPage, Felt};
use zeroxbridge_sequencer::config::{
    AppConfig, ContractConfig, Contracts, DatabaseConfig, EthereumConfig, LoggingConfig,
    MerkleConfig, OracleConfig, QueueConfig, RelayerConfig, ServerConfig, StarknetConfig,
};
use zeroxbridge_sequencer::events::fetch_l2_burn_events;
use zeroxbridge_sequencer::events::l2_event_watcher::TestProvider;

// Mock the Starknet Provider
mock! {
    pub StarknetProvider {
        fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
        fn get_events(
            &self,
            filter: EventFilter,
            continuation_token: Option<String>,
            chunk_size: u64,
        ) -> Result<EventsPage, Box<dyn std::error::Error + Send + Sync>>;
    }
    impl Clone for StarknetProvider {
        fn clone(&self) -> Self;
    }
}

impl TestProvider for MockStarknetProvider {
    fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        self.block_number()
    }

    fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<EventsPage, Box<dyn std::error::Error + Send + Sync>> {
        self.get_events(filter, continuation_token, chunk_size)
    }
}

// Test module to group all L2 event watcher tests
#[cfg(test)]
mod tests {
    use zeroxbridge_sequencer::config::HerodotusConfig;

    use super::*;

    // Helper function to create test database pool
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5435/zeroxdb".to_string());

        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    // Helper function to create test config
    pub fn create_test_config() -> AppConfig {
        AppConfig {
            contract: ContractConfig {
                name: String::new(),
            },
            contracts: Contracts {
                l1_contract_address: "0x123".to_string(),
                l2_contract_address: "0x456".to_string(),
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                server_url: "http://localhost:8080".to_string(),
            },
            database: DatabaseConfig { max_connections: 5 },
            ethereum: EthereumConfig {
                chain_id: 11155111, // Sepolia testnet
                confirmations: 1,
            },
            starknet: StarknetConfig {
                chain_id: "SN_SEPOLIA".to_string(),
            },
            relayer: RelayerConfig {
                max_retries: 3,
                retry_delay_seconds: 60,
                gas_limit: 300000,
            },
            queue: QueueConfig {
                process_interval_sec: 60,
                wait_time_seconds: 30,
                max_retries: 3,
                initial_retry_delay_sec: 60,
                retry_delay_seconds: 60,
                merkle_update_confirmations: 1,
            },
            merkle: MerkleConfig {
                tree_depth: 32,
                cache_size: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: "logs/zeroxbridge.log".to_string(),
            },
            oracle: OracleConfig {
                tolerance_percent: Some(0.01), // 1% tolerance
                polling_interval_seconds: 60,
            },
            herodotus: HerodotusConfig {
                herodotus_endpoint: "23478234".to_string(),
            },
        }
    }

    // Helper function to create a test event
    fn create_test_event(
        block_number: u64,
        tx_hash: &str,
        user: &str,
        amount_low: &str,
        amount_high: &str,
        commitment: &str,
    ) -> EmittedEvent {
        EmittedEvent {
            from_address: Felt::from_hex("0x456").unwrap(),
            keys: vec![Felt::from_hex(
                "0x0099de3f38fed0a76764f614c6bc2b958814813685abc1af6deedab612df44f3",
            )
            .unwrap()],
            data: vec![
                Felt::from_hex(user).unwrap(),
                Felt::from_hex(amount_low).unwrap(),
                Felt::from_hex(amount_high).unwrap(),
                Felt::from_hex(commitment).unwrap(),
            ],
            block_number: Some(block_number),
            block_hash: None,
            transaction_hash: Felt::from_hex(tx_hash).unwrap(),
        }
    }

    #[tokio::test]
    async fn test_commitment_hash_decoding() -> Result<()> {
        let pool = setup_test_db().await;
        let config = create_test_config();
        let mut mock_provider = MockStarknetProvider::new();

        // Mock block number response
        mock_provider.expect_block_number().returning(|| Ok(100));

        // Create test events
        let test_events = vec![
            create_test_event(95, "0x123", "0x1234567890abcdef", "0x1000", "0x0", "0xabc"),
            create_test_event(96, "0x456", "0xfedcba0987654321", "0x2000", "0x0", "0xdef"),
        ];

        // Mock get_events response
        mock_provider.expect_get_events().returning(move |_, _, _| {
            Ok(EventsPage {
                events: test_events.clone(),
                continuation_token: None,
            })
        });

        let result = fetch_l2_burn_events(&config, &pool, 90, &mock_provider).await?;

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].commitment_hash, "0xabc");
        assert_eq!(result[0].block_number, 95);
        assert_eq!(result[0].transaction_hash, "0x123");
        assert_eq!(result[1].commitment_hash, "0xdef");
        assert_eq!(result[1].block_number, 96);
        assert_eq!(result[1].transaction_hash, "0x456");

        Ok(())
    }

    #[tokio::test]
    async fn test_block_index_tracking() -> Result<()> {
        let pool = setup_test_db().await;
        let config = create_test_config();
        let mut mock_provider = MockStarknetProvider::new();

        mock_provider.expect_block_number().returning(|| Ok(100));

        let test_events = vec![create_test_event(
            95,
            "0x123",
            "0x1234567890abcdef",
            "0x1000",
            "0x0",
            "0xabc",
        )];

        mock_provider
            .expect_get_events()
            .times(1)
            .returning(move |_, _, _| {
                Ok(EventsPage {
                    events: test_events.clone(),
                    continuation_token: None,
                })
            });

        // First call: should process blocks 90-95
        let result = fetch_l2_burn_events(&config, &pool, 90, &mock_provider).await?;
        assert_eq!(result.len(), 1);

        // Verify block tracker was updated
        let last_block = sqlx::query!(
            "SELECT last_block FROM l2_block_trackers WHERE key = 'l2_burn_events_last_block'"
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(result[0].block_number, 95);
        assert_eq!(last_block.last_block, 95);

        Ok(())
    }

    #[tokio::test]
    async fn test_large_amount_handling() -> Result<()> {
        let pool = setup_test_db().await;
        let config = create_test_config();
        let mut mock_provider = MockStarknetProvider::new();

        mock_provider.expect_block_number().returning(|| Ok(100));

        let test_events = vec![create_test_event(
            95,
            "0x123",
            "0x1234567890abcdef",
            "0xffffffffffffffff",
            "0x1",
            "0xabc",
        )];

        mock_provider.expect_get_events().returning(move |_, _, _| {
            Ok(EventsPage {
                events: test_events.clone(),
                continuation_token: None,
            })
        });

        let result = fetch_l2_burn_events(&config, &pool, 92, &mock_provider).await?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].commitment_hash, "0xabc");
        assert_eq!(result[0].block_number, 95);
        assert_eq!(result[0].transaction_hash, "0x123");
        assert_eq!(result[0].amount_low, "0xffffffffffffffff");
        assert_eq!(result[0].amount_high, "0x1");

        Ok(())
    }
}
