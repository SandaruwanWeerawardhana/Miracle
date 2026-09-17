//! Genuinely cross-cutting building blocks with NO business knowledge.
//!
//! Rule: `shared` must never import from `crate::modules`. If code needs to know
//! what a quotation, order or supplier is, it belongs in that module.

pub mod auth;
pub mod error;
pub mod ids;
pub mod money;
pub mod pagination;
pub mod request_context;
pub mod response;
pub mod storage;
pub mod validation;
