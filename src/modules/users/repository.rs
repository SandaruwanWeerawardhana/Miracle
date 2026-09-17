//! Database rows are private to this file and converted into domain types, so
//! a new column never silently reaches an API response.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};

use super::domain::{User, UserCredentials, UserStatus};
use crate::shared::ids::UserId;

#[derive(FromRow)]
struct UserRow {
    id: UserId,
    email: String,
    full_name: String,
    phone: Option<String>,
    account_type: String,
    status: String,
    email_verified_at: Option<DateTime<Utc>>,
    preferred_language: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = sqlx::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            email: row.email,
            full_name: row.full_name,
            phone: row.phone,
            account_type: row.account_type.parse().map_err(decode_error)?,
            status: row.status.parse().map_err(decode_error)?,
            email_verified_at: row.email_verified_at,
            preferred_language: row.preferred_language,
            created_at: row.created_at,
        })
    }
}

pub async fn find_by_id(
    executor: impl PgExecutor<'_>,
    id: UserId,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email::text AS email, full_name, phone, account_type, status,
               email_verified_at, preferred_language, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(executor)
    .await?
    .map(User::try_from)
    .transpose()
}

#[derive(FromRow)]
struct CredentialsRow {
    id: UserId,
    password_hash: String,
    status: String,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
}

pub async fn find_credentials_by_email(
    executor: impl PgExecutor<'_>,
    email: &str,
) -> Result<Option<UserCredentials>, sqlx::Error> {
    let row = sqlx::query_as::<_, CredentialsRow>(
        r#"
        SELECT id, password_hash, status, failed_login_attempts, locked_until
        FROM users
        WHERE email = $1::citext
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await?;

    row.map(|row| {
        Ok(UserCredentials {
            user_id: row.id,
            password_hash: row.password_hash,
            status: row.status.parse::<UserStatus>().map_err(decode_error)?,
            failed_login_attempts: row.failed_login_attempts,
            locked_until: row.locked_until,
        })
    })
    .transpose()
}

fn decode_error(message: String) -> sqlx::Error {
    sqlx::Error::Decode(message.into())
}
