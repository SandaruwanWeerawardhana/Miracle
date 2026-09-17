//! Customer profiles (`customers` table).
//!
//! Currently only the facade other modules need for ownership checks.
//! TODO: profile read/update API (`/api/v1/customers/me`, staff views with
//! `customer.read` / `customer.update`).

use sqlx::PgExecutor;

use crate::shared::ids::UserId;

crate::typed_id!(
    /// Identifier of a row in `customers`.
    CustomerId
);

/// The customer profile owned by `user_id`, if the user is a customer.
pub async fn find_id_by_user(
    executor: impl PgExecutor<'_>,
    user_id: UserId,
) -> Result<Option<CustomerId>, sqlx::Error> {
    sqlx::query_scalar::<_, CustomerId>("SELECT id FROM customers WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(executor)
        .await
}
