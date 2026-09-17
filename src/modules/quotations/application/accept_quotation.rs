//! Use case: AcceptQuotation — the reference cross-module transactional workflow.
//!
//! ```text
//! BEGIN
//!   lock quotation ─► domain rule (SENT, not expired) ─► save status
//!   ─► orders::create_from_quotation (order + items + timeline)
//!   ─► payment schedule (TODO) ─► invoice record (TODO)
//!   ─► audit record ─► outbox job (notifications, PDFs)
//! COMMIT
//! ```
//! Any failure rolls back everything: `tx` is dropped without commit.

use chrono::Utc;
use serde_json::json;

use crate::{
    app::AppState,
    jobs::{self, EnqueueOptions, Job},
    modules::{
        audit::{self, Actor, AuditEntry},
        customers,
        orders::{self, NewOrderFromQuotation, NewOrderItem, OrderId},
        quotations::{
            domain::{Quotation, QuotationError, QuotationId},
            repository,
        },
    },
    shared::{
        auth::{CurrentUser, Permission},
        error::{AppError, AppResult},
    },
};

pub struct AcceptedQuotation {
    pub quotation_id: QuotationId,
    pub order_id: OrderId,
}

pub async fn execute(
    state: &AppState,
    actor: &CurrentUser,
    quotation_id: QuotationId,
) -> AppResult<AcceptedQuotation> {
    actor.require(Permission::QuotationRespondOwn)?;
    let customer_id = customers::find_id_by_user(&state.db, actor.user_id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let mut tx = state.db.begin().await?;

    let mut quotation = repository::find_for_update(&mut tx, quotation_id)
        .await?
        .ok_or(QuotationError::NotFound)?;

    // Report "not found" rather than "forbidden" so IDs of other customers'
    // quotations cannot be probed.
    if quotation.customer_id != customer_id {
        return Err(QuotationError::NotFound.into());
    }

    let now = Utc::now();
    quotation.accept(now)?;
    if !repository::update_status(&mut tx, &quotation).await? {
        return Err(QuotationError::ConcurrentModification.into());
    }

    let order_id = orders::create_from_quotation(&mut tx, order_input(&quotation, actor)?).await?;

    // TODO: payments::create_payment_schedule(&mut tx, order_id, terms) once payment
    //       terms are captured on the quotation (FULL / ADVANCE + balance / MILESTONE).
    // TODO: invoices::create_proforma_invoice(&mut tx, order_id) when the invoices module lands.

    audit::record(
        &mut *tx,
        AuditEntry::new(
            Actor::User(actor.user_id),
            "quotation.accepted",
            "quotation",
            quotation.id.as_uuid(),
        )
        .with_metadata(json!({
            "quotation_number": quotation.quotation_number,
            "order_id": order_id,
        })),
    )
    .await?;

    jobs::enqueue(
        &mut *tx,
        &Job::QuotationAccepted {
            quotation_id: quotation.id.as_uuid(),
            order_id: order_id.as_uuid(),
        },
        EnqueueOptions {
            idempotency_key: Some(format!("quotation_accepted:{}", quotation.id)),
            ..Default::default()
        },
    )
    .await?;

    tx.commit().await?;

    tracing::info!(%quotation_id, %order_id, "quotation accepted");
    Ok(AcceptedQuotation {
        quotation_id,
        order_id,
    })
}

fn order_input(
    quotation: &Quotation,
    actor: &CurrentUser,
) -> Result<NewOrderFromQuotation, QuotationError> {
    let totals = quotation.totals()?;
    let items = quotation
        .items
        .iter()
        .map(|item| {
            Ok(NewOrderItem {
                line_number: item.line_number,
                product_id: item.product_id,
                supplier_id: item.supplier_id,
                description: item.description.clone(),
                quantity: item.quantity,
                unit_of_measure: item.unit_of_measure.clone(),
                unit_price: item.unit_price,
                line_total: item.line_total()?,
            })
        })
        .collect::<Result<Vec<_>, QuotationError>>()?;

    Ok(NewOrderFromQuotation {
        customer_id: quotation.customer_id,
        quotation_id: quotation.id.as_uuid(),
        subtotal: totals.subtotal,
        charges_total: totals.charges_total,
        grand_total: totals.grand_total,
        items,
        created_by: actor.user_id,
    })
}
