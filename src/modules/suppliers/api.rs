use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::{
    domain::{SupplierId, VerificationStatus},
    verification::{self, RecordVerificationDecision},
};
use crate::{
    app::AppState,
    shared::{
        auth::CurrentUser,
        error::{AppResult, ErrorResponse},
        response::DataResponse,
        validation::ValidatedJson,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/{supplier_id}/verification",
        post(record_verification_decision),
    )
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerificationDecisionRequest {
    /// One of `VERIFIED`, `REJECTED`, `SUSPENDED`.
    pub decision: VerificationStatus,
    #[validate(length(max = 2000))]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerificationDecisionResponse {
    pub supplier_id: SupplierId,
    pub previous_status: VerificationStatus,
    pub status: VerificationStatus,
}

#[utoipa::path(
    post,
    path = "/api/v1/suppliers/{supplier_id}/verification",
    tag = "suppliers",
    security(("bearer" = [])),
    params(("supplier_id" = SupplierId, Path)),
    request_body = VerificationDecisionRequest,
    responses(
        (status = 200, body = DataResponse<VerificationDecisionResponse>),
        (status = 403, description = "Missing `supplier.verify`", body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, description = "Transition not allowed", body = ErrorResponse),
    )
)]
pub async fn record_verification_decision(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(supplier_id): Path<SupplierId>,
    ValidatedJson(request): ValidatedJson<VerificationDecisionRequest>,
) -> AppResult<Json<DataResponse<VerificationDecisionResponse>>> {
    let outcome = verification::record_decision(
        &state,
        &user,
        RecordVerificationDecision {
            supplier_id,
            decision: request.decision,
            notes: request.notes,
        },
    )
    .await?;

    Ok(Json(DataResponse::new(VerificationDecisionResponse {
        supplier_id: outcome.supplier_id,
        previous_status: outcome.previous,
        status: outcome.current,
    })))
}
