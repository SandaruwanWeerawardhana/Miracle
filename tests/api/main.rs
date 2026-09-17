//! HTTP-level tests: the real router, middleware and error handling, driven
//! in-process with `tower::ServiceExt::oneshot` (no network port).
//!
//! Run everything, including database tests:
//!   DATABASE_URL=postgres://mirac:5432/miracle_test \
//!     cargo test -- --include-ignored

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "panicking is the failure mode in tests"
)]

mod common;
mod health_and_errors;
mod jobs;
mod orders;
mod quotations;
mod requirements;
mod sessions;
mod suppliers;
