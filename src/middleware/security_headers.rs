use axum::{
    Router,
    http::{HeaderName, HeaderValue, header},
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::app::AppState;

/// Baseline security headers for every response. Content-Security-Policy and
/// `Cache-Control: no-store` are applied on the API router only, because the
/// Swagger UI needs to load scripts.
pub fn apply(router: Router<AppState>, enable_hsts: bool) -> Router<AppState> {
    let router = router
        .layer(set(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(set(header::X_FRAME_OPTIONS, "DENY"))
        .layer(set(header::REFERRER_POLICY, "no-referrer"));

    if enable_hsts {
        router.layer(set(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains",
        ))
    } else {
        router
    }
}

pub fn set(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}
