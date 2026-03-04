use super::friendbot::fund_account;
use super::types::StellarError;
use crate::friendbot::config::NetworkConfig;
use crate::friendbot::config::test_helpers::with_network;

// Mainnet guard

#[test]
fn fund_account_returns_error_on_mainnet() {
    with_network("mainnet", || {
        let result = fund_account("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN");
        assert!(
            result.is_err(),
            "Expected an error when calling fund_account on mainnet"
        );
        assert!(
            matches!(
                result.unwrap_err(),
                StellarError::FriendbotNotAvailable { .. }
            ),
            "Expected FriendbotNotAvailable variant"
        );
    });
}

#[test]
fn friendbot_not_available_display_mentions_network_and_alternatives() {
    let err = StellarError::FriendbotNotAvailable {
        network: "Mainnet".to_string(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("Mainnet"),
        "Error message should contain the network name"
    );
    assert!(
        msg.contains("testnet") || msg.contains("futurenet"),
        "Error message should mention supported networks"
    );
}

// NetworkConfig plumbing

#[test]
fn testnet_config_exposes_friendbot_url() {
    let cfg = NetworkConfig::testnet();
    assert!(
        cfg.friendbot_url.is_some(),
        "Testnet config must have a Friendbot URL"
    );
    let url = cfg.friendbot_url.unwrap();
    assert!(url.starts_with("https://"), "Friendbot URL should be HTTPS");
}

#[test]
fn mainnet_config_has_no_friendbot_url() {
    let cfg = NetworkConfig::mainnet();
    assert!(
        cfg.friendbot_url.is_none(),
        "Mainnet config must NOT have a Friendbot URL"
    );
}

#[test]
fn futurenet_config_exposes_friendbot_url() {
    let cfg = NetworkConfig::futurenet();
    assert!(
        cfg.friendbot_url.is_some(),
        "Futurenet config must have a Friendbot URL"
    );
}

// Error variants

#[test]
fn http_request_failed_display() {
    let err = StellarError::HttpRequestFailed("connection refused".to_string());
    assert!(err.to_string().contains("connection refused"));
}

#[test]
fn friendbot_error_display_includes_status_and_body() {
    let err = StellarError::FriendbotError {
        status: 500,
        body: "internal server error".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("500"));
    assert!(msg.contains("internal server error"));
}
