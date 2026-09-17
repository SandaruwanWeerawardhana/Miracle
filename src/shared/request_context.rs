//! Per-request context available anywhere inside the request's future.

use std::future::Future;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// The ID of the request currently being handled, if called inside a request.
///
/// Lets error bodies and audit records include the request ID without threading
/// it through every function signature.
pub fn current_request_id() -> Option<String> {
    REQUEST_ID
        .try_with(Clone::clone)
        .ok()
        .filter(|id| !id.is_empty())
}

/// Runs `future` with `request_id` as the current request ID.
pub async fn with_request_id<F: Future>(request_id: String, future: F) -> F::Output {
    REQUEST_ID.scope(request_id, future).await
}
