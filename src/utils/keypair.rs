//! Stellar key validation and derivation helpers.
//!
//! Wraps [`stellar_strkey`] to provide simple, boolean-returning predicates
//! for public and secret keys, plus a helper that derives a public key from
//! a secret key.

use ed25519_dalek::SigningKey;
use stellar_strkey::{Strkey, ed25519};
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur during key operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyError {
    /// The supplied secret key string is not a valid Stellar secret key.
    #[error("invalid secret key: {0}")]
    InvalidSecretKey(String),

    /// The supplied public key string is not a valid Stellar public key.
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),
}

// ── Validation helpers ────────────────────────────────────────────────────────

/// Returns `true` if `key` is a valid Stellar public (account) key.
///
/// A valid Stellar public key starts with `G` and is 56 characters long.
pub fn is_valid_public_key(key: &str) -> bool {
    matches!(Strkey::from_string(key), Ok(Strkey::PublicKeyEd25519(_)))
}

/// Returns `true` if `key` is a valid Stellar secret (seed) key.
///
/// A valid Stellar secret key starts with `S` and is 56 characters long.
pub fn is_valid_secret_key(key: &str) -> bool {
    matches!(Strkey::from_string(key), Ok(Strkey::PrivateKeyEd25519(_)))
}

// ── Derivation helper ─────────────────────────────────────────────────────────

/// Derives the Stellar public key (account ID) from a secret key string.
///
/// Returns the public key as a StrKey-encoded `String` (starts with `G`),
/// or a [`KeyError`] if the supplied secret key is invalid.
pub fn public_key_from_secret(secret: &str) -> Result<String, KeyError> {
    let seed = match Strkey::from_string(secret) {
        Ok(Strkey::PrivateKeyEd25519(s)) => s,
        _ => return Err(KeyError::InvalidSecretKey(secret.to_owned())),
    };

    let signing_key = SigningKey::from_bytes(&seed.0);
    let verifying_key: ed25519_dalek::VerifyingKey = signing_key.verifying_key();

    let public = ed25519::PublicKey(verifying_key.to_bytes());
    Ok(Strkey::PublicKeyEd25519(public).to_string())
}

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Generate a valid (secret, public) key pair string from a deterministic seed.
/// Only used in tests — the seed bytes are arbitrary but fixed.
#[cfg(test)]
fn make_keypair(seed_byte: u8) -> (String, String) {
    let seed_bytes = [seed_byte; 32];
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();

    let secret = Strkey::PrivateKeyEd25519(ed25519::PrivateKey(seed_bytes)).to_string();
    let public = Strkey::PublicKeyEd25519(ed25519::PublicKey(verifying_key.to_bytes())).to_string();
    (secret, public)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_valid_public_key ───────────────────────────────────────────────────

    #[test]
    fn valid_public_key_returns_true() {
        let (_, public) = make_keypair(1);
        assert!(is_valid_public_key(&public));
    }

    #[test]
    fn second_valid_public_key_returns_true() {
        let (_, public) = make_keypair(2);
        assert!(is_valid_public_key(&public));
    }

    #[test]
    fn secret_key_is_not_a_valid_public_key() {
        let (secret, _) = make_keypair(1);
        assert!(!is_valid_public_key(&secret));
    }

    #[test]
    fn empty_string_is_not_a_valid_public_key() {
        assert!(!is_valid_public_key(""));
    }

    #[test]
    fn random_string_is_not_a_valid_public_key() {
        assert!(!is_valid_public_key("not-a-key"));
    }

    #[test]
    fn truncated_public_key_is_invalid() {
        let (_, public) = make_keypair(1);
        let truncated = &public[..public.len() - 1];
        assert!(!is_valid_public_key(truncated));
    }

    #[test]
    fn public_key_with_wrong_prefix_is_invalid() {
        let (_, public) = make_keypair(1);
        let mangled = format!("X{}", &public[1..]);
        assert!(!is_valid_public_key(&mangled));
    }

    // ── is_valid_secret_key ───────────────────────────────────────────────────

    #[test]
    fn valid_secret_key_returns_true() {
        let (secret, _) = make_keypair(1);
        assert!(is_valid_secret_key(&secret));
    }

    #[test]
    fn second_valid_secret_key_returns_true() {
        let (secret, _) = make_keypair(2);
        assert!(is_valid_secret_key(&secret));
    }

    #[test]
    fn public_key_is_not_a_valid_secret_key() {
        let (_, public) = make_keypair(1);
        assert!(!is_valid_secret_key(&public));
    }

    #[test]
    fn empty_string_is_not_a_valid_secret_key() {
        assert!(!is_valid_secret_key(""));
    }

    #[test]
    fn random_string_is_not_a_valid_secret_key() {
        assert!(!is_valid_secret_key("not-a-key"));
    }

    #[test]
    fn truncated_secret_key_is_invalid() {
        let (secret, _) = make_keypair(1);
        let truncated = &secret[..secret.len() - 1];
        assert!(!is_valid_secret_key(truncated));
    }

    #[test]
    fn secret_key_with_wrong_prefix_is_invalid() {
        let (secret, _) = make_keypair(1);
        let mangled = format!("T{}", &secret[1..]);
        assert!(!is_valid_secret_key(&mangled));
    }

    // ── public_key_from_secret ────────────────────────────────────────────────

    #[test]
    fn derives_correct_public_key_from_secret() {
        let (secret, expected_public) = make_keypair(42);
        let derived = public_key_from_secret(&secret).unwrap();
        assert_eq!(derived, expected_public);
    }

    #[test]
    fn derived_public_key_starts_with_g_and_is_56_chars() {
        let (secret, _) = make_keypair(1);
        let public = public_key_from_secret(&secret).unwrap();
        assert!(public.starts_with('G'));
        assert_eq!(public.len(), 56);
    }

    #[test]
    fn public_key_from_invalid_secret_returns_err() {
        let err = public_key_from_secret("not-a-secret").unwrap_err();
        assert_eq!(err, KeyError::InvalidSecretKey("not-a-secret".to_owned()));
    }

    #[test]
    fn public_key_from_public_key_returns_err() {
        let (_, public) = make_keypair(1);
        let err = public_key_from_secret(&public).unwrap_err();
        assert!(matches!(err, KeyError::InvalidSecretKey(_)));
    }

    #[test]
    fn public_key_from_empty_string_returns_err() {
        let err = public_key_from_secret("").unwrap_err();
        assert!(matches!(err, KeyError::InvalidSecretKey(_)));
    }
}
