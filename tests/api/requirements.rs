use axum::http::{Method, StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_customer, create_user, router, send, test_state};

async fn create_requirement(
    pool: &PgPool,
    customer_id: Uuid,
    created_by: Uuid,
    title: &str,
    status: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO requirements (id, requirement_number, customer_id, kind, title, description,
                                  destination_country, status, created_by)
        VALUES ($1, 'REQ-TEST-' || nextval('requirement_number_seq'), $2, 'GENERAL', $3,
                'A sufficiently long description', 'LK', $4, $5)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(customer_id)
    .bind(title)
    .bind(status)
    .bind(created_by)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn customers_see_only_their_own_requirements_and_staff_see_all(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let alice = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let alice_customer = create_customer(&pool, alice.user_id).await;
    let bob = create_user(&state, "CUSTOMER", &["CUSTOMER"]).await;
    let bob_customer = create_customer(&pool, bob.user_id).await;
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;

    create_requirement(
        &pool,
        alice_customer,
        alice.user_id,
        "Alice tea",
        "SUBMITTED",
    )
    .await;
    create_requirement(
        &pool,
        alice_customer,
        alice.user_id,
        "Alice spices",
        "QUOTED",
    )
    .await;
    create_requirement(
        &pool,
        bob_customer,
        bob.user_id,
        "Bob machinery",
        "SUBMITTED",
    )
    .await;

    let own = send(
        &app,
        Method::GET,
        "/api/v1/requirements",
        Some(&alice.token),
        None,
    )
    .await;
    assert_eq!(own.status, StatusCode::OK, "{}", own.body);
    assert_eq!(own.body["pagination"]["total_items"], 2);

    let all = send(
        &app,
        Method::GET,
        "/api/v1/requirements?limit=2",
        Some(&sales.token),
        None,
    )
    .await;
    assert_eq!(all.body["pagination"]["total_items"], 3);
    assert_eq!(all.body["pagination"]["total_pages"], 2);
    assert_eq!(all.body["data"].as_array().unwrap().len(), 2);

    let filtered = send(
        &app,
        Method::GET,
        "/api/v1/requirements?status=SUBMITTED&search=tea",
        Some(&sales.token),
        None,
    )
    .await;
    assert_eq!(filtered.body["pagination"]["total_items"], 1);
    assert_eq!(filtered.body["data"][0]["title"], "Alice tea");
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn users_without_read_permission_are_forbidden(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let supplier = create_user(&state, "SUPPLIER", &["SUPPLIER"]).await;

    let response = send(
        &app,
        Method::GET,
        "/api/v1/requirements",
        Some(&supplier.token),
        None,
    )
    .await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn invalid_filter_is_rejected(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let sales = create_user(&state, "STAFF", &["SALES_STAFF"]).await;

    let bad_status = send(
        &app,
        Method::GET,
        "/api/v1/requirements?status=NOPE",
        Some(&sales.token),
        None,
    )
    .await;
    assert_eq!(bad_status.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_status.body["error"]["code"], "INVALID_QUERY");

    let bad_country = send(
        &app,
        Method::GET,
        "/api/v1/requirements?destination_country=LKA",
        Some(&sales.token),
        None,
    )
    .await;
    assert_eq!(bad_country.status, StatusCode::UNPROCESSABLE_ENTITY);
}
