use std::time::Duration;

use miracle_backend::jobs::{self, EnqueueOptions, Job, worker};
use serde_json::json;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::common::test_state;

#[sqlx::test]
#[ignore = "requires PostgreSQL (DATABASE_URL)"]
async fn worker_processes_enqueued_job_and_deduplicates_by_key(pool: PgPool) {
    let state = test_state(pool.clone());
    let job = Job::SendEmail {
        to: "customer@example.test".into(),
        template: "welcome".into(),
        variables: json!({}),
    };
    let options = || EnqueueOptions {
        idempotency_key: Some("welcome:customer@example.test".into()),
        ..Default::default()
    };

    jobs::enqueue(&pool, &job, options()).await.unwrap();
    jobs::enqueue(&pool, &job, options()).await.unwrap();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM background_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(total, 1, "idempotency key must deduplicate");

    let shutdown = CancellationToken::new();
    let handle = tokio::spawn(worker::run(state, shutdown.clone()));

    let mut status = String::new();
    for _ in 0..50 {
        status = sqlx::query_scalar("SELECT status FROM background_jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        if status == "SUCCEEDED" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    shutdown.cancel();
    handle.await.unwrap();
    assert_eq!(status, "SUCCEEDED");
}
