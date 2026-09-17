//! OpenAPI document. Served at `/openapi.json` with Swagger UI at `/docs` only
//! when `APP_API_DOCS_ENABLED=true` (default: development only).
//!
//! When adding an endpoint: annotate the handler with `#[utoipa::path]` and list
//! it under `paths(...)` below.

use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    modules::{auth, health, orders, quotations, requirements, suppliers, users},
    shared::error::{ErrorBody, ErrorResponse},
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Miracle International API",
        description = "Integrated Global Trade & Business Solutions Platform",
    ),
    paths(
        health::live,
        health::ready,
        auth::api::handlers::register,
        auth::api::handlers::login,
        auth::api::handlers::logout,
        users::api::get_me,
        suppliers::api::record_verification_decision,
        requirements::api::list_requirements,
        requirements::api::submit_requirement,
        quotations::api::handlers::accept_quotation,
        orders::api::handlers::update_status,
    ),
    components(schemas(ErrorResponse, ErrorBody)),
    modifiers(&BearerSecurity),
    tags(
        (name = "health", description = "Liveness and readiness probes"),
        (name = "auth", description = "Authentication and sessions"),
        (name = "users", description = "User accounts"),
        (name = "suppliers", description = "Suppliers and verification"),
        (name = "requirements", description = "Customer requirements"),
        (name = "quotations", description = "Customer quotations"),
        (name = "orders", description = "Wholesale orders and tracking"),
    )
)]
pub struct ApiDoc;

struct BearerSecurity;

impl Modify for BearerSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
