// Contract Fox Worker - Background service for contract management
mod logging;
mod redaction;
mod db;
mod horizon;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use db::cursor_store::CursorStore;
use horizon::client::HorizonClient;
use rusqlite::Connection;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub rpc_url: String,
    pub horizon_url: String,
    pub poll_interval: u64,
    pub network: String,
    pub tracked_addresses: Vec<String>, // List of stellar addresses to poll for payments
}

impl WorkerConfig {
    pub fn log_safe(&self) -> serde_json::Value {
        redaction::log_safe_config(&serde_json::json!({
            "rpc_url": self.rpc_url,
            "horizon_url": self.horizon_url,
            "poll_interval": self.poll_interval,
            "network": self.network,
            "tracked_addresses": self.tracked_addresses
        }))
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://soroban-testnet.stellar.org/".to_string(),
            horizon_url: "https://horizon-testnet.stellar.org".to_string(),
            poll_interval: 30,
            network: "testnet".to_string(),
            tracked_addresses: vec![],
        }
    }
}

pub async fn run_worker(config: WorkerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting contract fox worker");
    debug!(
        "Worker configuration: {}",
        serde_json::to_string_pretty(&config.log_safe())
            .unwrap_or_else(|_| "Failed to serialize config".to_string())
    );

    // Initialize database connection
    let conn = Connection::open("contract_fox_worker.db")?;
    let cursor_store = CursorStore::new(conn)?;
    info!("Cursor store initialized successfully");

    // Initialize Horizon client
    let horizon_client = HorizonClient::new(&config.horizon_url);
    info!("Horizon client initialized with URL: {}", config.horizon_url);

    loop {
        debug!("Polling for contract updates");

        match tokio::time::timeout(
            tokio::time::Duration::from_secs(config.poll_interval),
            check_contracts(&config, &cursor_store, &horizon_client),
        )
        .await
        {
            Ok(result) => {
                if let Err(e) = result {
                    error!("Error checking contracts: {}", e);
                }
            }
            Err(_) => {
                debug!("Polling timeout reached, continuing to next iteration");
            }
        }
    }
}

async fn check_contracts(
    config: &WorkerConfig,
    cursor_store: &CursorStore,
    horizon_client: &HorizonClient,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Checking contracts on network: {}", config.network);

    for address in &config.tracked_addresses {
        debug!("Polling address: {}", address);
        
        // Get the last saved cursor for this address (initializes to "now" if new)
        let cursor = cursor_store.get_cursor(address)?;
        debug!("Using cursor for {}: {}", address, cursor);

        // Fetch payments from Horizon with the cursor
        let payment_page = horizon_client.get_payments(address, Some(&cursor)).await?;
        debug!("Found {} payments for {}", payment_page._embedded.records.len(), address);

        // Process new payments
        let mut latest_cursor = cursor.clone();
        for payment in payment_page._embedded.records {
            debug!("Processing payment: {}", payment.id);
            // Here you would add your payment processing logic (verification, saving to DB, etc.)
            
            // Update the latest cursor to the most recent payment's paging_token
            if payment.paging_token > latest_cursor {
                latest_cursor = payment.paging_token;
            }
        }

        // Save the new cursor if we processed any payments
        if latest_cursor != cursor {
            cursor_store.save_cursor(address, &latest_cursor)?;
            info!("Updated cursor for {} to {}", address, latest_cursor);
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init_logging()?;

    let config = WorkerConfig::default();
    info!("Worker initialized with default configuration");

    run_worker(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorkerConfig::default();
        assert_eq!(config.network, "testnet");
        assert_eq!(config.poll_interval, 30);
    }
}