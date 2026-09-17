use async_trait::async_trait;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    modules::{permissions, users},
    shared::{
        auth::{AccessClaims, CurrentUser, PrincipalResolver},
        error::AppError,
        ids::UserId,
    },
};

/// Resolves bearer-token claims into a `CurrentUser` on every authenticated request.
pub struct PgPrincipalResolver {
    db: PgPool,
}

impl PgPrincipalResolver {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PrincipalResolver for PgPrincipalResolver {
    async fn resolve(&self, claims: &AccessClaims) -> Result<Option<CurrentUser>, AppError> {
        let user_id = UserId::from_uuid(claims.sub);

        // A logged-out or revoked session invalidates its access tokens immediately.
        if !is_session_active(&self.db, claims.sid, user_id).await? {
            return Ok(None);
        }

        let Some(user) = users::find_by_id(&self.db, user_id).await? else {
            return Ok(None);
        };
        if !user.status.can_authenticate() {
            return Ok(None);
        }

        let permissions =
            permissions::load_permissions_for_user(&self.db, user_id.as_uuid()).await?;

        Ok(Some(CurrentUser {
            user_id,
            session_id: claims.sid,
            account_type: user.account_type,
            permissions,
        }))
    }
}

async fn is_session_active(
    executor: impl PgExecutor<'_>,
    session_id: Uuid,
    user_id: UserId,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM user_sessions
            WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL AND expires_at > now()
        )
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_one(executor)
    .await
}

/// Revokes a single session (logout) or, with `session_id = None`, every session of
/// the user (password reset, account disable, "log out everywhere").
pub async fn revoke_sessions(
    executor: impl PgExecutor<'_>,
    user_id: UserId,
    session_id: Option<Uuid>,
    reason: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE user_sessions
        SET revoked_at = now(), revoked_reason = $3
        WHERE user_id = $1 AND revoked_at IS NULL AND ($2::uuid IS NULL OR id = $2)
        "#,
    )
    .bind(user_id)
    .bind(session_id)
    .bind(reason)
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}
