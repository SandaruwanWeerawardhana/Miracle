use axum::{Json, Router, extract::State, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use super::{domain::User, repository};
use crate::{
    app::AppState,
    shared::{
        auth::{AccountType, CurrentUser},
        error::{AppError, AppResult},
        ids::UserId,
        response::DataResponse,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

/// Public representation of a user. Never add credential or security fields here.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: UserId,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub account_type: AccountType,
    pub email_verified: bool,
    pub preferred_language: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            phone: user.phone,
            account_type: user.account_type,
            email_verified: user.email_verified_at.is_some(),
            preferred_language: user.preferred_language,
            created_at: user.created_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The authenticated user", body = DataResponse<UserResponse>),
        (status = 401, description = "Not authenticated", body = crate::shared::error::ErrorResponse),
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<DataResponse<UserResponse>>> {
    let account = repository::find_by_id(&state.db, user.user_id)
        .await?
        .ok_or_else(|| AppError::not_found("USER_NOT_FOUND", "User was not found"))?;

    Ok(Json(DataResponse::new(account.into())))
}
