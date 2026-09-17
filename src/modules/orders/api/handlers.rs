use axum::{
    Json,
    extract::{Path, State},
};

use super::dto::{OrderStatusResponse, UpdateOrderStatusRequest};
use crate::{
    app::AppState,
    modules::orders::{
        application::update_status::{self, UpdateOrderStatus},
        domain::OrderId,
    },
    shared::{
        auth::CurrentUser,
        error::{AppResult, ErrorResponse},
        response::DataResponse,
        validation::ValidatedJson,
    },
};

#[utoipa::path(
    patch,
    path = "/api/v1/orders/{order_id}/status",
    tag = "orders",
    security(("bearer" = [])),
    params(("order_id" = OrderId, Path)),
    request_body = UpdateOrderStatusRequest,
    responses(
        (status = 200, body = DataResponse<OrderStatusResponse>),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, description = "Transition not allowed", body = ErrorResponse),
    )
)]
pub async fn update_status(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(order_id): Path<OrderId>,
    ValidatedJson(request): ValidatedJson<UpdateOrderStatusRequest>,
) -> AppResult<Json<DataResponse<OrderStatusResponse>>> {
    let changed = update_status::execute(
        &state,
        &user,
        UpdateOrderStatus {
            order_id,
            status: request.status,
            note: request.note,
            is_public: request.is_public,
        },
    )
    .await?;

    Ok(Json(DataResponse::new(OrderStatusResponse {
        order_id: changed.order_id,
        previous_status: changed.from,
        status: changed.to,
    })))
}
