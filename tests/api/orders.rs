use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{count, create_customer, create_user, router, send, test_state};

async fn create_order(pool: &PgPool, customer_id: Uuid, created_by: Uuid, status: &str) -> Uuid {
    let order_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO orders (id, order_number, customer_id, currency, status, subtotal, charges_total,
                            grand_total, created_by)
        VALUES ($1, 'ORD-TEST-' || nextval('order_number_seq'), $2, 'USD', $3, 100, 0, 100, $4)
        "#,
    )
    .bind(order_id)
    .bind(customer_id)
    .bind(status)
    .bind(created_by)
    .execute(pool)
    .await
    .unwrap();
    order_id
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn status_moves_only_along_allowed_transitions(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let manager = create_user(&state, "STAFF", &["MANAGER"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let order_id = create_order(&pool, customer_id, manager.user_id, "PENDING").await;
    let uri = format!("/api/v1/orders/{order_id}/status");

    let skip = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&manager.token),
        Some(json!({ "status": "SHIPPED" })),
    )
    .await;
    assert_eq!(skip.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        skip.body["error"]["code"],
        "ORDER_INVALID_STATUS_TRANSITION"
    );

    let confirm = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&manager.token),
        Some(json!({ "status": "CONFIRMED" })),
    )
    .await;
    assert_eq!(confirm.status, StatusCode::OK, "{}", confirm.body);
    assert_eq!(confirm.body["data"]["previous_status"], "PENDING");
    assert_eq!(confirm.body["data"]["status"], "CONFIRMED");
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
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = $1",
            order_id
        )
        .await,
        1
    );
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn cancellation_requires_permission_and_reason(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let logistics = create_user(&state, "STAFF", &["LOGISTICS_STAFF"]).await;
    let manager = create_user(&state, "STAFF", &["MANAGER"]).await;
    let customer_user = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let customer_id = create_customer(&pool, customer_user.user_id).await;
    let order_id = create_order(&pool, customer_id, manager.user_id, "CONFIRMED").await;
    let uri = format!("/api/v1/orders/{order_id}/status");
    let cancel = json!({ "status": "CANCELLED", "note": "Customer request" });

    let no_permission = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&logistics.token),
        Some(cancel.clone()),
    )
    .await;
    assert_eq!(no_permission.status, StatusCode::FORBIDDEN);

    let customer = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&customer_user.token),
        Some(cancel.clone()),
    )
    .await;
    assert_eq!(customer.status, StatusCode::FORBIDDEN);

    let no_reason = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&manager.token),
        Some(json!({ "status": "CANCELLED" })),
    )
    .await;
    assert_eq!(no_reason.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        no_reason.body["error"]["code"],
        "ORDER_CANCELLATION_REASON_REQUIRED"
    );

    let cancelled = send(
        &app,
        Method::PATCH,
        &uri,
        Some(&manager.token),
        Some(cancel),
    )
    .await;
    assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
}
