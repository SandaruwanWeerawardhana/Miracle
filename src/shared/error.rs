//! Central API error type and its single mapping to HTTP responses.
//!
//! Modules define their own domain error enums (e.g. `QuotationError`) and
//! implement `From<ModuleError> for AppError`. Handlers then just use `?`.

use std::borrow::Cow;

use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use validator::ValidationErrors;

use super::request_context::current_request_id;

pub type AppResult<T> = Result<T, AppError>;

/// Stable, machine-readable error code. Clients branch on this, so never rename one.
pub type ErrorCode = &'static str;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("request validation failed")]
    Validation(#[from] ValidationErrors),

    #[error("{message}")]
    BadRequest {
        code: ErrorCode,
        message: Cow<'static, str>,
    },

    #[error("authentication required")]
    Unauthenticated { code: ErrorCode },

    #[error("permission denied")]
    Forbidden,

    #[error("{message}")]
    NotFound {
        code: ErrorCode,
        message: Cow<'static, str>,
    },

    #[error("{message}")]
    Conflict {
        code: ErrorCode,
        message: Cow<'static, str>,
    },

    /// A well-formed request that violates a business rule
    /// (e.g. accepting an expired quotation).
    #[error("{message}")]
    BusinessRule {
        code: ErrorCode,
        message: Cow<'static, str>,
    },

    #[error("too many requests")]
    RateLimited,

    #[error("not implemented")]
    NotImplemented,

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn bad_request(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::NotFound {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }

    pub fn business_rule(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::BusinessRule {
            code,
            message: message.into(),
        }
    }

    pub fn unauthenticated() -> Self {
        Self::Unauthenticated {
            code: "UNAUTHENTICATED",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Unauthenticated { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::BusinessRule { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> ErrorCode {
        match self {
            Self::Validation(_) => "VALIDATION_FAILED",
            Self::BadRequest { code, .. }
            | Self::Unauthenticated { code }
            | Self::NotFound { code, .. }
            | Self::Conflict { code, .. }
            | Self::BusinessRule { code, .. } => code,
            Self::Forbidden => "FORBIDDEN",
            Self::RateLimited => "RATE_LIMITED",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::Database(_) | Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    fn details(&self) -> Option<Value> {
        match self {
            Self::Validation(errors) => serde_json::to_value(errors.field_errors()).ok(),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let request_id = current_request_id();

        // Internal failures are logged with full detail and returned opaquely.
        let message = if status.is_server_error() {
            tracing::error!(error = ?self, request_id, "request failed");
            "An unexpected error occurred".to_owned()
        } else {
            tracing::debug!(error = %self, code = self.code(), "request rejected");
            self.to_string()
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code().to_owned(),
                message,
                details: self.details(),
                request_id,
            },
        };

        (status, Json(body)).into_response()
    }
}

// Extractor rejections map to the same envelope instead of axum's plain-text defaults.

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        match rejection {
            JsonRejection::MissingJsonContentType(_) => Self::bad_request(
                "UNSUPPORTED_CONTENT_TYPE",
                "Expected `Content-Type: application/json`",
            ),
            other if other.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                Self::bad_request("PAYLOAD_TOO_LARGE", "Request body is too large")
            }
            other => Self::bad_request("INVALID_JSON", other.body_text()),
        }
    }
}

impl From<QueryRejection> for AppError {
    fn from(rejection: QueryRejection) -> Self {
        Self::bad_request("INVALID_QUERY", rejection.body_text())
    }
}

impl From<PathRejection> for AppError {
    fn from(rejection: PathRejection) -> Self {
        Self::bad_request("INVALID_PATH", rejection.body_text())
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Stable machine-readable code, e.g. `QUOTATION_NOT_FOUND`.
    #[schema(example = "QUOTATION_NOT_FOUND")]
    pub code: String,
    /// Human-readable message. Safe to display; never contains internals.
    pub message: String,
    /// Structured details, e.g. per-field validation errors.
    pub details: Option<Value>,
    pub request_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_errors_are_not_leaked() {
        let error = AppError::Database(sqlx::Error::PoolTimedOut);
        assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.code(), "INTERNAL_ERROR");
    }

    #[test]
    fn business_rule_maps_to_unprocessable_entity() {
        let error = AppError::business_rule("QUOTATION_EXPIRED", "Quotation has expired");
        assert_eq!(error.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code(), "QUOTATION_EXPIRED");
    }
}
