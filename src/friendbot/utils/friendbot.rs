use super::types::StellarError;
use crate::friendbot::config::NetworkConfig;

/// Fund a Stellar account using Friendbot.
///
/// This is **only valid on testnet and futurenet**. When called with
/// `STELLAR_NETWORK=mainnet` it immediately returns
/// [`StellarError::FriendbotNotAvailable`] — no network request is made.
///
/// The function is **idempotent**: Friendbot returns HTTP 400 with
/// `"createAccountAlreadyExist"` in the body when the account has already
/// been funded. This implementation treats that response as success so callers
/// do not need to special-case it.
///
/// # Arguments
///
/// * `public_key` – The G… Stellar public key of the account to fund.
///
/// # Errors
///
/// Returns `Err` when:
/// - The active network has no Friendbot (Mainnet) →
///   [`StellarError::FriendbotNotAvailable`].
/// - The HTTP request fails (DNS, TLS, timeout, …) →
///   [`StellarError::HttpRequestFailed`].
/// - Friendbot returns an unexpected non-200/non-already-funded response →
///   [`StellarError::FriendbotError`].
///
/// # Example
///
/// ```no_run
/// use contract_fox::friendbot::utils::friendbot::fund_account;
///
/// // STELLAR_NETWORK must be "testnet" or "futurenet"
/// fund_account("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN")
///     .expect("Friendbot funding failed");
/// ```
pub fn fund_account(public_key: &str) -> Result<(), StellarError> {
    let config = NetworkConfig::from_env();

    // Guard: Mainnet has no Friendbot; return an error immediately.
    let friendbot_base =
        config
            .friendbot_url
            .ok_or_else(|| StellarError::FriendbotNotAvailable {
                network: config.name.to_string(),
            })?;

    let url = format!("{friendbot_base}?addr={public_key}");

    let response = reqwest::blocking::get(&url)
        .map_err(|e: reqwest::Error| StellarError::HttpRequestFailed(e.to_string()))?;

    let status = response.status();

    if status.is_success() {
        return Ok(());
    }

    // HTTP 400 with "createAccountAlreadyExist" means the account is already
    // funded. Treat this as a successful no-op (idempotent behaviour).
    if status.as_u16() == 400 {
        let body = response
            .text()
            .map_err(|e: reqwest::Error| StellarError::HttpRequestFailed(e.to_string()))?;

        if body.contains("createAccountAlreadyExist") {
            return Ok(());
        }

        return Err(StellarError::FriendbotError { status: 400, body });
    }

    let body = response.text().unwrap_or_default();
    Err(StellarError::FriendbotError {
        status: status.as_u16(),
        body,
    })
}
