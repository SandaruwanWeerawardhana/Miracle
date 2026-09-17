use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use super::dto::AcceptQuotationResponse;
use crate::{
    app::AppState,
    modules::quotations::{application::accept_quotation, domain::QuotationId},
    shared::{
        auth::CurrentUser,
        error::{AppResult, ErrorResponse},
        response::DataResponse,
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/quotations/{quotation_id}/accept",
    tag = "quotations",
    security(("bearer" = [])),
    params(("quotation_id" = QuotationId, Path)),
    responses(
        (status = 201, description = "Quotation accepted and order created", body = DataResponse<AcceptQuotationResponse>),
        (status = 404, body = ErrorResponse),
        (status = 409, description = "Concurrent modification or order already exists", body = ErrorResponse),
        (status = 422, description = "Expired or not in SENT status", body = ErrorResponse),
    )
)]
pub async fn accept_quotation(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(quotation_id): Path<QuotationId>,
) -> AppResult<(StatusCode, Json<DataResponse<AcceptQuotationResponse>>)> {
    let accepted = accept_quotation::execute(&state, &user, quotation_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(DataResponse::new(AcceptQuotationResponse {
            quotation_id: accepted.quotation_id,
            order_id: accepted.order_id,
        })),
    ))
}
