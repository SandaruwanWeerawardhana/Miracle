//! Token design
//!
//! * **Access token**: short-lived (default 15 min) HS256 JWT sent as
//!   `Authorization: Bearer`. Carries only identity (`sub`) and session (`sid`);
//!   permissions are resolved server-side so role changes apply immediately.
//! * **Refresh token**: opaque 256-bit random value, stored only as a SHA-256
//!   hash in `user_sessions`. Rotated on every use; reuse of a rotated token
//!   revokes the whole session (theft detection). Delivered to browsers as an
//!   `HttpOnly; Secure; SameSite=Strict` cookie scoped to the refresh endpoint,
//!   and in the body for mobile clients.
//! * **Email verification / password reset tokens**: opaque, single-use,
//!   hashed at rest, short expiry.

use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AuthConfig;

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token is invalid or expired")]
    Invalid,

    #[error("failed to issue token: {0}")]
    Issue(String),

    #[error("secure random generator unavailable")]
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    /// User ID.
    pub sub: Uuid,
    /// Session ID; lets logout and revocation invalidate outstanding access tokens.
    pub sid: Uuid,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl TokenService {
    pub fn new(config: &AuthConfig) -> Self {
        let secret = config.jwt_secret.expose().as_bytes();

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[config.jwt_issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "sub", "iss"]);
        validation.leeway = 30;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            issuer: config.jwt_issuer.clone(),
            access_ttl: config.access_token_ttl,
            refresh_ttl: config.refresh_token_ttl,
        }
    }

    pub fn access_ttl(&self) -> Duration {
        self.access_ttl
    }

    pub fn refresh_ttl(&self) -> Duration {
        self.refresh_ttl
    }

    pub fn issue_access_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<String, TokenError> {
        let now = Utc::now().timestamp();
        let ttl = i64::try_from(self.access_ttl.as_secs()).unwrap_or(i64::MAX);
        let claims = AccessClaims {
            sub: user_id,
            sid: session_id,
            iss: self.issuer.clone(),
            iat: now,
            exp: now.saturating_add(ttl),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|error| TokenError::Issue(error.to_string()))
    }

    pub fn verify_access_token(&self, token: &str) -> Result<AccessClaims, TokenError> {
        decode::<AccessClaims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|_| TokenError::Invalid)
    }
}

/// A random opaque token. `value` goes to the client once; only `hash` is stored.
pub struct OpaqueToken {
    pub value: String,
    pub hash: Vec<u8>,
}

impl OpaqueToken {
    pub fn generate() -> Result<Self, TokenError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| TokenError::Random)?;
        let value = URL_SAFE_NO_PAD.encode(bytes);
        let hash = Self::hash(&value);
        Ok(Self { value, hash })
    }

    /// Hash used for lookup. SHA-256 is sufficient because the input has 256 bits of entropy.
    pub fn hash(value: &str) -> Vec<u8> {
        Sha256::digest(value.as_bytes()).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Secret;

    fn service() -> TokenService {
        TokenService::new(&AuthConfig {
            jwt_secret: Secret::new("x".repeat(48)),
            jwt_issuer: "miracle-test".into(),
            access_token_ttl: Duration::from_secs(60),
            refresh_token_ttl: Duration::from_secs(3600),
        })
    }

    #[test]
    fn access_token_round_trip() {
        let service = service();
        let (user, session) = (Uuid::now_v7(), Uuid::now_v7());
        let token = service.issue_access_token(user, session).unwrap();
        let claims = service.verify_access_token(&token).unwrap();
        assert_eq!(claims.sub, user);
        assert_eq!(claims.sid, session);
    }

    #[test]
    fn rejects_tampered_token() {
        let service = service();
        let mut token = service
            .issue_access_token(Uuid::now_v7(), Uuid::now_v7())
            .unwrap();
        token.push('x');
        assert!(service.verify_access_token(&token).is_err());
    }

    #[test]
    fn opaque_tokens_are_unique_and_hash_matches() {
        let first = OpaqueToken::generate().unwrap();
        let second = OpaqueToken::generate().unwrap();
        assert_ne!(first.value, second.value);
        assert_eq!(OpaqueToken::hash(&first.value), first.hash);
    }
}
