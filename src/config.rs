use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;
use dotenv::dotenv;

/// Loads configuration from a given config file or environment variables.
pub fn load_config(config_file_path: Option<&Path>) -> anyhow::Result<AppConfig> {
    // Load .env file if it exists, ignore if not present
    dotenv().ok();

    let mut settings = Config::builder();

    if let Some(path) = config_file_path {
        settings = settings.add_source(File::from(path).required(true));
    }

    // Add environment variables with prefix ZEROOXBRIDGE
    settings = settings.add_source(Environment::with_prefix("ZEROOXBRIDGE").separator("__"));

    // Manually load Herodotus-specific environment variables
    let herodotus_api_key = std::env::var("HERODOTUS_API_KEY")
        .map_err(|_| anyhow::anyhow!("HERODOTUS_API_KEY is not set in environment or .env file"))?;
    
    if herodotus_api_key.is_empty() {
        return Err(anyhow::anyhow!("HERODOTUS_API_KEY cannot be empty"));
    }

    let herodotus_endpoint = std::env::var("HERODOTUS_ENDPOINT")
        .unwrap_or_else(|_| "https://staging.atlantic.api.herodotus.cloud/atlantic-query".to_string());

    // Log loaded Herodotus config for debugging (use proper logger in production)
    eprintln!(
        "Loaded Herodotus config: api_key=****, herodotus_endpoint={}",
        herodotus_endpoint
    );

    let mut app_config = settings.build()?.try_deserialize::<AppConfig>()?;

    // Override or set Herodotus config
    app_config.herodotus = HerodotusConfig {
        api_key: herodotus_api_key,
        herodotus_endpoint,
    };

    Ok(app_config)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub contract: ContractConfig,
    pub contracts: Contracts,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub starknet: StarknetConfig,
    pub prover: ProverConfig,
    pub relayer: RelayerConfig,
    pub queue: QueueConfig,
    pub merkle: MerkleConfig,
    pub logging: LoggingConfig,
    pub oracle: OracleConfig,
    pub herodotus: HerodotusConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HerodotusConfig {
    pub api_key: String,
    pub herodotus_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contracts {
    pub l1_contract_address: String,
    pub l2_contract_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub server_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub confirmations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarknetConfig {
    pub rpc_url: String,
    pub chain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProverConfig {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConfig {
    pub process_interval_sec: u64,
    pub wait_time_seconds: u32,
    pub max_retries: u32,
    pub initial_retry_delay_sec: u64,
    pub retry_delay_seconds: u32,
    pub merkle_update_confirmations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleConfig {
    pub tree_depth: u32,
    pub cache_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String, // "debug" | "info" | "warn" | "error"
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleConfig {
    pub tolerance_percent: Option<f64>, // e.g., 0.01 for 1%
    pub polling_interval_seconds: u64,  // e.g., 60 seconds
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_load_config_with_herodotus_env_vars() {
        // Set environment variables for testing
        env::set_var("HERODOTUS_API_KEY", "test_key");
        env::set_var("HERODOTUS_ENDPOINT", "https://test.api");
        env::set_var("ZEROOXBRIDGE__SERVER__HOST", "localhost");
        env::set_var("ZEROOXBRIDGE__SERVER__SERVER_URL", "http://localhost:8080");

        let config = load_config(None).expect("Failed to load config");
        assert_eq!(config.herodotus.api_key, "test_key");
        assert_eq!(config.herodotus.herodotus_endpoint, "https://test.api");
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.server_url, "http://localhost:8080");
    }

    #[test]
    fn test_load_config_missing_herodotus_api_key() {
        env::remove_var("HERODOTUS_API_KEY");
        env::set_var("HERODOTUS_ENDPOINT", "https://test.api");

        let result = load_config(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HERODOTUS_API_KEY is not set"));
    }

    #[test]
    fn test_load_config_empty_herodotus_api_key() {
        env::set_var("HERODOTUS_API_KEY", "");
        env::set_var("HERODOTUS_ENDPOINT", "https://test.api");

        let result = load_config(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HERODOTUS_API_KEY cannot be empty"));
    }

    #[test]
    fn test_load_config_default_herodotus_endpoint() {
        env::set_var("HERODOTUS_API_KEY", "test_key");
        env::remove_var("HERODOTUS_ENDPOINT");
        env::set_var("ZEROOXBRIDGE__SERVER__HOST", "localhost");
        env::set_var("ZEROOXBRIDGE__SERVER__SERVER_URL", "http://localhost:8080");

        let config = load_config(None).expect("Failed to load config");
        assert_eq!(config.herodotus.api_key, "test_key");
        assert_eq!(
            config.herodotus.herodotus_endpoint,
            "https://staging.atlantic.api.herodotus.cloud/atlantic-query"
        );
    }
}