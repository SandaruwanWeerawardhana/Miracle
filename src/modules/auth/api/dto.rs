use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 12, max = 128))]
    #[schema(format = Password)]
    pub password: String,
    #[validate(length(min = 1, max = 200))]
    pub full_name: String,
    pub account_type: RegistrationAccountType,
}

/// Staff accounts cannot self-register.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistrationAccountType {
    Customer,
    Supplier,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 1, max = 128))]
    #[schema(format = Password)]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    #[schema(example = "Bearer")]
    pub token_type: &'static str,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}
