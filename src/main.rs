mod api;
mod queue;
mod proof_generator;
mod relayer;
mod merkle_tree;
mod oracle_service;

use crate::relayer::starknet_relayer::{StarknetRelayer, StarknetRelayerConfig};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::spawn;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    info!("Starting ZeroXBridge Sequencer");
    
    // Load configuration from environment or config file
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
        
    // Create database connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    
    // Run database migrations
    info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&db_pool).await?;
    
    // Create and start services
    let db_pool_arc = Arc::new(db_pool);
    
    // Start the Starknet Relayer service
    spawn_starknet_relayer(db_pool_arc.clone()).await?;
    
    // Start other services (API, Queue, Proof Generator, etc.)
    // ...
    
    info!("All services started successfully");
    
    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down ZeroXBridge Sequencer");
    
    Ok(())
}

async fn spawn_starknet_relayer(db_pool: Arc<Pool<Postgres>>) -> Result<(), Box<dyn Error>> {
    // Load Starknet relayer configuration
    let config = StarknetRelayerConfig {
        bridge_contract_address: env::var("STARKNET_BRIDGE_CONTRACT")
            .expect("STARKNET_BRIDGE_CONTRACT must be set"),
        rpc_url: env::var("STARKNET_RPC_URL")
            .expect("STARKNET_RPC_URL must be set"),
        private_key: env::var("STARKNET_PRIVATE_KEY")
            .expect("STARKNET_PRIVATE_KEY must be set"),
        max_retries: env::var("STARKNET_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .expect("STARKNET_MAX_RETRIES must be a valid number"),
        retry_delay_ms: env::var("STARKNET_RETRY_DELAY_MS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse()
            .expect("STARKNET_RETRY_DELAY_MS must be a valid number"),
        transaction_timeout_ms: env::var("STARKNET_TX_TIMEOUT_MS")
            .unwrap_or_else(|_| "60000".to_string())
            .parse()
            .expect("STARKNET_TX_TIMEOUT_MS must be a valid number"),
    };
    
    // Initialize the Starknet relayer
    let relayer = StarknetRelayer::new(db_pool.as_ref().clone(), config).await
        .map_err(|e| {
            error!("Failed to initialize Starknet relayer: {:?}", e);
            Box::new(e) as Box<dyn Error>
        })?;
    
    // Spawn the relayer service in a separate task
    let relayer_handle = spawn(async move {
        info!("Starting Starknet relayer service");
        if let Err(e) = relayer.start().await {
            error!("Starknet relayer service stopped with error: {:?}", e);
        }
    });
    
    info!("Starknet relayer service spawned");
    
    Ok(())
}