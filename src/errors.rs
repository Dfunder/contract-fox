//! Unified error types for the StellarAid blockchain integration layer.
//!
//! [`StellarAidError`] is the top-level error enum that downstream code should
//! use as its return type.  Each variant maps to a distinct failure domain and
//! carries enough context for callers to decide how to recover (or surface a
//! useful message).
//!
//! Existing per-module error types ([`KeyError`](crate::utils::keypair::KeyError),
//! [`StellarError`](crate::friendbot::utils::types::StellarError),
//! [`TokenSetupError`](crate::setup::token_setup::TokenSetupError)) are
//! automatically converted via [`From`] impls so the `?` operator works
//! transparently.

use thiserror::Error;

use crate::friendbot::utils::types::StellarError;
use crate::setup::token_setup::TokenSetupError;
use crate::utils::keypair::KeyError;

/// Top-level error type for the StellarAid integration layer.
#[derive(Debug, Error)]
pub enum StellarAidError {
    /// Stellar Horizon REST-API returned an error or an unexpected response.
    #[error("Horizon API error: {0}")]
    HorizonError(String),

    /// Soroban JSON-RPC call failed.
    #[error("Soroban RPC error (code {code}): {message}")]
    SorobanError { code: i64, message: String },

    /// Key generation, parsing, or derivation failed.
    #[error("Keypair error: {0}")]
    KeypairError(String),

    /// Input or state did not meet a business-logic precondition.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// A submitted transaction was rejected or reverted by the network.
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// An on-chain smart-contract call returned an error.
    #[error("Contract error: {0}")]
    ContractError(String),

    /// A lower-level network / HTTP / I/O error.
    #[error("Network error: {0}")]
    NetworkError(String),
}

// ── From impls for ergonomic `?` propagation ────────────────────────────────

impl From<KeyError> for StellarAidError {
    fn from(err: KeyError) -> Self {
        Self::KeypairError(err.to_string())
    }
}

impl From<StellarError> for StellarAidError {
    fn from(err: StellarError) -> Self {
        match &err {
            StellarError::FriendbotNotAvailable { .. } => Self::NetworkError(err.to_string()),
            StellarError::HttpRequestFailed(_) => Self::NetworkError(err.to_string()),
            StellarError::FriendbotError { .. } => Self::HorizonError(err.to_string()),
        }
    }
}

impl From<TokenSetupError> for StellarAidError {
    fn from(err: TokenSetupError) -> Self {
        Self::ContractError(err.to_string())
    }
}

impl From<reqwest::Error> for StellarAidError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::friendbot::utils::types::StellarError;
    use crate::setup::token_setup::TokenSetupError;
    use crate::utils::keypair::KeyError;

    #[test]
    fn key_error_converts_to_keypair_variant() {
        let err: StellarAidError = KeyError::InvalidSecretKey("bad".into()).into();
        assert!(matches!(err, StellarAidError::KeypairError(_)));
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn stellar_friendbot_not_available_converts_to_network() {
        let err: StellarAidError = StellarError::FriendbotNotAvailable {
            network: "mainnet".into(),
        }
        .into();
        assert!(matches!(err, StellarAidError::NetworkError(_)));
    }

    #[test]
    fn stellar_http_failure_converts_to_network() {
        let err: StellarAidError = StellarError::HttpRequestFailed("timeout".into()).into();
        assert!(matches!(err, StellarAidError::NetworkError(_)));
    }

    #[test]
    fn stellar_friendbot_error_converts_to_horizon() {
        let err: StellarAidError = StellarError::FriendbotError {
            status: 500,
            body: "internal".into(),
        }
        .into();
        assert!(matches!(err, StellarAidError::HorizonError(_)));
    }

    #[test]
    fn token_setup_error_converts_to_contract() {
        let err: StellarAidError = TokenSetupError::CommandFailed {
            command: "stellar deploy".into(),
            stderr: "oops".into(),
        }
        .into();
        assert!(matches!(err, StellarAidError::ContractError(_)));
    }

    #[test]
    fn display_includes_variant_prefix() {
        let err = StellarAidError::ValidationError("amount must be positive".into());
        assert_eq!(err.to_string(), "Validation error: amount must be positive");
    }

    #[test]
    fn soroban_error_formats_code_and_message() {
        let err = StellarAidError::SorobanError {
            code: -32600,
            message: "invalid request".into(),
        };
        assert!(err.to_string().contains("-32600"));
        assert!(err.to_string().contains("invalid request"));
    }
}
