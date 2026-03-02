//! StellarAid Worker
//! 
//! This crate provides background processing utilities for the 
//! StellarAid platform, including event processing and indexing.

/// Worker configuration
pub struct WorkerConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Network passphrase
    pub network_passphrase: String,
}

impl WorkerConfig {
    /// Create a new worker configuration for testnet
    pub fn testnet() -> Self {
        Self {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
        }
    }

    /// Create a new worker configuration for mainnet
    pub fn mainnet() -> Self {
        Self {
            rpc_url: "https://soroban-rpc.stellar.org".to_string(),
            network_passphrase: "Public Global Stellar Network ; September 2015".to_string(),
        }
    }
}

/// Worker for processing blockchain events
pub struct Worker {
    config: WorkerConfig,
}

impl Worker {
    /// Create a new worker with the given configuration
    pub fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    /// Get the worker configuration
    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }

    /// Start the worker (placeholder for future implementation)
    pub fn start(&self) {
        // Worker implementation will be added here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_testnet() {
        let config = WorkerConfig::testnet();
        assert_eq!(config.network_passphrase, "Test SDF Network ; September 2015");
    }

    #[test]
    fn test_worker_config_mainnet() {
        let config = WorkerConfig::mainnet();
        assert_eq!(config.network_passphrase, "Public Global Stellar Network ; September 2015");
    }
}
