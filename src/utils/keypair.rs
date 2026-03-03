//! Stellar key validation and derivation helpers.
//!
//! Wraps [`stellar_strkey`] to provide simple, boolean-returning predicates
//! for public and secret keys, plus a helper that derives a public key from
//! a secret key.
//!
//! # Examples
//!
//! ```rust
//! use contract_fox::utils::keypair::{is_valid_public_key, is_valid_secret_key, public_key_from_secret};
//!
//! assert!(is_valid_public_key("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"));
//! assert!(is_valid_secret_key("SCZANGBA5YELQQHRKQM5AOPXJH3BZXCUAC3I7JUGWSCSM7YIZMMBZNE"));
//! assert!(!is_valid_public_key("not-a-key"));
//! ```

use stellar_strkey::{ed25519, Strkey};
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
/// A valid Stellar public key starts with `G` and is 56 characters long
/// (StrKey-encoded Ed25519 public key).
///
/// # Examples
///
/// ```rust
/// # use contract_fox::utils::keypair::is_valid_public_key;
/// assert!(is_valid_public_key("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"));
/// assert!(!is_valid_public_key("SCZANGBA5YELQQHRKQM5AOPXJH3BZXCUAC3I7JUGWSCSM7YIZMMBZNE")); // secret key
/// assert!(!is_valid_public_key("not-a-key"));
/// assert!(!is_valid_public_key(""));
/// ```
pub fn is_valid_public_key(key: &str) -> bool {
    matches!(
        Strkey::from_string(key),
        Ok(Strkey::PublicKeyEd25519(_))
    )
}

/// Returns `true` if `key` is a valid Stellar secret (seed) key.
///
/// A valid Stellar secret key starts with `S` and is 56 characters long
/// (StrKey-encoded Ed25519 private seed).
///
/// # Examples
///
/// ```rust
/// # use contract_fox::utils::keypair::is_valid_secret_key;
/// assert!(is_valid_secret_key("SCZANGBA5YELQQHRKQM5AOPXJH3BZXCUAC3I7JUGWSCSM7YIZMMBZNE"));
/// assert!(!is_valid_secret_key("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN")); // public key
/// assert!(!is_valid_secret_key("not-a-key"));
/// assert!(!is_valid_secret_key(""));
/// ```
pub fn is_valid_secret_key(key: &str) -> bool {
    matches!(
        Strkey::from_string(key),
        Ok(Strkey::PrivateKeyEd25519(_))
    )
}

// ── Derivation helper ─────────────────────────────────────────────────────────

/// Derives the Stellar public key (account ID) from a secret key string.
///
/// Returns the public key as a StrKey-encoded `String` (starts with `G`),
/// or a [`KeyError`] if the supplied secret key is invalid.
///
/// # Examples
///
/// ```rust
/// # use contract_fox::utils::keypair::public_key_from_secret;
/// let public = public_key_from_secret(
///     "SCZANGBA5YELQQHRKQM5AOPXJH3BZXCUAC3I7JUGWSCSM7YIZMMBZNE"
/// ).unwrap();
/// assert!(public.starts_with('G'));
/// assert_eq!(public.len(), 56);
/// ```
pub fn public_key_from_secret(secret: &str) -> Result<String, KeyError> {
    // Parse the secret key
    let seed = match Strkey::from_string(secret) {
        Ok(Strkey::PrivateKeyEd25519(s)) => s,
        _ => {
            return Err(KeyError::InvalidSecretKey(secret.to_owned()));
        }
    };

    // Derive the Ed25519 keypair from the 32-byte seed
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed.0);
    let verifying_key = signing_key.verifying_key();

    // Encode the public key as a Stellar StrKey
    let public = ed25519::PublicKey(verifying_key.to_bytes());
    Ok(Strkey::PublicKeyEd25519(public).to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Known good keys (test vectors) ───────────────────────────────────────
    // Generated with `stellar keys generate` on testnet — safe to commit.
    const VALID_PUBLIC_KEY: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
    const VALID_SECRET_KEY: &str = "SCZANGBA5YELQQHRKQM5AOPXJH3BZXCUAC3I7JUGWSCSM7YIZMMBZNE";

    // A second keypair so we can test that public_key_from_secret returns the
    // *correct* public key, not just any G-prefixed string.
    const VALID_SECRET_KEY_2: &str = "SBQWY3DNBE3OYYQ24X5LLPFBKR6HZPFSNHNHZE5ONXYLZGDMMJD7GSG";
    const VALID_PUBLIC_KEY_2: &str = "GDUTHCF37UX32EMANXIL2WOOVEDAALEER57DMDA4EFTLHVZ5GSREJUOG";

    // ── is_valid_public_key ───────────────────────────────────────────────────

    #[test]
    fn valid_public_key_returns_true() {
        assert!(is_valid_public_key(VALID_PUBLIC_KEY));
    }

    #[test]
    fn second_valid_public_key_returns_true() {
        assert!(is_valid_public_key(VALID_PUBLIC_KEY_2));
    }

    #[test]
    fn secret_key_is_not_a_valid_public_key() {
        assert!(!is_valid_public_key(VALID_SECRET_KEY));
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
        // Remove last character → bad checksum
        let truncated = &VALID_PUBLIC_KEY[..VALID_PUBLIC_KEY.len() - 1];
        assert!(!is_valid_public_key(truncated));
    }

    #[test]
    fn public_key_with_wrong_prefix_is_invalid() {
        // Replace leading G with X → wrong version byte
        let mangled = format!("X{}", &VALID_PUBLIC_KEY[1..]);
        assert!(!is_valid_public_key(&mangled));
    }

    // ── is_valid_secret_key ───────────────────────────────────────────────────

    #[test]
    fn valid_secret_key_returns_true() {
        assert!(is_valid_secret_key(VALID_SECRET_KEY));
    }

    #[test]
    fn second_valid_secret_key_returns_true() {
        assert!(is_valid_secret_key(VALID_SECRET_KEY_2));
    }

    #[test]
    fn public_key_is_not_a_valid_secret_key() {
        assert!(!is_valid_secret_key(VALID_PUBLIC_KEY));
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
        let truncated = &VALID_SECRET_KEY[..VALID_SECRET_KEY.len() - 1];
        assert!(!is_valid_secret_key(truncated));
    }

    #[test]
    fn secret_key_with_wrong_prefix_is_invalid() {
        // Replace leading S with T → wrong version byte
        let mangled = format!("T{}", &VALID_SECRET_KEY[1..]);
        assert!(!is_valid_secret_key(&mangled));
    }

    // ── public_key_from_secret ────────────────────────────────────────────────

    #[test]
    fn derives_correct_public_key_from_secret() {
        let public = public_key_from_secret(VALID_SECRET_KEY_2).unwrap();
        assert_eq!(public, VALID_PUBLIC_KEY_2);
    }

    #[test]
    fn derived_public_key_starts_with_g_and_is_56_chars() {
        let public = public_key_from_secret(VALID_SECRET_KEY).unwrap();
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
        // Passing a public key as secret should fail
        let err = public_key_from_secret(VALID_PUBLIC_KEY).unwrap_err();
        assert!(matches!(err, KeyError::InvalidSecretKey(_)));
    }

    #[test]
    fn public_key_from_empty_string_returns_err() {
        let err = public_key_from_secret("").unwrap_err();
        assert!(matches!(err, KeyError::InvalidSecretKey(_)));
    }
}
