use std::fmt;

/// Errors that can be returned by Stellar network utilities.
#[derive(Debug)]
pub enum StellarError {
    /// Friendbot is not available on this network (e.g. Mainnet has no faucet).
    FriendbotNotAvailable { network: String },
    /// The HTTP request to Friendbot could not be sent (network error, timeout, …).
    HttpRequestFailed(String),
    /// Friendbot replied with an unexpected error status.
    FriendbotError { status: u16, body: String },
}

impl fmt::Display for StellarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FriendbotNotAvailable { network } => write!(
                f,
                "Friendbot is not available on {network}. \
                 Account funding is only supported on testnet and futurenet."
            ),
            Self::HttpRequestFailed(msg) => {
                write!(f, "HTTP request to Friendbot failed: {msg}")
            }
            Self::FriendbotError { status, body } => {
                write!(f, "Friendbot returned HTTP {status}: {body}")
            }
        }
    }
}

impl std::error::Error for StellarError {}
