use crate::{
    app::AppState,
    shared::{auth::AccountType, error::AppResult, ids::UserId},
};

#[expect(dead_code, reason = "used once the TODO use case is implemented")]
pub struct RegisterUser {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub account_type: AccountType,
}

/// Registers a customer or supplier account in `PENDING_VERIFICATION` state.
///
/// TODO: implement, in ONE transaction:
/// 1. Reject `AccountType::Staff` (staff are created by admins).
/// 2. `hash_password` (Argon2id) — never store or log the plaintext.
/// 3. Insert `users` row; map unique violation on email to
///    `AuthError::EmailAlreadyRegistered`. To prevent account enumeration, the
///    HTTP response may instead always be 202 with an email sent to the owner.
/// 4. Grant the default role (`CUSTOMER` / `SUPPLIER`) and create the profile
///    via the customers/suppliers module facade.
/// 5. Create an `EMAIL_VERIFICATION` token (`OpaqueToken`, hash stored).
/// 6. `jobs::enqueue(Job::SendEmail { .. })` with the verification link.
/// 7. Audit `auth.user_registered`.
pub async fn execute(_state: &AppState, _command: RegisterUser) -> AppResult<UserId> {
    Err(crate::shared::error::AppError::NotImplemented)
}
