//! Argon2id password hashing.
//!
//! Hashing is deliberately CPU and memory expensive, so both operations run on
//! Tokio's blocking pool to avoid stalling async worker threads.

use argon2::{
    Argon2,
    password_hash::{Error as HashError, PasswordHasher, PasswordVerifier},
};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password hashing failed: {0}")]
    Hashing(String),

    #[error("stored password hash is malformed")]
    MalformedHash,

    #[error("hashing task was cancelled")]
    TaskCancelled,
}

/// Returns a PHC-format string (`$argon2id$v=19$m=...`) that embeds salt and parameters.
pub async fn hash_password(password: String) -> Result<String, PasswordError> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|error| PasswordError::Hashing(error.to_string()))
    })
    .await
    .map_err(|_| PasswordError::TaskCancelled)?
}

/// Constant-time verification. `Ok(false)` means the password did not match.
pub async fn verify_password(password: String, stored_hash: String) -> Result<bool, PasswordError> {
    tokio::task::spawn_blocking(move || {
        match Argon2::default().verify_password(password.as_bytes(), stored_hash.as_str()) {
            Ok(()) => Ok(true),
            Err(HashError::PasswordInvalid) => Ok(false),
            Err(_) => Err(PasswordError::MalformedHash),
        }
    })
    .await
    .map_err(|_| PasswordError::TaskCancelled)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies() {
        let hash = hash_password("correct horse battery staple".into())
            .await
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(
            verify_password("correct horse battery staple".into(), hash.clone())
                .await
                .unwrap()
        );
        assert!(!verify_password("wrong".into(), hash).await.unwrap());
    }
}
