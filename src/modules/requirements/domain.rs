use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

crate::typed_id!(
    /// Identifier of a row in `requirements`.
    RequirementId
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementKind {
    Product,
    General,
}

impl RequirementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Product => "PRODUCT",
            Self::General => "GENERAL",
        }
    }
}

impl FromStr for RequirementKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PRODUCT" => Ok(Self::Product),
            "GENERAL" => Ok(Self::General),
            other => Err(format!("unknown requirement kind `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementStatus {
    Submitted,
    UnderReview,
    Sourcing,
    Quoted,
    Closed,
    Cancelled,
}

impl RequirementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "SUBMITTED",
            Self::UnderReview => "UNDER_REVIEW",
            Self::Sourcing => "SOURCING",
            Self::Quoted => "QUOTED",
            Self::Closed => "CLOSED",
            Self::Cancelled => "CANCELLED",
        }
    }
}

impl FromStr for RequirementStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "SUBMITTED" => Ok(Self::Submitted),
            "UNDER_REVIEW" => Ok(Self::UnderReview),
            "SOURCING" => Ok(Self::Sourcing),
            "QUOTED" => Ok(Self::Quoted),
            "CLOSED" => Ok(Self::Closed),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(format!("unknown requirement status `{other}`")),
        }
    }
}
