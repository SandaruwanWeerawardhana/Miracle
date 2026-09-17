use std::{fmt, str::FromStr, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{AccessClaims, Permission, PermissionSet, TokenService};
use crate::shared::{error::AppError, ids::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Customer,
    Supplier,
    Staff,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Customer => "CUSTOMER",
            Self::Supplier => "SUPPLIER",
            Self::Staff => "STAFF",
        }
    }
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AccountType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CUSTOMER" => Ok(Self::Customer),
            "SUPPLIER" => Ok(Self::Supplier),
            "STAFF" => Ok(Self::Staff),
            other => Err(format!("unknown account type `{other}`")),
        }
    }
}

/// The authenticated caller of the current request.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user_id: UserId,
    pub session_id: Uuid,
    pub account_type: AccountType,
    pub permissions: PermissionSet,
}

/// How much of a resource collection the caller may see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessScope {
    /// Staff-style access to every record.
    All,
    /// Only records owned by this user's customer/supplier profile.
    Own(UserId),
}

impl CurrentUser {
    pub fn has(&self, permission: Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Fails with 403 unless the caller holds `permission`.
    pub fn require(&self, permission: Permission) -> Result<(), AppError> {
        if self.has(permission) {
            Ok(())
        } else {
            tracing::info!(user_id = %self.user_id, permission = %permission, "permission denied");
            Err(AppError::Forbidden)
        }
    }

    /// Resolves an `all` / `own` permission pair (e.g. `quotation.read` /
    /// `quotation.read.own`) into a query scope. Repositories must apply the scope.
    pub fn scope(&self, all: Permission, own: Permission) -> Result<AccessScope, AppError> {
        if self.has(all) {
            Ok(AccessScope::All)
        } else if self.has(own) {
            Ok(AccessScope::Own(self.user_id))
        } else {
            tracing::info!(user_id = %self.user_id, permission = %all, "permission denied");
            Err(AppError::Forbidden)
        }
    }
}

/// Turns verified token claims into a principal, checking that the session is
/// still active and the account is enabled. `Ok(None)` means "not authenticated".
#[async_trait]
pub trait PrincipalResolver: Send + Sync + 'static {
    async fn resolve(&self, claims: &AccessClaims) -> Result<Option<CurrentUser>, AppError>;
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    Arc<TokenService>: FromRef<S>,
    Arc<dyn PrincipalResolver>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Resolve at most once per request, even if several extractors ask.
        if let Some(user) = parts.extensions.get::<CurrentUser>() {
            return Ok(user.clone());
        }

        let token = bearer_token(parts).ok_or_else(AppError::unauthenticated)?;

        let claims = Arc::<TokenService>::from_ref(state)
            .verify_access_token(token)
            .map_err(|_| AppError::Unauthenticated {
                code: "INVALID_ACCESS_TOKEN",
            })?;

        let user = Arc::<dyn PrincipalResolver>::from_ref(state)
            .resolve(&claims)
            .await?
            .ok_or(AppError::Unauthenticated {
                code: "SESSION_INVALID",
            })?;

        parts.extensions.insert(user.clone());
        Ok(user)
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    (scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty()).then(|| token.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(permissions: &[&str]) -> CurrentUser {
        CurrentUser {
            user_id: UserId::new(),
            session_id: Uuid::now_v7(),
            account_type: AccountType::Customer,
            permissions: PermissionSet::from_codes(permissions.iter().copied()),
        }
    }

    #[test]
    fn scope_prefers_all_over_own() {
        let staff = user(&["quotation.read", "quotation.read.own"]);
        assert_eq!(
            staff
                .scope(Permission::QuotationRead, Permission::QuotationReadOwn)
                .unwrap(),
            AccessScope::All
        );

        let customer = user(&["quotation.read.own"]);
        assert!(matches!(
            customer.scope(Permission::QuotationRead, Permission::QuotationReadOwn),
            Ok(AccessScope::Own(_))
        ));
    }

    #[test]
    fn require_denies_missing_permission() {
        assert!(matches!(
            user(&[]).require(Permission::PaymentConfirm),
            Err(AppError::Forbidden)
        ));
    }
}
