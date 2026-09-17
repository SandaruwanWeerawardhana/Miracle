pub mod dto;
pub mod handlers;

use axum::{Router, routing::post};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
    // TODO: POST /refresh, /verify-email, /forgot-password, /reset-password,
    //       and DELETE /sessions/{id} (session management).
}
