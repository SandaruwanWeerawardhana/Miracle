//! Security primitives shared by every module:
//!
//! * password hashing and token issuance/verification,
//! * the permission catalogue,
//! * the [`CurrentUser`] extractor used by every protected handler.
//!
//! Authentication *flows* (register, login, refresh, logout) and the database
//! backed [`PrincipalResolver`] implementation live in `modules::auth`. The trait
//! keeps `shared` independent of modules and lets API tests use a fake resolver.

mod current_user;
mod password;
mod permission;
mod tokens;

pub use current_user::{AccessScope, AccountType, CurrentUser, PrincipalResolver};
pub use password::{PasswordError, hash_password, verify_password};
pub use permission::{Permission, PermissionSet, UnknownPermission};
pub use tokens::{AccessClaims, OpaqueToken, TokenError, TokenService};
