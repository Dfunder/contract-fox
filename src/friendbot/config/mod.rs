use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    Testnet,
    Futurenet,
    Mainnet,
}

impl Network {
    /// Parse from the `STELLAR_NETWORK` environment variable.
    /// Accepts: "testnet", "futurenet", "mainnet" (case-insensitive).
    /// Defaults to `Testnet` if the variable is unset or unrecognised.
    pub fn from_env() -> Self {
        let raw = env::var("STELLAR_NETWORK").unwrap_or_default();
        match raw.to_lowercase().as_str() {
            "mainnet" => Self::Mainnet,
            "futurenet" => Self::Futurenet,
            "testnet" => Self::Testnet,
            other => {
                if !other.is_empty() {
                    eprintln!(
                        "[stellar-aid] Unknown STELLAR_NETWORK={:?}, defaulting to Testnet",
                        other
                    );
                }
                Self::Testnet
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Human-readable label for log output.
    pub name: &'static str,
    /// Stellar network passphrase used when signing transactions.
    pub network_passphrase: &'static str,
    /// Horizon REST API base URL.
    #[allow(dead_code)]
    pub horizon_url: &'static str,
    /// Soroban JSON-RPC base URL.
    #[allow(dead_code)]
    pub soroban_rpc_url: &'static str,
    /// Friendbot funding URL (`None` on Mainnet).
    pub friendbot_url: Option<&'static str>,
}

impl NetworkConfig {
    pub fn testnet() -> Self {
        Self {
            name: "Testnet",
            network_passphrase: "Test SDF Network ; September 2015",
            horizon_url: "https://horizon-testnet.stellar.org",
            soroban_rpc_url: "https://soroban-testnet.stellar.org",
            friendbot_url: Some("https://friendbot.stellar.org"),
        }
    }

    pub fn futurenet() -> Self {
        Self {
            name: "Futurenet",
            network_passphrase: "Test SDF Future Network ; October 2022",
            horizon_url: "https://horizon-futurenet.stellar.org",
            soroban_rpc_url: "https://rpc-futurenet.stellar.org",
            friendbot_url: Some("https://friendbot-futurenet.stellar.org"),
        }
    }

    pub fn mainnet() -> Self {
        Self {
            name: "Mainnet",
            network_passphrase: "Public Global Stellar Network ; September 2015",
            horizon_url: "https://horizon.stellar.org",
            soroban_rpc_url: "https://soroban-rpc.mainnet.stellar.gateway.fm",
            friendbot_url: None,
        }
    }

    pub fn from_env() -> Self {
        match Network::from_env() {
            Network::Mainnet => Self::mainnet(),
            Network::Futurenet => Self::futurenet(),
            Network::Testnet => Self::testnet(),
        }
    }

    #[allow(dead_code)]
    pub fn log_startup(&self) {
        println!("[stellar-aid] Active network : {}", self.name);
        println!("[stellar-aid]   horizon_url     = {}", self.horizon_url);
        println!("[stellar-aid]   soroban_rpc_url = {}", self.soroban_rpc_url);
        println!(
            "[stellar-aid]   friendbot       = {}",
            self.friendbot_url.unwrap_or("N/A")
        );
    }

    pub fn has_friendbot(&self) -> bool {
        self.friendbot_url.is_some()
    }
}

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
mod tests;
