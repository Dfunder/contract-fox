//! Router construction and convenience `AppState` factories.
//!
//! `build_router` is the single source of truth for which routes exist.
//! `production_state` wires the real `SorobanRpcClient` and a `DonationsRepo`
//! together with the default 10 second polling policy.

use std::sync::Arc;
use std::time::Duration;

use axum::{Router, routing::post};

use super::handlers::{AppState, submit_donation};
use crate::db::donations_repo::DonationsRepo;
use crate::soroban::rpc_client::{SorobanRpc, SorobanRpcClient};

/// Default poll policy: up to 10 attempts at 1-second intervals (≈10s total),
/// matching Soroban's ~5-second ledger close time with sufficient leeway for
/// slow RPC nodes.
pub const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 10;
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Build the [`axum::Router`] hosting the public REST endpoints.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/donations/submit", post(submit_donation))
        .with_state(state)
}

/// Build a production [`AppState`] from a Soroban RPC endpoint and a repo.
/// Polling uses the [`DEFAULT_POLL_MAX_ATTEMPTS`] / [`DEFAULT_POLL_INTERVAL`]
/// policy.
pub fn production_state(rpc_endpoint: impl Into<String>, repo: Arc<DonationsRepo>) -> AppState {
    let rpc: Arc<dyn SorobanRpc> = Arc::new(SorobanRpcClient::new(rpc_endpoint));
    AppState::new(rpc, repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    use crate::db::donations_repo::DonationsRepo;

    #[test]
    fn production_state_uses_default_polling() {
        let conn = Connection::open_in_memory().unwrap();
        let repo = Arc::new(DonationsRepo::new(conn).unwrap());
        let state = production_state("http://test", repo);
        assert_eq!(state.poll_max_attempts, DEFAULT_POLL_MAX_ATTEMPTS);
        assert_eq!(state.poll_interval, DEFAULT_POLL_INTERVAL);
    }
}
