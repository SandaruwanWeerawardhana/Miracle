//! Audit trail for security-relevant and business-critical actions.
//!
//! Call [`record`] with the SAME transaction as the change being audited, so the
//! audit entry and the change commit or roll back together.
//!
//! TODO: `GET /api/v1/audit-logs` (permission `audit.read`) with cursor pagination.

use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::shared::{ids::UserId, request_context::current_request_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    User(UserId),
    /// Background jobs, schedulers.
    System,
    /// Payment gateway webhooks and other integrations.
    External,
}

/// One audit record. `action` uses `<module>.<past_tense_verb>` naming,
/// e.g. `quotation.accepted`, `order.status_changed`, `supplier.verified`.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub actor: Actor,
    pub action: &'static str,
    pub entity_type: &'static str,
    pub entity_id: Option<Uuid>,
    /// Context such as `{"from": "SENT", "to": "ACCEPTED"}`. NEVER include
    /// passwords, tokens, card data or full documents.
    pub metadata: Value,
}

impl AuditEntry {
    pub fn new(
        actor: Actor,
        action: &'static str,
        entity_type: &'static str,
        entity_id: Uuid,
    ) -> Self {
        Self {
            actor,
            action,
            entity_type,
            entity_id: Some(entity_id),
            metadata: Value::Object(Default::default()),
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

pub async fn record(executor: impl PgExecutor<'_>, entry: AuditEntry) -> Result<(), sqlx::Error> {
    let (actor_type, actor_user_id) = match entry.actor {
        Actor::User(id) => ("USER", Some(id)),
        Actor::System => ("SYSTEM", None),
        Actor::External => ("EXTERNAL", None),
    };

    sqlx::query(
        r#"
        INSERT INTO audit_logs
            (id, actor_type, actor_user_id, action, entity_type, entity_id, request_id, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(actor_type)
    .bind(actor_user_id)
    .bind(entry.action)
    .bind(entry.entity_type)
    .bind(entry.entity_id)
    .bind(current_request_id())
    .bind(entry.metadata)
    .execute(executor)
    .await?;

    Ok(())
}
