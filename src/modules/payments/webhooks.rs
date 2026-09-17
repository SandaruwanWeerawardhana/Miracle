use sqlx::PgExecutor;
use uuid::Uuid;

use super::gateway::GatewayEvent;

pub enum RecordedEvent {
    /// First delivery: process it.
    New(Uuid),
    /// Redelivery of an event already recorded: acknowledge with 200, do nothing.
    Duplicate,
}

/// Idempotency for gateway webhooks: the `(provider, provider_event_id)` unique
/// constraint guarantees each event is recorded — and therefore processed — once,
/// even when the provider retries or delivers concurrently.
///
/// Call only with an event returned by `PaymentGateway::verify_webhook`.
/// TODO: processing step — in one transaction, lock the payment, apply
/// `PaymentStatus` transition, update schedule `amount_paid`/status, audit,
/// enqueue receipt + notification jobs, set `processed_at`.
pub async fn record_gateway_event(
    executor: impl PgExecutor<'_>,
    provider: &str,
    event: &GatewayEvent,
) -> Result<RecordedEvent, sqlx::Error> {
    let inserted: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO payment_gateway_events (id, provider, provider_event_id, event_type, payload)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (provider, provider_event_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(provider)
    .bind(&event.provider_event_id)
    .bind(&event.event_type)
    .bind(&event.raw_payload)
    .fetch_optional(executor)
    .await?;

    Ok(inserted.map_or(RecordedEvent::Duplicate, RecordedEvent::New))
}
