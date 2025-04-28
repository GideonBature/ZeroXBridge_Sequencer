use crate::config::AppConfig;
use crate::db::{get_last_processed_block, update_last_processed_block};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use starknet::{
    core::types::{BlockId, Event, EventFilter, EventsPage, FieldElement},
    providers::{Provider, ProviderError},
};
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;
const DEFAULT_PAGE_SIZE: u32 = 100;

// Name of the table for storing block tracker
const BLOCK_TRACKER_KEY: &str = "l2_burn_events_last_block";
// Event key for BurnEvent (calculated from event name "BurnEvent")
const BURN_EVENT_KEY: &str = "0x0099de3f38fed0a76764f614c6bc2b958814813685abc1af6deedab612df44f3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentLog {
    pub commitment_hash: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub user: String,
    pub amount_low: String,
    pub amount_high: String,
}

/// This function queries the L2 burn contract for events,
/// filters them by the "BurnEvent" event selector, and parses them into CommitmentLog structs.
pub async fn fetch_l2_burn_events(
    config: &AppConfig,
    db_pool: &PgPool,
    from_block: u64,
) -> Result<Vec<CommitmentLog>> {
    let provider = starknet::providers::JsonRpcHttpProvider::new(config.starknet.rpc_url.clone());
    
    // Get a connection from the pool
    let mut conn = db_pool.acquire().await?;
    
    // Load last processed block if available
    let start_block = match get_last_processed_block(&mut conn, BLOCK_TRACKER_KEY).await {
        Ok(Some(last_block)) => last_block + 1,
        Ok(None) => from_block,
        Err(e) => {
            warn!("Failed to get last processed block: {}", e);
            from_block
        }
    };
    
    // Get current block for pagination purposes
    let latest_block = get_latest_block_with_retry(&provider).await?;
    
    // Define the L2 burn contract address
    let burn_contract_address = FieldElement::from_hex_be(&config.contracts.l2_contract_address)
        .context("Invalid L2 burn contract address")?;
    
    // Create event selector for the "BurnEvent" event
    let burned_event_key = FieldElement::from_hex_be(BURN_EVENT_KEY)
        .context("Failed to create event key")?;
    
    // Define the event filter
    let event_filter = EventFilter {
        from_block: Some(BlockId::Number(start_block)),
        to_block: Some(BlockId::Number(latest_block)),
        address: Some(burn_contract_address),
        keys: vec![vec![burned_event_key]],
    };
    
    let mut all_events = Vec::new();
    let mut continuation_token = None;
    
    // Fetch events with pagination
    loop {
        let events = fetch_events_with_retry(
            &provider, 
            &event_filter, 
            continuation_token.clone(), 
            DEFAULT_PAGE_SIZE
        ).await?;
        
        if events.events.is_empty() {
            break;
        }
        
        // Parse events into CommitmentLog structs
        for event in &events.events {
            if event.data.len() >= 4 {
                // BurnEvent data structure:
                // [user, amount_low, amount_high, commitment_hash]
                let user = format!("0x{}", event.data[0].to_hex_string());
                let amount_low = format!("0x{}", event.data[1].to_hex_string());
                let amount_high = format!("0x{}", event.data[2].to_hex_string());
                let commitment_hash = format!("0x{}", event.data[3].to_hex_string());
                
                all_events.push(CommitmentLog {
                    commitment_hash,
                    block_number: event.block_number,
                    transaction_hash: format!("0x{}", event.transaction_hash.to_hex_string()),
                    user,
                    amount_low,
                    amount_high,
                });
            } else {
                warn!(
                    "Invalid event data length: expected 4, got {}",
                    event.data.len()
                );
            }
        }
        
        // Update continuation token for next page
        continuation_token = events.continuation_token;
        
        // If no continuation token, we've fetched all events
        if continuation_token.is_none() {
            break;
        }
    }
    
    // Store the latest block number we've processed
    if !all_events.is_empty() {
        let max_block = all_events.iter().map(|e| e.block_number).max().unwrap_or(start_block);
        if let Err(e) = update_last_processed_block(&mut conn, BLOCK_TRACKER_KEY, max_block).await {
            warn!("Failed to update last processed block: {}", e);
        }
    } else if latest_block > start_block {
        // Even if no events found, update the last processed block to avoid rescanning
        if let Err(e) = update_last_processed_block(&mut conn, BLOCK_TRACKER_KEY, latest_block).await {
            warn!("Failed to update last processed block: {}", e);
        }
    }
    
    info!(
        "Fetched {} L2 burn events from blocks {} to {}",
        all_events.len(),
        start_block,
        latest_block
    );
    
    Ok(all_events)
}

async fn get_latest_block_with_retry<P: Provider>(provider: &P) -> Result<u64> {
    for attempt in 1..=MAX_RETRIES {
        match provider.block_number().await {
            Ok(block) => return Ok(block),
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow!("Failed to get latest block: {}", e));
                }
                warn!("Failed to get latest block (attempt {}/{}): {}", attempt, MAX_RETRIES, e);
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }
    
    Err(anyhow!("Failed to get latest block after {} attempts", MAX_RETRIES))
}

async fn fetch_events_with_retry<P: Provider>(
    provider: &P,
    filter: &EventFilter,
    continuation_token: Option<String>,
    chunk_size: u32,
) -> Result<EventsPage> {
    for attempt in 1..=MAX_RETRIES {
        match provider
            .get_events(
                filter,
                continuation_token.clone(),
                chunk_size,
            )
            .await
        {
            Ok(events) => return Ok(events),
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow!("Failed to fetch events: {}", e));
                }
                warn!("Failed to fetch events (attempt {}/{}): {}", attempt, MAX_RETRIES, e);
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }
    
    Err(anyhow!("Failed to fetch events after {} attempts", MAX_RETRIES))
}
