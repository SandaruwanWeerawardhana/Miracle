//! Background jobs backed by the PostgreSQL `background_jobs` table.
//!
//! Why PostgreSQL rather than Redis for the queue: jobs are enqueued inside the
//! business transaction (transactional outbox), so a job exists if and only if
//! the change that caused it was committed. Redis stays available for rate
//! limiting and caching. The queue is only reachable through [`enqueue`] and
//! [`worker::run`], so swapping the backend later touches only this folder.
//!
//! Adding a job: add a [`Job`] variant, handle it in `handlers.rs`. Handlers must
//! be idempotent — a job can run more than once after a crash.

mod handlers;
pub mod worker;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Job {
    SendEmail {
        to: String,
        template: String,
        variables: serde_json::Value,
    },
    /// Fan-out after a quotation is accepted (notify customer & sales, generate PDFs).
    QuotationAccepted { quotation_id: Uuid, order_id: Uuid },
    // TODO: GenerateInvoicePdf, SendSms, ProcessUploadedDocument, SyncShipmentTracking,
    //       GenerateReport — add as their modules are built.
}

impl Job {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SendEmail { .. } => "send_email",
            Self::QuotationAccepted { .. } => "quotation_accepted",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct EnqueueOptions {
    pub run_at: Option<DateTime<Utc>>,
    /// A second enqueue with the same key is silently ignored.
    pub idempotency_key: Option<String>,
}

/// Enqueues a job. Pass the business transaction as the executor.
pub async fn enqueue(
    executor: impl PgExecutor<'_>,
    job: &Job,
    options: EnqueueOptions,
) -> Result<(), sqlx::Error> {
    let payload =
        serde_json::to_value(job).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;

    sqlx::query(
        r#"
        INSERT INTO background_jobs (id, kind, payload, run_at, idempotency_key)
        VALUES ($1, $2, $3, COALESCE($4, now()), $5)
        ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(job.kind())
    .bind(payload)
    .bind(options.run_at)
    .bind(options.idempotency_key)
    .execute(executor)
    .await?;

    Ok(())
}
