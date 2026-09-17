use sqlx::PgExecutor;
use uuid::Uuid;

use crate::shared::auth::PermissionSet;

/// Effective permissions = union of the permissions of every role the user holds.
///
/// TODO: cache in Redis (`perm:{user_id}`, short TTL) and invalidate on role
/// changes once request volume justifies it.
pub async fn load_permissions_for_user(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<PermissionSet, sqlx::Error> {
    let codes: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT rp.permission_code
        FROM user_roles ur
        JOIN role_permissions rp ON rp.role_code = ur.role_code
        WHERE ur.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(executor)
    .await?;

    Ok(PermissionSet::from_codes(codes))
}
