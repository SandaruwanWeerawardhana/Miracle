use super::QuotationStatus;
use crate::shared::{error::AppError, money::MoneyError};

#[derive(Debug, thiserror::Error)]
pub enum QuotationError {
    #[error("quotation was not found")]
    NotFound,

    #[error("cannot change quotation status from {from} to {to}")]
    InvalidTransition {
        from: QuotationStatus,
        to: QuotationStatus,
    },

    #[error("quotation has expired")]
    Expired,

    #[error("quotation must contain at least one item")]
    NoItems,

    #[error("validity date must be in the future")]
    InvalidValidity,

    #[error("quotation total cannot be negative")]
    NegativeTotal,

    #[error("quotation was modified by another request; reload and retry")]
    ConcurrentModification,

    #[error(transparent)]
    Money(#[from] MoneyError),
}

impl From<QuotationError> for AppError {
    fn from(error: QuotationError) -> Self {
        let message = error.to_string();
        match error {
            QuotationError::NotFound => AppError::not_found("QUOTATION_NOT_FOUND", message),
            QuotationError::InvalidTransition { .. } => {
                AppError::business_rule("QUOTATION_INVALID_STATUS_TRANSITION", message)
            }
            QuotationError::Expired => AppError::business_rule("QUOTATION_EXPIRED", message),
            QuotationError::NoItems => AppError::business_rule("QUOTATION_HAS_NO_ITEMS", message),
            QuotationError::InvalidValidity => {
                AppError::business_rule("QUOTATION_INVALID_VALIDITY", message)
            }
            QuotationError::NegativeTotal => {
                AppError::business_rule("QUOTATION_NEGATIVE_TOTAL", message)
            }
            QuotationError::ConcurrentModification => {
                AppError::conflict("QUOTATION_CONCURRENT_MODIFICATION", message)
            }
            QuotationError::Money(inner) => {
                AppError::business_rule("MONEY_INVALID", inner.to_string())
            }
        }
    }
}
