use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::shared::error::AppError;

crate::typed_id!(
    /// Identifier of a row in `payments`.
    PaymentId
);

/// Why the payment is being made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentType {
    Full,
    Partial,
    Advance,
    Milestone,
}

/// Status of one money movement (a row in `payments`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Pending,
    Paid,
    Failed,
    Refunded,
}

impl PaymentStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        use PaymentStatus::*;
        matches!((self, next), (Pending, Paid | Failed) | (Paid, Refunded))
    }
}

/// Settlement status of an installment (a row in `payment_schedules`), derived
/// from the confirmed payments allocated to it. `PARTIALLY_PAID` lives here, not
/// on individual payments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleStatus {
    Pending,
    PartiallyPaid,
    Paid,
    Refunded,
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("payment was not found")]
    NotFound,

    #[error("payment cannot change from {from:?} to {to:?}")]
    InvalidTransition {
        from: PaymentStatus,
        to: PaymentStatus,
    },

    #[error("payment amount exceeds the outstanding balance")]
    Overpayment,

    #[error("unknown payment provider `{0}`")]
    UnknownProvider(String),

    #[error("webhook signature verification failed")]
    InvalidSignature,
}

impl From<PaymentError> for AppError {
    fn from(error: PaymentError) -> Self {
        let message = error.to_string();
        match error {
            PaymentError::NotFound => AppError::not_found("PAYMENT_NOT_FOUND", message),
            PaymentError::InvalidTransition { .. } => {
                AppError::business_rule("PAYMENT_INVALID_STATUS_TRANSITION", message)
            }
            PaymentError::Overpayment => AppError::business_rule("PAYMENT_OVERPAYMENT", message),
            PaymentError::UnknownProvider(_) => {
                AppError::not_found("PAYMENT_PROVIDER_NOT_FOUND", message)
            }
            PaymentError::InvalidSignature => {
                AppError::bad_request("PAYMENT_WEBHOOK_INVALID_SIGNATURE", message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PaymentStatus::*;

    #[test]
    fn refunds_require_a_paid_payment() {
        assert!(Paid.can_transition_to(Refunded));
        assert!(!Pending.can_transition_to(Refunded));
        assert!(!Failed.can_transition_to(Paid));
    }
}
