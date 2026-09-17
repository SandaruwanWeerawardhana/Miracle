//! Business modules (bounded contexts).
//!
//! Dependency rules (enforced in code review, see docs/ARCHITECTURE.md):
//! 1. A module may use `crate::shared`, `crate::app::AppState` and `crate::jobs`.
//! 2. A module may use ANOTHER module only through what that module re-exports
//!    from its `mod.rs` (its public facade), never its internals.
//! 3. A module never reads or writes another module's tables directly.
//! 4. No dependency cycles between modules. Upstream modules (auth, permissions,
//!    audit) must not depend on downstream ones (quotations, orders, ...).

pub mod audit;
pub mod auth;
pub mod customers;
pub mod health;
pub mod notifications;
pub mod orders;
pub mod payments;
pub mod permissions;
pub mod quotations;
pub mod requirements;
pub mod suppliers;
pub mod users;

use axum::{Router, http::header};

use crate::{app::AppState, middleware::security_headers, shared::error::AppError};

pub fn api_v1_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/users", users::routes())
        .nest("/suppliers", suppliers::routes())
        .nest("/requirements", requirements::routes())
        .nest("/quotations", quotations::routes())
        .nest("/orders", orders::routes())
        .fallback(|| async { AppError::not_found("ROUTE_NOT_FOUND", "Route not found") })
        .layer(security_headers::set(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; frame-ancestors 'none'",
        ))
        .layer(security_headers::set(header::CACHE_CONTROL, "no-store"))
}
