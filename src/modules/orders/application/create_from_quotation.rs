//! Use case: CreateOrderFromQuotation.
//!
//! Takes a connection instead of the pool: it is always one step inside the
//! caller's transaction (quotation acceptance), never a transaction of its own.

use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    modules::{
        customers::CustomerId,
        orders::domain::{OrderError, OrderId, OrderStatus},
        suppliers::SupplierId,
    },
    shared::{error::AppError, ids::UserId, money::Money},
};

/// Input copied from an accepted quotation. Orders keep their own copy of lines
/// and prices so later quotation edits can never change a placed order.
pub struct NewOrderFromQuotation {
    pub customer_id: CustomerId,
    pub quotation_id: Uuid,
    pub subtotal: Money,
    pub charges_total: Money,
    pub grand_total: Money,
    pub items: Vec<NewOrderItem>,
    pub created_by: UserId,
}

pub struct NewOrderItem {
    pub line_number: i32,
    pub product_id: Option<Uuid>,
    pub supplier_id: Option<SupplierId>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_of_measure: String,
    pub unit_price: Money,
    pub line_total: Money,
}

pub async fn create_from_quotation(
    conn: &mut PgConnection,
    input: NewOrderFromQuotation,
) -> Result<OrderId, AppError> {
    if input.items.is_empty() {
        return Err(OrderError::NoItems.into());
    }

    let order_id = OrderId::new();
    let currency = input.grand_total.currency();

    sqlx::query(
        r#"
        INSERT INTO orders
            (id, order_number, customer_id, quotation_id, currency, status,
             subtotal, charges_total, grand_total, created_by)
        VALUES
            ($1, 'ORD-' || to_char(now(), 'YYYY') || '-' || lpad(nextval('order_number_seq')::text, 6, '0'),
             $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(order_id)
    .bind(input.customer_id)
    .bind(input.quotation_id)
    .bind(currency.code())
    .bind(OrderStatus::Pending.as_str())
    .bind(input.subtotal.amount())
    .bind(input.charges_total.amount())
    .bind(input.grand_total.amount())
    .bind(input.created_by)
    .execute(&mut *conn)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db) if db.constraint() == Some("orders_quotation_id_key") => {
            AppError::from(OrderError::AlreadyExistsForQuotation)
        }
        _ => AppError::from(error),
    })?;

    for item in &input.items {
        sqlx::query(
            r#"
            INSERT INTO order_items
                (id, order_id, line_number, product_id, supplier_id, description,
                 quantity, unit_of_measure, unit_price, line_total)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(order_id)
        .bind(item.line_number)
        .bind(item.product_id)
        .bind(item.supplier_id)
        .bind(&item.description)
        .bind(item.quantity)
        .bind(&item.unit_of_measure)
        .bind(item.unit_price.amount())
        .bind(item.line_total.amount())
        .execute(&mut *conn)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO order_status_history (id, order_id, from_status, to_status, note, changed_by)
        VALUES ($1, $2, NULL, $3, 'Order created from accepted quotation', $4)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(order_id)
    .bind(OrderStatus::Pending.as_str())
    .bind(input.created_by)
    .execute(&mut *conn)
    .await?;

    Ok(order_id)
}
