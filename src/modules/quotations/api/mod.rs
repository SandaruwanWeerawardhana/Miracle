pub mod dto;
pub mod handlers;

use axum::{Router, routing::post};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/{quotation_id}/accept", post(handlers::accept_quotation))
}
