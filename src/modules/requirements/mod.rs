//! Customer requirements (product sourcing requests and general requirements).
//!
//! Flat layout: `domain.rs`, `dto.rs`, `repository.rs`, `api.rs`.
//! Demonstrates typed filtering, safe dynamic SQL and `.own` access scoping.

pub(crate) mod api;
mod domain;
pub(crate) mod dto;
mod repository;

pub use api::routes;
pub use domain::{RequirementId, RequirementKind, RequirementStatus};
