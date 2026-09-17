//! Composition root: builds shared application state and assembles the HTTP router.

use std::sync::Arc;

use anyhow::Context;
use axum::{Router, extract::FromRef};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    config::AppConfig,
    database, middleware,
    modules::{self, auth::PgPrincipalResolver},
    openapi::ApiDoc,
    shared::auth::{PrincipalResolver, TokenService},
};

/// State shared by every request handler and background worker.
///
/// Cheap to clone: every field is reference counted or a pooled handle. Add a
/// field only for application-wide infrastructure; module-specific
/// collaborators belong inside the module.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub tokens: Arc<TokenService>,
    pub principals: Arc<dyn PrincipalResolver>,
}

impl AppState {
    /// Connects to every required backing service, failing fast when one is unreachable.
    pub async fn connect(config: AppConfig) -> anyhow::Result<Self> {
        let db = database::connect(&config.database)
            .await
            .context("failed to connect to PostgreSQL")?;

        let redis_client =
            redis::Client::open(config.redis.url.expose()).context("invalid REDIS_URL")?;
        let redis = ConnectionManager::new(redis_client)
            .await
            .context("failed to connect to Redis")?;

        Ok(Self::new(config, db, redis))
    }

    pub fn new(config: AppConfig, db: PgPool, redis: ConnectionManager) -> Self {
        let tokens = Arc::new(TokenService::new(&config.auth));
        let principals = Arc::new(PgPrincipalResolver::new(db.clone()));
        Self {
            config: Arc::new(config),
            db,
            redis,
            tokens,
            principals,
        }
    }
}

impl FromRef<AppState> for Arc<TokenService> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.tokens)
    }
}

impl FromRef<AppState> for Arc<dyn PrincipalResolver> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.principals)
    }
}

pub fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .merge(modules::health::routes())
        .nest("/api/v1", modules::api_v1_router());

    if state.config.server.api_docs_enabled {
        router = router.merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()));
    }

    middleware::apply(router, &state.config).with_state(state)
}
