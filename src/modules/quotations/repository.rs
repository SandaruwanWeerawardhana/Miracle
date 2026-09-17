use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::domain::{ChargeKind, Quotation, QuotationCharge, QuotationId, QuotationItem};
use crate::{
    modules::{customers::CustomerId, suppliers::SupplierId},
    shared::money::{Currency, Money},
};

#[derive(FromRow)]
struct QuotationRow {
    id: QuotationId,
    quotation_number: String,
    customer_id: CustomerId,
    requirement_id: Option<Uuid>,
    currency: String,
    status: String,
    valid_until: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    responded_at: Option<DateTime<Utc>>,
    rejection_reason: Option<String>,
    version: i32,
}

#[derive(FromRow)]
struct ItemRow {
    line_number: i32,
    product_id: Option<Uuid>,
    supplier_id: Option<SupplierId>,
    description: String,
    quantity: Decimal,
    unit_of_measure: String,
    unit_price: Decimal,
}

#[derive(FromRow)]
struct ChargeRow {
    kind: String,
    description: String,
    amount: Decimal,
}

/// Loads the quotation with its lines and charges, locking the header row until
/// the surrounding transaction ends. Must be called inside a transaction.
pub async fn find_for_update(
    conn: &mut PgConnection,
    id: QuotationId,
) -> Result<Option<Quotation>, sqlx::Error> {
    let Some(row) = sqlx::query_as::<_, QuotationRow>(
        r#"
        SELECT id, quotation_number, customer_id, requirement_id, currency::text AS currency,
               status, valid_until, sent_at, responded_at, rejection_reason, version
        FROM quotations
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(None);
    };

    let items = sqlx::query_as::<_, ItemRow>(
        r#"
        SELECT line_number, product_id, supplier_id, description, quantity, unit_of_measure, unit_price
        FROM quotation_items
        WHERE quotation_id = $1
        ORDER BY line_number
        "#,
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    let charges = sqlx::query_as::<_, ChargeRow>(
        "SELECT kind, description, amount FROM quotation_charges WHERE quotation_id = $1 ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    to_domain(row, items, charges).map(Some)
}

/// Persists a status change using optimistic concurrency on `version`.
/// Returns `false` when another request changed the row first.
pub async fn update_status(
    conn: &mut PgConnection,
    quotation: &Quotation,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE quotations
        SET status = $2, sent_at = $3, valid_until = $4, responded_at = $5,
            rejection_reason = $6, version = version + 1
        WHERE id = $1 AND version = $7
        "#,
    )
    .bind(quotation.id)
    .bind(quotation.status.as_str())
    .bind(quotation.sent_at)
    .bind(quotation.valid_until)
    .bind(quotation.responded_at)
    .bind(quotation.rejection_reason.as_deref())
    .bind(quotation.version)
    .execute(conn)
    .await?;

    Ok(result.rows_affected() == 1)
}

fn to_domain(
    row: QuotationRow,
    items: Vec<ItemRow>,
    charges: Vec<ChargeRow>,
) -> Result<Quotation, sqlx::Error> {
    let currency: Currency = row.currency.parse().map_err(decode)?;
    let money = |amount: Decimal| Money::new(amount, currency);

    Ok(Quotation {
        id: row.id,
        quotation_number: row.quotation_number,
        customer_id: row.customer_id,
        requirement_id: row.requirement_id,
        currency,
        status: row.status.parse().map_err(|error: String| decode(error))?,
        items: items
            .into_iter()
            .map(|item| QuotationItem {
                line_number: item.line_number,
                product_id: item.product_id,
                supplier_id: item.supplier_id,
                description: item.description,
                quantity: item.quantity,
                unit_of_measure: item.unit_of_measure,
                unit_price: money(item.unit_price),
            })
            .collect(),
        charges: charges
            .into_iter()
            .map(|charge| {
                Ok(QuotationCharge {
                    kind: charge
                        .kind
                        .parse::<ChargeKind>()
                        .map_err(|error: String| decode(error))?,
                    description: charge.description,
                    amount: money(charge.amount),
                })
            })
            .collect::<Result<_, sqlx::Error>>()?,
        valid_until: row.valid_until,
        sent_at: row.sent_at,
        responded_at: row.responded_at,
        rejection_reason: row.rejection_reason,
        version: row.version,
    })
}

fn decode(error: impl std::fmt::Display) -> sqlx::Error {
    sqlx::Error::Decode(error.to_string().into())
}
