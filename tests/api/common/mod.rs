//! Test harness shared by all API tests.
//!
//! * Tests that need PostgreSQL use `#[sqlx::test]`, which creates a fresh,
//!   migrated database per test on the server in `DATABASE_URL` and drops it
//!   afterwards. Point `DATABASE_URL` at a dedicated TEST server, never production.
//!   Those tests are `#[ignore]`d so `cargo test` works without infrastructure;
//!   CI runs them with `cargo test -- --include-ignored`.
//! * Redis is connected lazily, so tests that never touch Redis do not need it.

#![allow(dead_code)]

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
};
use miracle_backend::{
    app::{self, AppState},
    config::{
        AppConfig, AuthConfig, CorsConfig, DatabaseConfig, Environment, JobsConfig, LogFormat,
        RedisConfig, Secret, ServerConfig, TelemetryConfig,
    },
};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

pub fn test_config() -> AppConfig {
    AppConfig {
        environment: Environment::Test,
        server: ServerConfig {
            host: [127, 0, 0, 1].into(),
            port: 0,
            request_body_limit_bytes: 1024 * 1024,
            request_timeout: Duration::from_secs(10),
            api_docs_enabled: false,
        },
        telemetry: TelemetryConfig {
            format: LogFormat::Pretty,
            level: "warn".into(),
        },
        database: DatabaseConfig {
            url: Secret::new("postgres://unused".into()),
            max_connections: 5,
            min_connections: 0,
            acquire_timeout: Duration::from_secs(5),
            run_migrations: false,
        },
        redis: RedisConfig {
            url: Secret::new("redis://127.0.0.1:6379".into()),
        },
        auth: AuthConfig {
            jwt_secret: Secret::new("test-secret-".repeat(4)),
            jwt_issuer: "miracle-test".into(),
            access_token_ttl: Duration::from_secs(900),
            refresh_token_ttl: Duration::from_secs(3600),
        },
        storage: None,
        cors: CorsConfig {
            allowed_origins: vec![HeaderValue::from_static("http://localhost:3000")],
            allow_any_origin: false,
        },
        jobs: JobsConfig {
            enabled: false,
            poll_interval: Duration::from_secs(1),
            batch_size: 10,
        },
    }
}

pub fn test_state(pool: PgPool) -> AppState {
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("valid redis url");
    let redis = ConnectionManager::new_lazy_with_config(client, ConnectionManagerConfig::new())
        .expect("lazy redis connection manager");
    AppState::new(test_config(), pool, redis)
}

/// A pool that never connects: for tests that must not reach the database.
pub fn unreachable_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .expect("lazy pool")
}

pub fn router(state: &AppState) -> Router {
    app::router(state.clone())
}

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> TestResponse {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(json) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("valid request");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("infallible router");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("readable body");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
    };

    TestResponse {
        status,
        headers,
        body,
    }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub token: String,
}

/// Inserts an ACTIVE user with the given roles and an active session.
pub async fn create_user(state: &AppState, account_type: &str, roles: &[&str]) -> TestUser {
    let user_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, full_name, account_type, status, email_verified_at)
        VALUES ($1, $2, 'not-a-real-hash', 'Test User', $3, 'ACTIVE', now())
        "#,
    )
    .bind(user_id)
    .bind(format!("{user_id}@example.test"))
    .bind(account_type)
    .execute(&state.db)
    .await
    .expect("insert user");

    for role in roles {
        sqlx::query("INSERT INTO user_roles (user_id, role_code) VALUES ($1, $2)")
            .bind(user_id)
            .bind(role)
            .execute(&state.db)
            .await
            .expect("grant role");
    }

    let session_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO user_sessions (id, user_id, refresh_token_hash, expires_at)
        VALUES ($1, $2, $3, now() + interval '1 day')
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(Uuid::now_v7().as_bytes().to_vec())
    .execute(&state.db)
    .await
    .expect("insert session");

    let token = state
        .tokens
        .issue_access_token(user_id, session_id)
        .expect("issue token");

    TestUser {
        user_id,
        session_id,
        token,
    }
}

pub async fn create_customer(pool: &PgPool, user_id: Uuid) -> Uuid {
    let customer_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO customers (id, user_id, customer_number, company_name, country_code)
        VALUES ($1, $2, 'CUS-' || nextval('customer_number_seq'), 'Test Imports (Pvt) Ltd', 'LK')
        "#,
    )
    .bind(customer_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert customer");
    customer_id
}

/// A SENT quotation: 100 × 12.50 USD + 50.00 shipping = 1300.00 USD.
pub async fn create_sent_quotation(
    pool: &PgPool,
    customer_id: Uuid,
    created_by: Uuid,
    valid_for: &str,
) -> Uuid {
    let quotation_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO quotations
            (id, quotation_number, customer_id, currency, status, subtotal, charges_total, grand_total,
             valid_until, sent_at, created_by)
        VALUES ($1, 'QT-TEST-' || nextval('quotation_number_seq'), $2, 'USD', 'SENT', 1250, 50, 1300,
                now() + $3::interval, now() - interval '1 day', $4)
        "#,
    )
    .bind(quotation_id)
    .bind(customer_id)
    .bind(valid_for)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("insert quotation");

    sqlx::query(
        r#"
        INSERT INTO quotation_items
            (id, quotation_id, line_number, description, quantity, unit_of_measure, unit_price, line_total)
        VALUES ($1, $2, 1, 'Ceylon black tea, bulk', 100, 'kg', 12.50, 1250)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(quotation_id)
    .execute(pool)
    .await
    .expect("insert quotation item");

    sqlx::query(
        "INSERT INTO quotation_charges (id, quotation_id, kind, description, amount) VALUES ($1, $2, 'SHIPPING', 'Sea freight', 50)",
    )
    .bind(Uuid::now_v7())
    .bind(quotation_id)
    .execute(pool)
    .await
    .expect("insert quotation charge");

    quotation_id
}

pub async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("count query")
}
