use super::test_helpers::{with_network, without_network};
use super::*;

#[test]
fn testnet_has_correct_passphrase() {
    let cfg = NetworkConfig::testnet();
    assert_eq!(cfg.network_passphrase, "Test SDF Network ; September 2015");
}

#[test]
fn mainnet_has_no_friendbot() {
    let cfg = NetworkConfig::mainnet();
    assert!(!cfg.has_friendbot());
}

#[test]
fn testnet_and_futurenet_have_friendbot() {
    assert!(NetworkConfig::testnet().has_friendbot());
    assert!(NetworkConfig::futurenet().has_friendbot());
}

#[test]
fn from_env_defaults_to_testnet() {
    without_network(|| {
        let cfg = NetworkConfig::from_env();
        assert_eq!(cfg.name, "Testnet");
    });
}

#[test]
fn from_env_picks_mainnet() {
    with_network("mainnet", || {
        let cfg = NetworkConfig::from_env();
        assert_eq!(cfg.name, "Mainnet");
    });
}

#[test]
fn from_env_picks_futurenet_case_insensitive() {
    with_network("FUTURENET", || {
        let cfg = NetworkConfig::from_env();
        assert_eq!(cfg.name, "Futurenet");
    });
}
