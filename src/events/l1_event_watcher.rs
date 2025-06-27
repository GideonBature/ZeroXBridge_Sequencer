use crate::db::database::{get_last_processed_block, update_last_processed_block};
use anyhow::Result;
use sqlx::PgPool;
use tracing::log::warn;

use std::str::FromStr;

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};

// Name of the table for storing block tracker
const BLOCK_TRACKER_KEY: &str = "l1_deposit_events_last_block";

sol! {
    contract ZeroXBridge {
        enum AssetType {
            ETH,
            ERC20
        }

        event DepositEvent(
            address indexed token, AssetType assetType, uint256 amount, address indexed user, bytes32 commitmentHash
        );
    }
}

pub async fn fetch_l1_deposit_events(
    db_pool: &mut PgPool,
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
) -> Result<Vec<Log<ZeroXBridge::DepositEvent>>, Box<dyn std::error::Error>> {
    let mut conn = db_pool.acquire().await?;

    // Load last processed block if available
    let from_block = match get_last_processed_block(&mut conn, BLOCK_TRACKER_KEY).await {
        Ok(Some(last_block)) => last_block + 1,
        Ok(None) => from_block,
        Err(e) => {
            warn!("Failed to get last processed block: {}", e);
            from_block
        }
    };

    let event_name = ZeroXBridge::DepositEvent::SIGNATURE;
    let logs = fetch_events_logs_at_address(rpc_url, from_block, contract_addr, event_name).await?;

    let last_log = logs.last().ok_or("No logs found")?;
    let block_number = last_log.block_number.ok_or("Block number not found")?;

    // Update the last processed block in the database
    if let Err(e) = update_last_processed_block(&mut conn, BLOCK_TRACKER_KEY, block_number).await {
        warn!("Failed to update last processed block: {}", e);
    }

    Ok(logs)
}

async fn fetch_events_logs_at_address<T>(
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
    event_name: &str,
) -> Result<Vec<Log<T>>, Box<dyn std::error::Error>>
where
    T: alloy::sol_types::SolEvent,
{
    let contract_addr = Address::from_str(contract_addr)?;

    let provider = ProviderBuilder::new().connect(rpc_url).await?;

    let filter = Filter::new()
        .address(contract_addr)
        .event(event_name)
        .from_block(from_block);

    let logs = provider.get_logs(&filter).await?;
    let decoded_logs = logs
        .into_iter()
        .map(|log| log.log_decode::<T>().map_err(|e| Box::new(e)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(decoded_logs)
}
