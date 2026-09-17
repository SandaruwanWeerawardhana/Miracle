use axum::http::{Method, StatusCode};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    count, create_customer, create_sent_quotation, create_user, router, send, test_state,
};

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn accepting_a_quotation_creates_order_audit_and_job_atomically(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let quotation_id = create_sent_quotation(&pool, customer_id, sales.user_id, "7 days").await;

    let uri = format!("/api/v1/quotations/{quotation_id}/accept");
    let response = send(&app, Method::POST, &uri, Some(&customer_user.token), None).await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    let order_id: Uuid = response.body["data"]["order_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(quotation_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "ACCEPTED");

    let (order_quotation, grand_total): (Uuid, Decimal) =
        sqlx::query_as("SELECT quotation_id, grand_total FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(order_quotation, quotation_id);
    assert_eq!(grand_total, Decimal::new(130_000, 2));

    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM order_items WHERE order_id = $1",
            order_id
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM order_status_history WHERE order_id = $1",
            order_id
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = $1 AND action = 'quotation.accepted'",
            quotation_id
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM background_jobs WHERE payload->'data'->>'quotation_id' = $1::text",
            quotation_id
        )
        .await,
        1
    );
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn a_quotation_cannot_be_accepted_twice(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let quotation_id = create_sent_quotation(&pool, customer_id, sales.user_id, "7 days").await;
    let uri = format!("/api/v1/quotations/{quotation_id}/accept");

    let first = send(&app, Method::POST, &uri, Some(&customer_user.token), None).await;
    assert_eq!(first.status, StatusCode::CREATED);

    let second = send(&app, Method::POST, &uri, Some(&customer_user.token), None).await;
    assert_eq!(second.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        second.body["error"]["code"],
        "QUOTATION_INVALID_STATUS_TRANSITION"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM orders WHERE quotation_id = $1",
            quotation_id
        )
        .await,
        1
    );
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn expired_quotation_is_rejected_and_nothing_is_written(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let quotation_id = create_sent_quotation(&pool, customer_id, sales.user_id, "-1 hour").await;

    let uri = format!("/api/v1/quotations/{quotation_id}/accept");
    let response = send(&app, Method::POST, &uri, Some(&customer_user.token), None).await;

    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.body["error"]["code"], "QUOTATION_EXPIRED");
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM orders WHERE quotation_id = $1",
            quotation_id
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = $1",
            quotation_id
        )
        .await,
        0
    );
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn another_customers_quotation_is_reported_as_not_found(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;
    let owner = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let owner_customer = create_customer(&pool, owner.user_id).await;
    let intruder = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    create_customer(&pool, intruder.user_id).await;
    let quotation_id = create_sent_quotation(&pool, owner_customer, sales.user_id, "7 days").await;

    let uri = format!("/api/v1/quotations/{quotation_id}/accept");
    let response = send(&app, Method::POST, &uri, Some(&intruder.token), None).await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.body["error"]["code"], "QUOTATION_NOT_FOUND");
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn staff_cannot_accept_on_behalf_of_customer(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let quotation_id = create_sent_quotation(&pool, customer_id, sales.user_id, "7 days").await;

    let uri = format!("/api/v1/quotations/{quotation_id}/accept");
    let response = send(&app, Method::POST, &uri, Some(&sales.token), None).await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
}
