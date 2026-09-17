//! Suppliers and supplier verification.
//!
//! A deliberately flat module: the domain is small, so it does not need the
//! `domain/ application/ infrastructure/ api/` directories used by quotations.
//! Split into directories when a file grows past what one screen can explain.
//!
//! TODO: supplier CRUD and listing (filters: country, verification status,
//! category), supplier products, and verification documents via `documents`.

pub(crate) mod api;
mod domain;
mod verification;

pub use api::routes;
pub use domain::{SupplierError, SupplierId, VerificationStatus};
