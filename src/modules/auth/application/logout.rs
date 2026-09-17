use crate::{
    app::AppState,
    modules::auth::sessions,
    shared::{auth::CurrentUser, error::AppResult},
};

/// Revokes the caller's current session. Outstanding access tokens for this
/// session stop working immediately because every request checks the session.
pub async fn execute(state: &AppState, user: &CurrentUser) -> AppResult<()> {
    sessions::revoke_sessions(&state.db, user.user_id, Some(user.session_id), "logout").await?;
    // TODO: audit `auth.logout` once login is implemented.
    Ok(())
}
