use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

use super::domain::{RequirementId, RequirementKind, RequirementStatus};
use crate::shared::{
    money::Currency,
    pagination::{PageParams, PageRequest},
};

/// Query parameters for `GET /requirements`. Pagination fields are inline because
/// `serde(flatten)` does not work with numeric query parameters.
#[derive(Debug, Default, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListRequirementsQuery {
    pub status: Option<RequirementStatus>,
    pub kind: Option<RequirementKind>,
    /// ISO 3166-1 alpha-2 destination country, e.g. `LK`.
    #[validate(length(equal = 2))]
    pub destination_country: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    /// Case-insensitive search in the title.
    #[validate(length(min = 2, max = 100))]
    pub search: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl ListRequirementsQuery {
    pub fn page(&self) -> PageRequest {
        PageParams {
            page: self.page,
            limit: self.limit,
        }
        .resolve()
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[validate(schema(function = "validate_submit_request"))]
pub struct SubmitRequirementRequest {
    pub kind: RequirementKind,
    #[validate(length(min = 3, max = 200))]
    pub title: String,
    #[validate(length(min = 10, max = 10_000))]
    pub description: String,
    #[schema(value_type = Option<String>, example = "5000")]
    pub quantity: Option<Decimal>,
    #[validate(length(min = 1, max = 30))]
    pub unit_of_measure: Option<String>,
    #[schema(value_type = Option<String>, example = "2.35")]
    pub target_unit_price: Option<Decimal>,
    pub target_currency: Option<Currency>,
    #[validate(length(equal = 2))]
    pub destination_country: String,
    #[validate(length(equal = 2))]
    pub preferred_origin: Option<String>,
    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub required_by: Option<NaiveDate>,
}

/// Cross-field shape checks. Business rules (e.g. whether the customer may submit)
/// belong to the use case, not here.
fn validate_submit_request(request: &SubmitRequirementRequest) -> Result<(), ValidationError> {
    if request.kind == RequirementKind::Product
        && (request.quantity.is_none() || request.unit_of_measure.is_none())
    {
        return Err(ValidationError::new("product_requires_quantity_and_unit"));
    }
    if request
        .quantity
        .is_some_and(|quantity| quantity <= Decimal::ZERO)
    {
        return Err(ValidationError::new("quantity_must_be_positive"));
    }
    if request.target_unit_price.is_some() && request.target_currency.is_none() {
        return Err(ValidationError::new("target_price_requires_currency"));
    }
    Ok(())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RequirementSummaryResponse {
    pub id: RequirementId,
    pub requirement_number: String,
    pub kind: RequirementKind,
    pub title: String,
    pub status: RequirementStatus,
    pub destination_country: String,
    pub required_by: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}
