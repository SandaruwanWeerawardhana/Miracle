//! Success response envelopes.
//!
//! Single resource: `{ "data": { ... } }`
//! Collection:      `{ "data": [ ... ], "pagination": { ... } }`

use serde::Serialize;
use utoipa::ToSchema;

use super::pagination::PageMeta;

#[derive(Debug, Serialize, ToSchema)]
pub struct DataResponse<T> {
    pub data: T,
}

impl<T> DataResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
    pub pagination: PageMeta,
}

impl<T> ListResponse<T> {
    pub fn new(data: Vec<T>, pagination: PageMeta) -> Self {
        Self { data, pagination }
    }
}
