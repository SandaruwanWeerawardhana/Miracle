//! Handlers stay thin: extract + validate, call one use case, map the result.

use axum::{Json, extract::State, http::StatusCode};

use super::dto::{LoginRequest, RegisterRequest, RegistrationAccountType, TokenResponse};
use crate::{
    app::AppState,
    modules::auth::application::{login, logout, register},
    shared::{
        auth::{AccountType, CurrentUser},
        error::{AppResult, ErrorResponse},
        validation::ValidatedJson,
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 202, description = "Registration accepted; verification email sent"),
        (status = 422, description = "Validation failed", body = ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<RegisterRequest>,
) -> AppResult<StatusCode> {
    let command = register::RegisterUser {
        email: request.email.trim().to_lowercase(),
        password: request.password,
        full_name: request.full_name.trim().to_owned(),
        account_type: match request.account_type {
            RegistrationAccountType::Customer => AccountType::Customer,
            RegistrationAccountType::Supplier => AccountType::Supplier,
        },
    };
    register::execute(&state, command).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated", body = TokenResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 429, description = "Too many attempts", body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<LoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let tokens = login::execute(
        &state,
        login::LoginUser {
            email: request.email.trim().to_lowercase(),
            password: request.password,
            user_agent: None,
            ip_address: None,
        },
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token: tokens.access_token,
        token_type: "Bearer",
        expires_in: tokens.access_token_expires_in,
        refresh_token: Some(tokens.refresh_token),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Session revoked"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    )
)]
pub async fn logout(State(state): State<AppState>, user: CurrentUser) -> AppResult<StatusCode> {
    logout::execute(&state, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}
