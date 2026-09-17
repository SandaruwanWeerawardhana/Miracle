use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::shared::{auth::AccountType, ids::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    PendingVerification,
    Active,
    Disabled,
}

impl UserStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingVerification => "PENDING_VERIFICATION",
            Self::Active => "ACTIVE",
            Self::Disabled => "DISABLED",
        }
    }

    /// Whether the account may hold an authenticated session.
    pub fn can_authenticate(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl FromStr for UserStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PENDING_VERIFICATION" => Ok(Self::PendingVerification),
            "ACTIVE" => Ok(Self::Active),
            "DISABLED" => Ok(Self::Disabled),
            other => Err(format!("unknown user status `{other}`")),
        }
    }
}

/// Domain view of an account. Deliberately has no password hash.
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub account_type: AccountType,
    pub status: UserStatus,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub preferred_language: String,
    pub created_at: DateTime<Utc>,
}

/// Only for the auth module's login flow.
#[derive(Clone)]
pub struct UserCredentials {
    pub user_id: UserId,
    pub password_hash: String,
    pub status: UserStatus,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl std::fmt::Debug for UserCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserCredentials")
            .field("user_id", &self.user_id)
            .field("password_hash", &"[REDACTED]")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
