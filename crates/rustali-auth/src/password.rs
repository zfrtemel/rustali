//! Password hashing with argon2id.
//!
//! One-way only: there is no API that returns a plaintext password, and verification
//! is constant-time through argon2's PHC comparison. Hashes are self-describing PHC
//! strings (`$argon2id$v=19$m=...$...`), so parameters can change over time without a
//! migration.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rustali_core::{Error, Result};

/// Hash `password` with argon2id and a fresh random salt, returning a PHC string.
pub fn hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| Error::internal(PasswordError(err.to_string())))
}

/// Verify `password` against a stored PHC `hash`. `Ok(false)` for a wrong password;
/// `Err` only when the stored hash cannot be parsed.
pub fn verify(password: &str, hash: &str) -> Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|err| Error::internal(PasswordError(err.to_string())))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Error raised when hashing fails or a stored hash is malformed.
#[derive(Debug, Clone, thiserror::Error)]
#[error("password hashing error: {0}")]
pub struct PasswordError(String);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_round_trip() {
        let hash = hash("correct horse battery staple").expect("hash");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify("correct horse battery staple", &hash).expect("verify"));
        assert!(!verify("wrong password", &hash).expect("verify"));
    }

    #[test]
    fn each_hash_has_a_unique_salt() {
        assert_ne!(hash("same").unwrap(), hash("same").unwrap());
    }

    #[test]
    fn malformed_hash_is_an_error() {
        assert!(verify("x", "not-a-phc-string").is_err());
    }
}
