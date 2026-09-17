use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::shared::error::AppError;

crate::typed_id!(
    /// Identifier of a row in `suppliers`.
    SupplierId
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Unverified,
    PendingReview,
    Verified,
    Rejected,
    Suspended,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "UNVERIFIED",
            Self::PendingReview => "PENDING_REVIEW",
            Self::Verified => "VERIFIED",
            Self::Rejected => "REJECTED",
            Self::Suspended => "SUSPENDED",
        }
    }

    /// The verification lifecycle:
    ///
    /// ```text
    /// UNVERIFIED ─► PENDING_REVIEW ─► VERIFIED ◄─► SUSPENDED
    ///                    ▲    │
    ///                    │    ▼
    ///                    └─ REJECTED
    /// ```
    pub fn can_transition_to(self, next: Self) -> bool {
        use VerificationStatus::*;
        matches!(
            (self, next),
            (Unverified | Rejected, PendingReview)
                | (PendingReview, Verified | Rejected)
                | (Verified, Suspended)
                | (Suspended, Verified)
        )
    }

    /// Decisions a staff reviewer may record (as opposed to supplier-initiated submission).
    pub fn is_review_decision(self) -> bool {
        matches!(self, Self::Verified | Self::Rejected | Self::Suspended)
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VerificationStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "UNVERIFIED" => Ok(Self::Unverified),
            "PENDING_REVIEW" => Ok(Self::PendingReview),
            "VERIFIED" => Ok(Self::Verified),
            "REJECTED" => Ok(Self::Rejected),
            "SUSPENDED" => Ok(Self::Suspended),
            other => Err(format!("unknown verification status `{other}`")),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupplierError {
    #[error("supplier was not found")]
    NotFound,

    #[error("cannot change verification status from {from} to {to}")]
    InvalidVerificationTransition {
        from: VerificationStatus,
        to: VerificationStatus,
    },

    #[error("{0} is not a review decision")]
    NotAReviewDecision(VerificationStatus),
}

impl From<SupplierError> for AppError {
    fn from(error: SupplierError) -> Self {
        let message = error.to_string();
        match error {
            SupplierError::NotFound => AppError::not_found("SUPPLIER_NOT_FOUND", message),
            SupplierError::InvalidVerificationTransition { .. } => {
                AppError::business_rule("SUPPLIER_INVALID_VERIFICATION_TRANSITION", message)
            }
            SupplierError::NotAReviewDecision(_) => {
                AppError::bad_request("SUPPLIER_INVALID_REVIEW_DECISION", message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VerificationStatus::*;

    #[test]
    fn verification_requires_review_first() {
        assert!(!Unverified.can_transition_to(Verified));
        assert!(Unverified.can_transition_to(PendingReview));
        assert!(PendingReview.can_transition_to(Verified));
    }

    #[test]
    fn suspension_only_applies_to_verified_suppliers() {
        assert!(Verified.can_transition_to(Suspended));
        assert!(!PendingReview.can_transition_to(Suspended));
        assert!(Suspended.can_transition_to(Verified));
    }
}
