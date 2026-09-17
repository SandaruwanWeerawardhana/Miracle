use axum::{Json, Router, extract::State, http::StatusCode, routing::get};

use super::{
    dto::{ListRequirementsQuery, RequirementSummaryResponse, SubmitRequirementRequest},
    repository,
};
use crate::{
    app::AppState,
    modules::customers,
    shared::{
        auth::{AccessScope, CurrentUser, Permission},
        error::{AppError, AppResult, ErrorResponse},
        response::ListResponse,
        validation::{ValidatedJson, ValidatedQuery},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list_requirements).post(submit_requirement))
}

#[utoipa::path(
    get,
    path = "/api/v1/requirements",
    tag = "requirements",
    security(("bearer" = [])),
    params(ListRequirementsQuery),
    responses(
        (status = 200, body = ListResponse<RequirementSummaryResponse>),
        (status = 403, body = ErrorResponse),
    )
)]
pub async fn list_requirements(
    State(state): State<AppState>,
    user: CurrentUser,
    ValidatedQuery(query): ValidatedQuery<ListRequirementsQuery>,
) -> AppResult<Json<ListResponse<RequirementSummaryResponse>>> {
    let customer = match user.scope(Permission::RequirementRead, Permission::RequirementReadOwn)? {
        AccessScope::All => None,
        AccessScope::Own(user_id) => Some(
            customers::find_id_by_user(&state.db, user_id)
                .await?
                .ok_or(AppError::Forbidden)?,
        ),
    };

    let page = query.page();
    let (items, total) = repository::list(&state.db, customer, &query, page).await?;

    Ok(Json(ListResponse::new(items, page.meta(total))))
}

#[utoipa::path(
    post,
    path = "/api/v1/requirements",
    tag = "requirements",
    security(("bearer" = [])),
    request_body = SubmitRequirementRequest,
    responses(
        (status = 201, description = "Requirement submitted"),
        (status = 422, body = ErrorResponse),
    )
)]
pub async fn submit_requirement(
    State(_state): State<AppState>,
    user: CurrentUser,
    ValidatedJson(_request): ValidatedJson<SubmitRequirementRequest>,
) -> AppResult<StatusCode> {
    user.require(Permission::RequirementCreate)?;

    // TODO (SubmitRequirement use case, one transaction):
    // 1. Resolve the caller's customer profile (`customers::find_id_by_user`).
    // 2. Insert with `requirement_number` from `requirement_number_seq` (REQ-YYYY-000001).
    // 3. Audit `requirement.submitted`; enqueue notification to sales staff.
    // 4. Return 201 with `DataResponse<RequirementResponse>` and a Location header.
    Err(AppError::NotImplemented)
}
