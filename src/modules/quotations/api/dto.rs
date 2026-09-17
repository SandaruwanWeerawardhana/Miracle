use serde::Serialize;
use utoipa::ToSchema;

use crate::modules::{orders::OrderId, quotations::domain::QuotationId};

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptQuotationResponse {
    pub quotation_id: QuotationId,
    pub order_id: OrderId,
}
