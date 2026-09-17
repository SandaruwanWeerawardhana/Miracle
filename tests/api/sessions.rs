use axum::http::{Method, StatusCode};
use sqlx::PgPool;

use crate::common::{create_user, router, send, test_state};

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn current_user_is_returned_without_credentials(pool: PgPool) {
    let state = test_state(pool);
    let app = router(&state);
    let user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;

    let response = send(
        &app,
        Method::GET,
        "/api/v1/users/me",
        Some(&user.token),
        None,
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["data"]["id"], user.user_id.to_string());
    assert_eq!(response.body["data"]["account_type"], "CUSTOMER");
    assert!(response.body["data"].get("password_hash").is_none());
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn logout_revokes_session_and_its_access_tokens(pool: PgPool) {
    let state = test_state(pool);
    let app = router(&state);
    let user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;

    let logout = send(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(&user.token),
        None,
    )
    .await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);

    let after = send(
        &app,
        Method::GET,
        "/api/v1/users/me",
        Some(&user.token),
        None,
    )
    .await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
    assert_eq!(after.body["error"]["code"], "SESSION_INVALID");
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn disabled_account_cannot_use_existing_token(pool: PgPool) {
    let state = test_state(pool);
    let app = router(&state);
    let user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;

    sqlx::query("UPDATE users SET status = 'DISABLED' WHERE id = $1")
        .bind(user.user_id)
        .execute(&state.db)
        .await
        .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/v1/users/me",
        Some(&user.token),
        None,
    )
    .await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}
