//! Cross-cutting HTTP middleware (Tower layers) applied to the whole router.
//!
//! Layer order matters. `ServiceBuilder` applies layers top-to-bottom for the
//! request (the first layer listed is the outermost).
//!
//! TODO: add Redis-backed rate limiting (`rate_limit.rs`) — a strict budget on
//! `/api/v1/auth/*` (brute-force protection) and a general per-client budget.

mod request_context;
pub mod security_headers;

use std::time::Duration;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, Method, Request, StatusCode, header},
};
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{app::AppState, config::AppConfig};

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
pub const IDEMPOTENCY_KEY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");

pub fn apply(router: Router<AppState>, config: &AppConfig) -> Router<AppState> {
    // Innermost: closest to the handlers.
    let inner = ServiceBuilder::new()
        .layer(cors_layer(config))
        .layer(axum::middleware::from_fn(request_context::scope))
        .layer(CatchPanicLayer::custom(request_context::panic_response))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            config.server.request_timeout,
        ))
        .layer(DefaultBodyLimit::max(
            config.server.request_body_limit_bytes,
        ));

    // Outermost: request identity and tracing wrap everything, including errors
    // produced by the inner layers.
    let outer = ServiceBuilder::new()
        .layer(SetSensitiveRequestHeadersLayer::new([
            header::AUTHORIZATION,
            header::COOKIE,
        ]))
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuidV7))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_request_span)
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                )
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER));

    let router = router.layer(inner);
    let router = security_headers::apply(router, config.environment.is_deployed());
    router.layer(outer)
}

/// Request IDs use UUID v7 so they sort by time in log search.
#[derive(Clone, Copy)]
struct MakeRequestUuidV7;

impl MakeRequestId for MakeRequestUuidV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        uuid::Uuid::now_v7()
            .to_string()
            .parse()
            .ok()
            .map(RequestId::new)
    }
}

/// Only the path is recorded: query strings may carry tokens or personal data.
fn make_request_span<B>(request: &Request<B>) -> tracing::Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-");

    tracing::info_span!(
        "http_request",
        request_id,
        method = %request.method(),
        path = %request.uri().path(),
    )
}

fn cors_layer(config: &AppConfig) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            REQUEST_ID_HEADER,
            IDEMPOTENCY_KEY_HEADER,
        ])
        .expose_headers([REQUEST_ID_HEADER])
        .max_age(Duration::from_secs(3600));

    if config.cors.allow_any_origin {
        // Development only (rejected by config validation elsewhere). Browsers forbid
        // credentials with a wildcard origin, so credentials stay disabled here.
        base.allow_origin(AllowOrigin::any())
    } else {
        // Credentials are allowed so the HttpOnly refresh-token cookie can be sent.
        base.allow_origin(AllowOrigin::list(config.cors.allowed_origins.clone()))
            .allow_credentials(true)
    }
}
