use crate::{
    app::AppState,
    shared::error::{AppError, AppResult},
};

#[expect(dead_code, reason = "used once the TODO use case is implemented")]
pub struct LoginUser {
    pub email: String,
    pub password: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
}

pub struct IssuedTokens {
    pub access_token: String,
    pub access_token_expires_in: u64,
    pub refresh_token: String,
    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub refresh_token_expires_in: u64,
}

/// TODO: implement:
/// 1. Apply the Redis brute-force limiter keyed by IP and by email.
/// 2. `users::find_credentials_by_email`. If absent, still run
///    `verify_password` against a dummy hash (constant-ish timing), then return
///    `AuthError::InvalidCredentials`.
/// 3. Reject when `locked_until > now()` (`AccountLocked`) or status is
///    `DISABLED` / `PENDING_VERIFICATION`.
/// 4. On wrong password increment `failed_login_attempts`; lock for
///    `LOCKOUT_MINUTES` after `MAX_FAILED_LOGIN_ATTEMPTS`.
/// 5. On success, in one transaction: reset counters, set `last_login_at`,
///    insert `user_sessions` with a hashed `OpaqueToken` refresh token, audit
///    `auth.login_succeeded`.
/// 6. Issue the access JWT with `sub = user_id`, `sid = session_id`.
pub async fn execute(_state: &AppState, _command: LoginUser) -> AppResult<IssuedTokens> {
    Err(AppError::NotImplemented)
}
