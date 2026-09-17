use super::OrderStatus;
use crate::shared::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("order was not found")]
    NotFound,

    #[error("cannot change order status from {from} to {to}")]
    InvalidTransition { from: OrderStatus, to: OrderStatus },

    #[error("a cancellation reason is required")]
    CancellationReasonRequired,

    #[error("an order already exists for this quotation")]
    AlreadyExistsForQuotation,

    #[error("an order must contain at least one item")]
    NoItems,
}

impl From<OrderError> for AppError {
    fn from(error: OrderError) -> Self {
        let message = error.to_string();
        match error {
            OrderError::NotFound => AppError::not_found("ORDER_NOT_FOUND", message),
            OrderError::InvalidTransition { .. } => {
                AppError::business_rule("ORDER_INVALID_STATUS_TRANSITION", message)
            }
            OrderError::CancellationReasonRequired => {
                AppError::business_rule("ORDER_CANCELLATION_REASON_REQUIRED", message)
            }
            OrderError::AlreadyExistsForQuotation => {
                AppError::conflict("ORDER_ALREADY_EXISTS_FOR_QUOTATION", message)
            }
            OrderError::NoItems => AppError::business_rule("ORDER_HAS_NO_ITEMS", message),
        }
    }
}
