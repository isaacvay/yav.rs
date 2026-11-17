//! Argon2id password hashing exposed behind the `argon2` Cargo feature.
//!
//! Uses argon2 0.6 together with password-hash 0.6 (transitively re-exported by
//! argon2). The stored representation is the standard PHC string
//! `$argon2id$v=19$m=...$...$...`, suitable for a database column.
#![cfg(feature = "argon2")]

use argon2::password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::Argon2;

/// Stateless Argon2id hasher/verifier for password strings.
///
/// Default Argon2id v19 parameters and a CSPRNG-generated salt are used for
/// every hash; the parameters are embedded in the returned PHC string, so
/// verification does not depend on the instance configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2Hasher;

impl Argon2Hasher {
    /// Hash a password and return its PHC string representation.
    ///
    /// The digest produced by `Argon2::hash_password` already renders as a
    /// standard PHC string with algorithm, version, parameters and salt, so it
    /// is returned directly without an extra parse/render round trip.
    pub fn hash_password(password: &str) -> Result<String, String> {
        let hash = Argon2::default()
            .hash_password(password.as_bytes())
            .map_err(|err| err.to_string())?;
        Ok(hash.to_string())
    }

    /// Verify a password against a previously generated PHC string.
    ///
    /// Returns `Ok(false)` for a wrong password. Returns `Err` only when the
    /// stored string is malformed or uses an unsupported algorithm/parameters.
    pub fn verify_password(password: &str, encoded: &str) -> Result<bool, String> {
        let parsed = PasswordHash::new(encoded).map_err(|err| err.to_string())?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_a_phc_string_and_verifies() {
        let hash = Argon2Hasher::hash_password("correct horse battery staple")
            .expect("hashing must succeed");
        assert!(
            hash.starts_with("$argon2id$"),
            "unexpected PHC string: {hash}"
        );
        assert!(
            Argon2Hasher::verify_password("correct horse battery staple", &hash).unwrap_or(false)
        );
    }

    #[test]
    fn wrong_password_does_not_verify() {
        let hash = Argon2Hasher::hash_password("s3cret").expect("hashing must succeed");
        assert!(!Argon2Hasher::verify_password("wrong", &hash).unwrap_or(true));
    }

    #[test]
    fn each_hash_uses_a_fresh_salt() {
        let first = Argon2Hasher::hash_password("same").expect("first hash");
        let second = Argon2Hasher::hash_password("same").expect("second hash");
        assert_ne!(first, second, "salt must be randomly generated per hash");
    }

    #[test]
    fn malformed_encoded_value_is_an_error() {
        assert!(Argon2Hasher::verify_password("x", "not-a-phc-string").is_err());
    }
}
