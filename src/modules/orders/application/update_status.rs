//! Use case: UpdateOrderStatus. Status can only move along the transitions
//! defined by `OrderStatus::can_transition_to`; there is no "set status" API.

use serde_json::json;
use uuid::Uuid;

use crate::{
    app::AppState,
    modules::{
        audit::{self, Actor, AuditEntry},
        orders::domain::{OrderError, OrderId, OrderStatus},
    },
    shared::{
        auth::{CurrentUser, Permission},
        error::AppResult,
    },
};

pub struct UpdateOrderStatus {
    pub order_id: OrderId,
    pub status: OrderStatus,
    pub note: Option<String>,
    /// Whether the change is shown on the customer's tracking timeline.
    pub is_public: bool,
}

pub struct StatusChanged {
    pub order_id: OrderId,
    pub from: OrderStatus,
    pub to: OrderStatus,
}

pub async fn execute(
    state: &AppState,
    actor: &CurrentUser,
    command: UpdateOrderStatus,
) -> AppResult<StatusChanged> {
    if command.status == OrderStatus::Cancelled {
        actor.require(Permission::OrderCancel)?;
        if command
            .note
            .as_deref()
            .is_none_or(|note| note.trim().is_empty())
        {
            return Err(OrderError::CancellationReasonRequired.into());
        }
    } else {
        actor.require(Permission::OrderUpdate)?;
    }

    let mut tx = state.db.begin().await?;

    let current: String = sqlx::query_scalar("SELECT status FROM orders WHERE id = $1 FOR UPDATE")
        .bind(command.order_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(OrderError::NotFound)?;
    let from: OrderStatus = current
        .parse()
        .map_err(|error: String| sqlx::Error::Decode(error.into()))?;

    if !from.can_transition_to(command.status) {
        return Err(OrderError::InvalidTransition {
            from,
            to: command.status,
        }
        .into());
    }

    let cancellation_reason = (command.status == OrderStatus::Cancelled)
        .then(|| command.note.clone())
        .flatten();

    sqlx::query(
        r#"
        UPDATE orders
        SET status = $2,
            cancellation_reason = COALESCE($3, cancellation_reason),
            version = version + 1
        WHERE id = $1
        "#,
    )
    .bind(command.order_id)
    .bind(command.status.as_str())
    .bind(cancellation_reason)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO order_status_history (id, order_id, from_status, to_status, note, is_public, changed_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(command.order_id)
    .bind(from.as_str())
    .bind(command.status.as_str())
    .bind(command.note.as_deref())
    .bind(command.is_public)
    .bind(actor.user_id)
    .execute(&mut *tx)
    .await?;

    audit::record(
        &mut *tx,
        AuditEntry::new(
            Actor::User(actor.user_id),
            "order.status_changed",
            "order",
            command.order_id.as_uuid(),
        )
        .with_metadata(json!({ "from": from, "to": command.status })),
    )
    .await?;

    // TODO: enqueue an OrderStatusChanged notification job (customer email / in-app).

    tx.commit().await?;

    Ok(StatusChanged {
        order_id: command.order_id,
        from,
        to: command.status,
    })
}
