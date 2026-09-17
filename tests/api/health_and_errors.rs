//! These tests need no database or Redis.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{router, send, test_state, unreachable_pool};

#[tokio::test]
async fn liveness_is_ok_and_carries_request_id_and_security_headers() {
    let app = router(&test_state(unreachable_pool()));

    let response = send(&app, Method::GET, "/health/live", None, None).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["status"], "ok");
    assert!(response.headers.contains_key("x-request-id"));
    assert_eq!(response.headers["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn unknown_api_route_uses_error_envelope() {
    let app = router(&test_state(unreachable_pool()));

    let response = send(&app, Method::GET, "/api/v1/does-not-exist", None, None).await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.body["error"]["code"], "ROUTE_NOT_FOUND");
    assert_eq!(
        response.body["error"]["request_id"],
        response.headers["x-request-id"].to_str().unwrap()
    );
}

#[tokio::test]
async fn protected_route_requires_bearer_token() {
    let app = router(&test_state(unreachable_pool()));

    let missing = send(&app, Method::GET, "/api/v1/users/me", None, None).await;
    assert_eq!(missing.status, StatusCode::UNAUTHORIZED);
    assert_eq!(missing.body["error"]["code"], "UNAUTHENTICATED");

    let garbage = send(
        &app,
        Method::GET,
        "/api/v1/users/me",
        Some("not-a-jwt"),
        None,
    )
    .await;
    assert_eq!(garbage.status, StatusCode::UNAUTHORIZED);
    assert_eq!(garbage.body["error"]["code"], "INVALID_ACCESS_TOKEN");
}

#[tokio::test]
async fn invalid_payload_returns_field_level_validation_errors() {
    let app = router(&test_state(unreachable_pool()));

    let response = send(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({
            "email": "not-an-email",
            "password": "short",
            "full_name": "A",
            "account_type": "CUSTOMER"
        })),
    )
    .await;

    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.body["error"]["code"], "VALIDATION_FAILED");
    assert!(response.body["error"]["details"]["email"].is_array());
    assert!(response.body["error"]["details"]["password"].is_array());
}

#[tokio::test]
async fn staff_account_type_cannot_self_register() {
    let app = router(&test_state(unreachable_pool()));

    let response = send(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({
            "email": "new@example.test",
            "password": "a-long-enough-password",
            "full_name": "New User",
            "account_type": "STAFF"
        })),
    )
    .await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.body["error"]["code"], "INVALID_JSON");
}
