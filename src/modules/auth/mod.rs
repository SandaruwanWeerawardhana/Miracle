//! Authentication: registration, email verification, login, refresh, logout,
//! password reset and session revocation.
//!
//! Depends on: `users` (accounts), `permissions` (effective permissions), `audit`.
//! Provides: [`PgPrincipalResolver`], plugged into `AppState` so that the shared
//! `CurrentUser` extractor can validate sessions.
//!
//! Layout:
//! * `domain.rs`         — auth errors and session rules
//! * `application/`      — one file per use case
//! * `sessions.rs`       — `user_sessions` persistence and principal resolution
//! * `api/`              — routes, handlers, request/response DTOs

pub(crate) mod api;
mod application;
mod domain;
mod sessions;

pub use api::routes;
pub use domain::AuthError;
pub use sessions::PgPrincipalResolver;
