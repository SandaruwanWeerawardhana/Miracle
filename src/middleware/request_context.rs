use std::any::Any;

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::REQUEST_ID_HEADER;
use crate::shared::{error::AppError, request_context::with_request_id};

/// Makes the request ID (set by `SetRequestIdLayer`) available through
/// `shared::request_context::current_request_id`.
pub async fn scope(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();

    with_request_id(request_id, next.run(request)).await
}

/// Converts a handler panic into the standard error body. Details are logged, never returned.
pub fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
    let detail = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic payload");
    tracing::error!(panic = detail, "request handler panicked");

    AppError::Internal(anyhow::anyhow!("handler panicked")).into_response()
}
