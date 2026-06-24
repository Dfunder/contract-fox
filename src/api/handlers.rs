//! HTTP handlers for the donation submission API.
//!
//! TODO(post-merge): wire `services::donation_verifier::verify_donation_tx`
//! into the FAILED/SUCCESS arms so off-chain trust does not depend solely on
//! the client's claims about which campaign / amount they targeted. Out of
//! scope for the initial PR per the issue spec.
//!
//! The flagship endpoint is [`submit_donation`], which:
//!
//! 1. Validates the JSON body (signed XDR, campaign id, amount, Stellar address).
//! 2. Submits the signed XDR via [`SorobanRpc::send_transaction`].
//! 3. Polls [`SorobanRpc::get_transaction_status`] until the transaction
//!    reaches a terminal state (SUCCESS / FAILED) or the poll budget is
//!    exhausted.
//! 4. On SUCCESS, persists a [`Donation`](crate::db::donations_repo::Donation)
//!    record; on FAILED or timeout, returns a structured response without
//!    persisting.
//!
//! Putting a transaction on-chain is idempotent (the
//! `SorobanRpcClient::send_transaction` call), but records are persisted with
//! `INSERT OR IGNORE` keyed on `tx_hash`, so duplicate submissions return the
//! existing record rather than creating a new one.

use std::sync::Arc;
use std::time::Duration;

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use stellar_strkey::ed25519::PublicKey;

use super::ApiError;
use super::server::{DEFAULT_POLL_INTERVAL, DEFAULT_POLL_MAX_ATTEMPTS};
use crate::db::donations_repo::{DonationsRepo, NewDonation};
use crate::soroban::rpc_client::{SorobanRpc, TransactionStatus};

/// Request body for `POST /api/donations/submit`.
///
/// `amount` is a numeric string (per the issue spec); the handler parses it
/// into a `u64` after validation.
#[derive(Debug, Deserialize)]
pub struct SubmitDonationRequest {
    /// Base64-encoded signed `TransactionEnvelope` XDR.
    pub signed_xdr: String,
    /// Campaign id the donation is intended for (must be > 0).
    pub campaign_id: u64,
    /// Donation amount encoded as a numeric string.
    pub amount: String,
    /// Stellar public address (G…) of the donor.
    pub donor_address: String,
    /// Optional on-chain memo to persist alongside the donation.
    pub memo: Option<String>,
}

/// Response body for `POST /api/donations/submit`.
///
/// `status` is always one of: `confirmed`, `failed`, `pending`.
#[derive(Debug, Serialize, PartialEq)]
pub struct SubmitDonationResponse {
    pub status: &'static str,
    pub tx_hash: String,
    pub campaign_id: String,
    pub donor_address: String,
    pub amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Shared state threaded through every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Submit + poll operations. Swappable with a mock in unit tests.
    pub rpc: Arc<dyn SorobanRpc>,
    /// Thread-safe SQLite repository (wrapped in `Arc` so each request can
    /// borrow without serialising the executor).
    pub repo: Arc<DonationsRepo>,
    /// Maximum number of poll attempts before we give up.
    pub poll_max_attempts: u32,
    /// Sleep duration between poll attempts.
    pub poll_interval: Duration,
}

impl AppState {
    /// Construct a new [`AppState`] with the default polling policy.
    pub fn new(rpc: Arc<dyn SorobanRpc>, repo: Arc<DonationsRepo>) -> Self {
        Self {
            rpc,
            repo,
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }

    /// Construct a new [`AppState`] with explicit polling policy (used by
    /// tests to keep timing tight).
    #[cfg(test)]
    pub fn with_poll_policy(
        rpc: Arc<dyn SorobanRpc>,
        repo: Arc<DonationsRepo>,
        max_attempts: u32,
        interval: Duration,
    ) -> Self {
        Self {
            rpc,
            repo,
            poll_max_attempts: max_attempts,
            poll_interval: interval,
        }
    }
}

/// `POST /api/donations/submit` — see module docs for the full flow.
pub async fn submit_donation(
    State(state): State<AppState>,
    Json(req): Json<SubmitDonationRequest>,
) -> Result<(StatusCode, Json<SubmitDonationResponse>), ApiError> {
    // ── 1. Validate ────────────────────────────────────────────────────────
    let amount = validate_request(&req)?;

    // ── 2. Submit signed XDR to Soroban RPC ─────────────────────────────────
    let send = state.rpc.send_transaction(&req.signed_xdr).await?;
    let tx_hash = send.hash;

    // Short-circuit: if Soroban rejected the submission synchronously, we can
    // skip the polling round-trip and just report the failure.
    if matches!(send.status, TransactionStatus::Failed) {
        return Ok((
            StatusCode::OK,
            Json(SubmitDonationResponse {
                status: "failed",
                tx_hash,
                campaign_id: req.campaign_id.to_string(),
                donor_address: req.donor_address,
                amount: amount.to_string(),
                ledger: None,
                error: Some("transaction rejected at submission time".to_string()),
            }),
        ));
    }

    // ── 3. Poll for terminal status ────────────────────────────────────────
    let outcome = poll_for_terminal_status(state.rpc.as_ref(), &tx_hash, &state).await?;

    // ── 4. Persist on SUCCESS, otherwise just respond ───────────────────────
    let campaign_id_str = req.campaign_id.to_string();
    let amount_str = amount.to_string();

    match outcome {
        PollOutcome::Success(ledger) => {
            // Persist inside `spawn_blocking` so the synchronous SQLite call
            // does not block the Tokio executor thread.
            let new = NewDonation {
                tx_hash: tx_hash.clone(),
                campaign_id: campaign_id_str.clone(),
                donor_address: req.donor_address.clone(),
                // `donor_user_id` stays `None` because this endpoint has no
                // auth context; `DonationsRepo.get_campaign_donations` already
                // renders such rows as \"Anonymous Donor\".
                donor_user_id: None,
                amount,
                status: "confirmed".to_string(),
                memo: req.memo.clone(),
            };
            let repo_for_save = state.repo.clone();
            tokio::task::spawn_blocking(move || repo_for_save.save_donation(&new))
                .await
                .map_err(|e| ApiError::internal(format!("persist task panicked: {e}")))?
                .map_err(|e| ApiError::internal(format!("persist donation: {e}")))?;

            Ok((
                StatusCode::OK,
                Json(SubmitDonationResponse {
                    status: "confirmed",
                    tx_hash,
                    campaign_id: campaign_id_str,
                    donor_address: req.donor_address,
                    amount: amount_str,
                    ledger,
                    error: None,
                }),
            ))
        }
        PollOutcome::Failed => Ok((
            StatusCode::OK,
            Json(SubmitDonationResponse {
                status: "failed",
                tx_hash,
                campaign_id: campaign_id_str,
                donor_address: req.donor_address,
                amount: amount_str,
                ledger: None,
                error: Some("transaction rejected on-chain".to_string()),
            }),
        )),
        PollOutcome::TimedOut => Ok((
            StatusCode::ACCEPTED,
            Json(SubmitDonationResponse {
                status: "pending",
                tx_hash,
                campaign_id: campaign_id_str,
                donor_address: req.donor_address,
                amount: amount_str,
                ledger: None,
                error: Some(
                    "polling budget exhausted; transaction still pending on the network"
                        .to_string(),
                ),
            }),
        )),
    }
}

/// Terminal outcome of polling.
enum PollOutcome {
    /// Transaction succeeded; include ledger if reported by the RPC.
    Success(Option<u64>),
    /// Transaction was rejected by the network.
    Failed,
    /// We exhausted `poll_max_attempts` without seeing a terminal status.
    TimedOut,
}

/// Poll `rpc.get_transaction_status` until SUCCESS, FAILED, or budget exhaustion.
async fn poll_for_terminal_status(
    rpc: &dyn SorobanRpc,
    hash: &str,
    state: &AppState,
) -> Result<PollOutcome, ApiError> {
    for _ in 0..state.poll_max_attempts {
        let resp = rpc.get_transaction_status(hash).await?;
        match resp.status {
            TransactionStatus::Success => return Ok(PollOutcome::Success(resp.ledger)),
            TransactionStatus::Failed => return Ok(PollOutcome::Failed),
            TransactionStatus::Pending
            | TransactionStatus::NotFound
            | TransactionStatus::Unknown => {
                tokio::time::sleep(state.poll_interval).await;
            }
        }
    }
    Ok(PollOutcome::TimedOut)
}

/// Validate the request body. Returns the parsed amount on success.
fn validate_request(req: &SubmitDonationRequest) -> Result<u64, ApiError> {
    if req.signed_xdr.trim().is_empty() {
        return Err(ApiError::bad_request("signed_xdr must not be empty"));
    }

    if req.campaign_id == 0 {
        return Err(ApiError::bad_request("campaign_id must be positive"));
    }

    let amount: u64 = req.amount.trim().parse().map_err(|_| {
        ApiError::bad_request(format!(
            "amount must be a non-negative integer encoded as a string (got {:?})",
            req.amount
        ))
    })?;
    if amount == 0 {
        return Err(ApiError::bad_request("amount must be greater than zero"));
    }

    PublicKey::from_string(&req.donor_address).map_err(|e| {
        ApiError::bad_request(format!(
            "donor_address is not a valid Stellar public key: {e}"
        ))
    })?;

    Ok(amount)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use http_body_util::BodyExt;
    use rusqlite::Connection;
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tower::ServiceExt;

    use crate::soroban::rpc_client::{RpcError, SendResult, TransactionStatusResponse};

    /// Mock RPC that returns pre-programmed responses for submit + status.
    struct MockRpc {
        /// What to return from `send_transaction`.
        send: SendResult,
        /// Hash-keyed responses for `get_transaction_status`.
        statuses: HashMap<String, Vec<TransactionStatusResponse>>,
        /// How many times `get_transaction_status` was called (for assertions).
        calls: Mutex<usize>,
    }

    impl MockRpc {
        fn new(send: SendResult, statuses: Vec<TransactionStatusResponse>) -> Self {
            let mut map = HashMap::new();
            map.insert(send.hash.clone(), statuses);
            Self {
                send,
                statuses: map,
                calls: Mutex::new(0),
            }
        }

        fn send_then_immediate_success(send_hash: &str, ledger: u64) -> Self {
            let send = SendResult {
                hash: send_hash.into(),
                status: TransactionStatus::Pending,
                error_result_xdr: None,
                latest_ledger: None,
                latest_ledger_close_time: None,
            };
            Self::new(
                send,
                vec![TransactionStatusResponse {
                    status: TransactionStatus::Success,
                    result_xdr: None,
                    result_meta_xdr: None,
                    envelope_xdr: None,
                    ledger: Some(ledger),
                    created_at: None,
                    latest_ledger: None,
                    latest_ledger_close_time: None,
                }],
            )
        }

        fn send_then_fail(send_hash: &str) -> Self {
            let send = SendResult {
                hash: send_hash.into(),
                status: TransactionStatus::Pending,
                error_result_xdr: None,
                latest_ledger: None,
                latest_ledger_close_time: None,
            };
            Self::new(
                send,
                vec![TransactionStatusResponse {
                    status: TransactionStatus::Failed,
                    result_xdr: None,
                    result_meta_xdr: None,
                    envelope_xdr: None,
                    ledger: None,
                    created_at: None,
                    latest_ledger: None,
                    latest_ledger_close_time: None,
                }],
            )
        }

        fn send_then_pending(send_hash: &str, attempts: usize) -> Self {
            let send = SendResult {
                hash: send_hash.into(),
                status: TransactionStatus::Pending,
                error_result_xdr: None,
                latest_ledger: None,
                latest_ledger_close_time: None,
            };
            let mut statuses = Vec::new();
            for _ in 0..attempts {
                statuses.push(TransactionStatusResponse {
                    status: TransactionStatus::Pending,
                    result_xdr: None,
                    result_meta_xdr: None,
                    envelope_xdr: None,
                    ledger: None,
                    created_at: None,
                    latest_ledger: None,
                    latest_ledger_close_time: None,
                });
            }
            Self::new(send, statuses)
        }

        /// Build a mock whose `send_transaction` already returns `Failed`.
        /// Exercises the early-return short-circuit in `submit_donation`
        /// without ever calling `get_transaction_status`.
        fn send_then_synchronous_failure(send_hash: &str) -> Self {
            let send = SendResult {
                hash: send_hash.into(),
                status: TransactionStatus::Failed,
                error_result_xdr: Some("bad-auth".into()),
                latest_ledger: None,
                latest_ledger_close_time: None,
            };
            // Empty `statuses` map: any `get_transaction_status` call would
            // panic, proving that the handler short-circuited.
            Self {
                send,
                statuses: HashMap::new(),
                calls: Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl SorobanRpc for MockRpc {
        async fn send_transaction(&self, _xdr: &str) -> Result<SendResult, RpcError> {
            Ok(SendResult {
                hash: self.send.hash.clone(),
                status: self.send.status,
                error_result_xdr: self.send.error_result_xdr.clone(),
                latest_ledger: self.send.latest_ledger.clone(),
                latest_ledger_close_time: self.send.latest_ledger_close_time.clone(),
            })
        }

        async fn get_transaction_status(
            &self,
            hash: &str,
        ) -> Result<TransactionStatusResponse, RpcError> {
            let _ = valid_test_public_key;

    let mut calls = self.calls.lock().unwrap();            *calls += 1;
            let queue = self
                .statuses
                .get(hash)
                .expect("no statuses programmed for hash");
            // If we run out of programmed responses, keep returning Pending.
            let idx = (*calls - 1).min(queue.len().saturating_sub(1));
            // `TransactionStatusResponse` derives `Clone` (see rpc_client.rs).
            Ok(queue[idx].clone())
        }
    }

    /// A run-time helper producing a Stellar ed25519 public-key strkey derived
    /// from the all-zeros secret. Avoids hard-coding a magic 56-char literal
    /// whose CRC cannot be verified by inspection.
    fn valid_test_public_key() -> String {
        let secret_bytes = [0u8; 32];
        let secret = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let public = secret.verifying_key();
        let pk_bytes = public.to_bytes();
        // `stellar-strkey` 0.0.8 exposes `from_payload(&[u8]) -> Result<PublicKey, _>`.
        stellar_strkey::ed25519::PublicKey::from_payload(&pk_bytes)
            .expect("stellar-strkey construction should not fail for 32 zero bytes")
            .to_string()
    }

    const TEST_SIGNED_XDR: &str = "AAAA";
    const TEST_CAMPAIGN_ID: u64 = 42;
    const TEST_AMOUNT: &str = "100";
    const TEST_MEMO: &str = "happy birthday";

    fn in_memory_repo() -> Arc<DonationsRepo> {
        let conn = Connection::open_in_memory().unwrap();
        Arc::new(DonationsRepo::new(conn).unwrap())
    }

    fn valid_request_body() -> Value {
        json!({
            "signed_xdr": TEST_SIGNED_XDR,
            "campaign_id": TEST_CAMPAIGN_ID,
            "amount": TEST_AMOUNT,
            "donor_address": valid_test_public_key(),
            "memo": TEST_MEMO
        })
    }

    fn app_router(state: AppState) -> axum::Router {
        super::super::build_router(state)
    }

    #[tokio::test]
    async fn confirms_on_chain_persists_record_and_returns_200() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx-success-1", 999));
        let state =
            AppState::with_poll_policy(rpc.clone(), repo.clone(), 5, Duration::from_millis(1));
        let app = app_router(state);
        let donor_address = valid_test_public_key();

        // `donor_address` is unused in the test body since the display logic
        // renders anonymous donors, but we still build it to assert
        // validation succeeds upstream.
        let _ = donor_address;

        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(valid_request_body().to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), AxStatus::OK);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["status"], "confirmed");
        assert_eq!(body["tx_hash"], "tx-success-1");
        assert_eq!(body["campaign_id"], "42");
        assert_eq!(body["amount"], "100");
        assert_eq!(body["ledger"], 999);

        // Verify the handler actually persisted a `confirmed` row by querying
        // the repo through its public API rather than re-saving ourselves.
        let (total_raised, count) = repo.get_campaign_stats("42").unwrap();
        assert_eq!(
            total_raised, 100,
            "confirmed donation should be counted in stats"
        );
        assert_eq!(count, 1);
        let donated = repo.get_campaign_donations("42").unwrap();
        assert_eq!(donated.len(), 1);
        assert_eq!(donated[0].1, 100);
        // The handler does not link a `donor_user_id` (no auth context in the
        // current PR), so the repo's existing display rule renders anonymous
        // donations as \"Anonymous Donor\" — that is the contract.
        assert_eq!(
            donated[0].0, "Anonymous Donor",
            "donations without a linked user display as Anonymous Donor"
        );
    }

    #[tokio::test]
    async fn reports_failed_status_without_persisting() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_fail("tx-fail-1"));
        let state = AppState::with_poll_policy(rpc, repo.clone(), 5, Duration::from_millis(1));
        let app = app_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(valid_request_body().to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), AxStatus::OK);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["status"], "failed");
        assert_eq!(body["tx_hash"], "tx-fail-1");
        assert!(body.get("ledger").is_none() || body["ledger"].is_null());

        // Verify nothing was persisted.
        let stats = repo.get_campaign_stats("42").unwrap();
        assert_eq!(stats.0, 0, "no confirmed donations should be present");
    }

    #[tokio::test]
    async fn short_circuits_when_send_returns_failed_synchronously() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_synchronous_failure("tx-sync-fail"));
        let state =
            AppState::with_poll_policy(rpc.clone(), repo.clone(), 5, Duration::from_millis(1));
        let app = app_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(valid_request_body().to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), AxStatus::OK);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["status"], "failed");
        assert_eq!(body["tx_hash"], "tx-sync-fail");
        // We never polled because the handler short-circuited on `Failed`.
        assert_eq!(
            *rpc.calls.lock().unwrap(),
            0,
            "get_transaction_status must not be called when send returned Failed"
        );
        // Nothing persisted.
        let (total, count) = repo.get_campaign_stats("42").unwrap();
        assert_eq!((total, count), (0, 0));
    }

    #[tokio::test]
    async fn returns_202_when_poll_budget_exhausted() {
        let repo = in_memory_repo();
        // 100 pending responses — way more than the 3-attempt budget below.
        let rpc = Arc::new(MockRpc::send_then_pending("tx-pending-1", 100));
        let state =
            AppState::with_poll_policy(rpc.clone(), repo.clone(), 3, Duration::from_millis(1));
        let app = app_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(valid_request_body().to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), AxStatus::ACCEPTED);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["status"], "pending");
        assert_eq!(body["tx_hash"], "tx-pending-1");
        // We polled exactly `poll_max_attempts` times.
        assert_eq!(*rpc.calls.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn rejects_empty_xdr_with_400() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx", 1));
        let state = AppState::new(rpc, repo);
        let app = app_router(state);

        let body = json!({
            "signed_xdr": "",
            "campaign_id": TEST_CAMPAIGN_ID,
            "amount": TEST_AMOUNT,
            "donor_address": valid_test_public_key()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_zero_campaign_id_with_400() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx", 1));
        let state = AppState::new(rpc, repo);
        let app = app_router(state);

        let body = json!({
            "signed_xdr": TEST_SIGNED_XDR,
            "campaign_id": 0,
            "amount": TEST_AMOUNT,
            "donor_address": valid_test_public_key()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_non_numeric_amount_with_400() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx", 1));
        let state = AppState::new(rpc, repo);
        let app = app_router(state);

        let body = json!({
            "signed_xdr": TEST_SIGNED_XDR,
            "campaign_id": TEST_CAMPAIGN_ID,
            "amount": "abc",
            "donor_address": valid_test_public_key()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_zero_amount_with_400() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx", 1));
        let state = AppState::new(rpc, repo);
        let app = app_router(state);

        let body = json!({
            "signed_xdr": TEST_SIGNED_XDR,
            "campaign_id": TEST_CAMPAIGN_ID,
            "amount": "0",
            "donor_address": valid_test_public_key()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_invalid_donor_address_with_400() {
        let repo = in_memory_repo();
        let rpc = Arc::new(MockRpc::send_then_immediate_success("tx", 1));
        let state = AppState::new(rpc, repo);
        let app = app_router(state);

        let body = json!({
            "signed_xdr": TEST_SIGNED_XDR,
            "campaign_id": TEST_CAMPAIGN_ID,
            "amount": TEST_AMOUNT,
            "donor_address": "NOT_A_STELLAR_ADDRESS"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/donations/submit")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn validate_request_accepts_valid_payload() {
        let req = SubmitDonationRequest {
            signed_xdr: TEST_SIGNED_XDR.into(),
            campaign_id: 1,
            amount: "100".into(),
            donor_address: valid_test_public_key(),
            memo: Some("m".into()),
        };
        let amount = validate_request(&req).unwrap();
        assert_eq!(amount, 100);
    }
}
