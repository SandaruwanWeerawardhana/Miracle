//! Filtered listing uses `QueryBuilder` with `push_bind`, so every user-supplied
//! value is a bind parameter. Never `format!` user input into SQL.
//!
//! Fixed-shape queries elsewhere should move to `sqlx::query_as!` (compile-time
//! checked) once `.sqlx` offline data is generated — see docs/ARCHITECTURE.md.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

use super::{
    domain::RequirementId,
    dto::{ListRequirementsQuery, RequirementSummaryResponse},
};
use crate::{modules::customers::CustomerId, shared::pagination::PageRequest};

#[derive(FromRow)]
struct RequirementSummaryRow {
    id: RequirementId,
    requirement_number: String,
    kind: String,
    title: String,
    status: String,
    destination_country: String,
    required_by: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

impl TryFrom<RequirementSummaryRow> for RequirementSummaryResponse {
    type Error = sqlx::Error;

    fn try_from(row: RequirementSummaryRow) -> Result<Self, Self::Error> {
        let decode = |message: String| sqlx::Error::Decode(message.into());
        Ok(Self {
            id: row.id,
            requirement_number: row.requirement_number,
            kind: row.kind.parse().map_err(decode)?,
            title: row.title,
            status: row.status.parse().map_err(decode)?,
            destination_country: row.destination_country,
            required_by: row.required_by,
            created_at: row.created_at,
        })
    }
}

/// `customer = Some(..)` restricts results to that customer (the `.own` scope).
pub async fn list(
    db: &PgPool,
    customer: Option<CustomerId>,
    filter: &ListRequirementsQuery,
    page: PageRequest,
) -> Result<(Vec<RequirementSummaryResponse>, i64), sqlx::Error> {
    let mut count = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM requirements r WHERE TRUE");
    push_filters(&mut count, customer, filter);
    let total: i64 = count.build_query_scalar().fetch_one(db).await?;

    let mut select = QueryBuilder::<Postgres>::new(
        r#"
        SELECT r.id, r.requirement_number, r.kind, r.title, r.status,
               r.destination_country::text AS destination_country, r.required_by, r.created_at
        FROM requirements r
        WHERE TRUE
        "#,
    );
    push_filters(&mut select, customer, filter);
    select
        .push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ")
        .push_bind(page.limit())
        .push(" OFFSET ")
        .push_bind(page.offset());

    let rows = select
        .build_query_as::<RequirementSummaryRow>()
        .fetch_all(db)
        .await?;

    let items = rows
        .into_iter()
        .map(RequirementSummaryResponse::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((items, total))
}

fn push_filters(
    builder: &mut QueryBuilder<Postgres>,
    customer: Option<CustomerId>,
    filter: &ListRequirementsQuery,
) {
    if let Some(customer_id) = customer {
        builder.push(" AND r.customer_id = ").push_bind(customer_id);
    }
    if let Some(status) = filter.status {
        builder.push(" AND r.status = ").push_bind(status.as_str());
    }
    if let Some(kind) = filter.kind {
        builder.push(" AND r.kind = ").push_bind(kind.as_str());
    }
    if let Some(country) = &filter.destination_country {
        builder
            .push(" AND r.destination_country = ")
            .push_bind(country.to_ascii_uppercase());
    }
    if let Some(from) = filter.created_from {
        builder.push(" AND r.created_at >= ").push_bind(from);
    }
    if let Some(to) = filter.created_to {
        builder.push(" AND r.created_at < ").push_bind(to);
    }
    if let Some(search) = &filter.search {
        builder
            .push(r" AND r.title ILIKE ")
            .push_bind(format!("%{}%", escape_like(search)))
            .push(r" ESCAPE '\'");
    }
}

/// Escapes LIKE wildcards so user input matches literally.
fn escape_like(input: &str) -> String {
    input
        .replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_")
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(escape_like(r"50%_off\"), r"50\%\_off\\");
    }
}
