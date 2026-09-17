//! User accounts (`users` table).
//!
//! Upstream module: owns the account record, including the password hash, which
//! never leaves this module except through [`UserCredentials`] for `auth`.
//!
//! TODO: staff/admin endpoints (list, disable, enable) behind `user.manage`,
//! revoking sessions and writing audit records on disable.

pub(crate) mod api;
mod domain;
mod repository;

pub use api::routes;
pub use domain::{User, UserCredentials, UserStatus};
pub use repository::{find_by_id, find_credentials_by_email};
