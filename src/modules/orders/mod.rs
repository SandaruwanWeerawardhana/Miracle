//! Wholesale orders and order tracking (status timeline).
//!
//! Depends on: `customers`, `suppliers` (IDs), `audit`.
//! Used by: `quotations` (order creation on acceptance), later `payments`,
//! `invoices`, `logistics`.
//!
//! TODO: list/get endpoints with typed filters, customer tracking timeline
//! (`GET /orders/{id}/timeline`), OrderStatusChanged notification job.

pub(crate) mod api;
mod application;
mod domain;

pub use api::routes;
pub use application::create_from_quotation::{
    NewOrderFromQuotation, NewOrderItem, create_from_quotation,
};
pub use domain::{OrderError, OrderId, OrderStatus};
