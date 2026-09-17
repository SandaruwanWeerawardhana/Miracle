use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{count, create_user, router, send, test_state};

async fn create_supplier(pool: &PgPool, status: &str) -> Uuid {
    let supplier_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO suppliers (id, supplier_number, legal_name, country_code, verification_status)
        VALUES ($1, 'SUP-' || nextval('supplier_number_seq'), 'Guangzhou Textiles Co.', 'CN', $2)
        "#,
    )
    .bind(supplier_id)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    supplier_id
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn manager_verifies_supplier_pending_review(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let manager = create_user(&state, "STAFF", &["MANAGER"]).await;
    let supplier_id = create_supplier(&pool, "PENDING_REVIEW").await;
    let uri = format!("/api/v1/suppliers/{supplier_id}/verification");

    let response = send(
        &app,
        Method::POST,
        &uri,
        Some(&manager.token),
        Some(json!({ "decision": "VERIFIED", "notes": "Business licence checked" })),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
    assert_eq!(response.body["data"]["status"], "VERIFIED");
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM supplier_verification_reviews WHERE supplier_id = $1",
            supplier_id
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM suppliers WHERE id = $1 AND verified_at IS NOT NULL",
            supplier_id
        )
        .await,
        1
    );
}

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn verification_cannot_skip_review_or_be_done_without_permission(pool: PgPool) {
    let state = test_state(pool.clone());
    let app = router(&state);
    let manager = create_user(&state, "STAFF", &["MANAGER"]).await;
    let procurement = create_user(&state, "STAFF", &["PROCUREMENT_STAFF"]).await;
    let supplier_id = create_supplier(&pool, "UNVERIFIED").await;
    let uri = format!("/api/v1/suppliers/{supplier_id}/verification");
    let verify = json!({ "decision": "VERIFIED" });

    let forbidden = send(
        &app,
        Method::POST,
        &uri,
        Some(&procurement.token),
        Some(verify.clone()),
    )
    .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    let skipped = send(&app, Method::POST, &uri, Some(&manager.token), Some(verify)).await;
    assert_eq!(skipped.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        skipped.body["error"]["code"],
        "SUPPLIER_INVALID_VERIFICATION_TRANSITION"
    );
}
