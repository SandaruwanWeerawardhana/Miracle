use crate::shared::error::AppError;

/// Maximum consecutive failed logins before the account is temporarily locked.
#[expect(dead_code, reason = "used once the TODO use case is implemented")]
pub const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
/// Lock duration after exceeding [`MAX_FAILED_LOGIN_ATTEMPTS`].
#[expect(dead_code, reason = "used once the TODO use case is implemented")]
pub const LOCKOUT_MINUTES: i64 = 15;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Deliberately identical for "unknown email" and "wrong password" to
    /// prevent account enumeration.
    #[error("invalid email or password")]
    InvalidCredentials,

    #[error("email address is already registered")]
    EmailAlreadyRegistered,

    #[error("email address has not been verified")]
    EmailNotVerified,

    #[error("account is disabled")]
    AccountDisabled,

    #[error("account is temporarily locked")]
    AccountLocked,

    #[error("token is invalid or has expired")]
    InvalidToken,

    #[error("refresh token reuse detected; session revoked")]
    RefreshTokenReused,
}

impl From<AuthError> for AppError {
    fn from(error: AuthError) -> Self {
        let message = error.to_string();
        match error {
            AuthError::InvalidCredentials => AppError::Unauthenticated {
                code: "INVALID_CREDENTIALS",
            },
            AuthError::InvalidToken => AppError::Unauthenticated {
                code: "INVALID_TOKEN",
            },
            AuthError::RefreshTokenReused => AppError::Unauthenticated {
                code: "REFRESH_TOKEN_REUSED",
            },
            AuthError::EmailAlreadyRegistered => {
                AppError::conflict("EMAIL_ALREADY_REGISTERED", message)
            }
            AuthError::EmailNotVerified => AppError::business_rule("EMAIL_NOT_VERIFIED", message),
            AuthError::AccountDisabled => AppError::business_rule("ACCOUNT_DISABLED", message),
            AuthError::AccountLocked => AppError::business_rule("ACCOUNT_LOCKED", message),
        }
    }
}
