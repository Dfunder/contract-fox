//! HTTP API surface for `contract-fox`.
//!
//! Exposes the public REST endpoints that the on-chain Soroban contracts and
//! off-chain worker interact with. The current surface is:
//!
//! - `POST /api/donations/submit` – submit a signed donation transaction to
//!   Soroban RPC, poll for confirmation, then persist the record.

pub mod error;
pub mod handlers;
pub mod server;

pub use error::{ApiError, ApiErrorCode};
pub use handlers::{AppState, SubmitDonationRequest, SubmitDonationResponse, submit_donation};
pub use server::{build_router, production_state};
