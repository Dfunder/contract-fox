pub mod api;
pub mod config;
pub mod db;
pub mod errors;
pub mod models;

pub mod friendbot;
pub mod horizon;
pub mod services;
pub mod setup;
pub mod soroban;
pub mod utils;
pub mod webhooks;

use std::sync::Arc;

use crate::api::production_state;
use crate::db::donations_repo::DonationsRepo;
use crate::errors::StellarAidError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load and validate required env config.
    let config = config::Config::from_env()?;

    // 2. Initialize tracing. Honour `RUST_LOG`, defaulting to `info`.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    // 3. Resolve listening address + DB path (env-overridable with sensible defaults).
    let bind = std::env::var("HTTP_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "stellar-aid.db".into());

    tracing::info!(target: "contract_fox::startup", %bind, %db_path, "boot");

    // 4. Open the SQLite DB and build the donations repo.
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| StellarAidError::DatabaseError(format!("open {db_path}: {e}")))?;
    let repo: Arc<DonationsRepo> = Arc::new(
        DonationsRepo::new(conn)
            .map_err(|e| StellarAidError::DatabaseError(format!("init schema: {e}")))?,
    );

    // 5. Wire the AppState and router.
    let state = production_state(&config.soroban_rpc_url, repo);
    let app = api::build_router(state);

    // 6. Bind TCP and serve.
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| StellarAidError::NetworkError(format!("bind {bind}: {e}")))?;
    tracing::info!(target: "contract_fox::startup", %bind, "listening");

    axum::serve(listener, app)
        .await
        .map_err(|e| StellarAidError::NetworkError(format!("serve: {e}")))?;

    Ok(())
}
