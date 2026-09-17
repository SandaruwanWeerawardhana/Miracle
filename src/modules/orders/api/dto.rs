use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::orders::domain::{OrderId, OrderStatus};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateOrderStatusRequest {
    pub status: OrderStatus,
    /// Required when cancelling.
    #[validate(length(max = 1000))]
    pub note: Option<String>,
    /// Show this change on the customer's tracking timeline (default `true`).
    #[serde(default = "default_true")]
    pub is_public: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderStatusResponse {
    pub order_id: OrderId,
    pub previous_status: OrderStatus,
    pub status: OrderStatus,
}
