//! Customer quotations.
//!
//! Full layered layout — the reference for complex modules:
//! * `domain/`        — entity, status machine, totals, errors (no I/O, unit tested)
//! * `application/`   — one file per use case; owns transaction boundaries
//! * `repository.rs`  — SQLx persistence; rows converted to domain types
//! * `api/`           — routes, handlers, DTOs
//!
//! Depends on: `customers`, `suppliers` (IDs), `orders` (order creation on
//! acceptance), `audit`, `jobs`.
//!
//! TODO: CreateQuotation, UpdateDraftQuotation, SendQuotation (PDF job + email),
//! RejectQuotation, CancelQuotation, scheduled expiry job, list/get endpoints.

pub(crate) mod api;
mod application;
mod domain;
mod repository;

pub use api::routes;
pub use domain::{QuotationError, QuotationId, QuotationStatus};
