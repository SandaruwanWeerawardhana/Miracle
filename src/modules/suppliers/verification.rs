//! Use case: VerifySupplier. Reference example of a transactional workflow:
//! lock → apply domain rule → persist → history → audit → outbox job → commit.

use serde_json::json;
use uuid::Uuid;

use super::domain::{SupplierError, SupplierId, VerificationStatus};
use crate::{
    app::AppState,
    modules::audit::{self, Actor, AuditEntry},
    shared::{
        auth::{CurrentUser, Permission},
        error::AppResult,
    },
};

pub struct RecordVerificationDecision {
    pub supplier_id: SupplierId,
    pub decision: VerificationStatus,
    pub notes: Option<String>,
}

pub struct VerificationOutcome {
    pub supplier_id: SupplierId,
    pub previous: VerificationStatus,
    pub current: VerificationStatus,
}

pub async fn record_decision(
    state: &AppState,
    actor: &CurrentUser,
    command: RecordVerificationDecision,
) -> AppResult<VerificationOutcome> {
    actor.require(Permission::SupplierVerify)?;

    if !command.decision.is_review_decision() {
        return Err(SupplierError::NotAReviewDecision(command.decision).into());
    }

    let mut tx = state.db.begin().await?;

    // Row lock prevents two reviewers deciding concurrently.
    let current: String =
        sqlx::query_scalar("SELECT verification_status FROM suppliers WHERE id = $1 FOR UPDATE")
            .bind(command.supplier_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(SupplierError::NotFound)?;
    let previous: VerificationStatus = current
        .parse()
        .map_err(|error: String| sqlx::Error::Decode(error.into()))?;

    if !previous.can_transition_to(command.decision) {
        return Err(SupplierError::InvalidVerificationTransition {
            from: previous,
            to: command.decision,
        }
        .into());
    }

    sqlx::query(
        r#"
        UPDATE suppliers
        SET verification_status = $2,
            verified_at = CASE WHEN $2 = 'VERIFIED' THEN now() ELSE verified_at END
        WHERE id = $1
        "#,
    )
    .bind(command.supplier_id)
    .bind(command.decision.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO supplier_verification_reviews
            (id, supplier_id, from_status, to_status, notes, reviewed_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(command.supplier_id)
    .bind(previous.as_str())
    .bind(command.decision.as_str())
    .bind(command.notes.as_deref())
    .bind(actor.user_id)
    .execute(&mut *tx)
    .await?;

    audit::record(
        &mut *tx,
        AuditEntry::new(
            Actor::User(actor.user_id),
            "supplier.verification_changed",
            "supplier",
            command.supplier_id.as_uuid(),
        )
        .with_metadata(json!({ "from": previous, "to": command.decision })),
    )
    .await?;

    // TODO: enqueue a notification job to the supplier contact (SupplierVerified event).

    tx.commit().await?;

    Ok(VerificationOutcome {
        supplier_id: command.supplier_id,
        previous,
        current: command.decision,
    })
}
