pub mod dto;
pub mod handlers;

use axum::{Router, routing::patch};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/{order_id}/status", patch(handlers::update_status))
}
