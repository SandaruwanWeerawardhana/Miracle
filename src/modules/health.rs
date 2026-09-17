//! Liveness and readiness probes (unversioned, outside `/api/v1`).
//!
//! * `/health/live`  — process is up. Never touches dependencies, so a database
//!   outage does not make the orchestrator restart healthy pods.
//! * `/health/ready` — dependencies reachable; failing removes the instance
//!   from the load balancer.

use std::time::Duration;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{app::AppState, database};

const DEPENDENCY_TIMEOUT: Duration = Duration::from_secs(2);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(live))
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LivenessResponse {
    #[schema(example = "ok")]
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: CheckStatus,
    pub database: CheckStatus,
    pub redis: CheckStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Unavailable,
}

#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    responses((status = 200, description = "Process is running", body = LivenessResponse))
)]
pub async fn live() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "All dependencies reachable", body = ReadinessResponse),
        (status = 503, description = "A dependency is unavailable", body = ReadinessResponse),
    )
)]
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let (database, redis) = tokio::join!(check_database(&state), check_redis(&state));

    let status = if database == CheckStatus::Ok && redis == CheckStatus::Ok {
        CheckStatus::Ok
    } else {
        CheckStatus::Unavailable
    };
    let code = match status {
        CheckStatus::Ok => StatusCode::OK,
        CheckStatus::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };

    (
        code,
        Json(ReadinessResponse {
            status,
            database,
            redis,
        }),
    )
}

async fn check_database(state: &AppState) -> CheckStatus {
    match tokio::time::timeout(DEPENDENCY_TIMEOUT, database::ping(&state.db)).await {
        Ok(Ok(())) => CheckStatus::Ok,
        Ok(Err(error)) => {
            tracing::warn!(%error, "readiness: database check failed");
            CheckStatus::Unavailable
        }
        Err(_) => {
            tracing::warn!("readiness: database check timed out");
            CheckStatus::Unavailable
        }
    }
}

async fn check_redis(state: &AppState) -> CheckStatus {
    let mut connection = state.redis.clone();
    let command = redis::cmd("PING");
    let ping = command.query_async::<String>(&mut connection);

    match tokio::time::timeout(DEPENDENCY_TIMEOUT, ping).await {
        Ok(Ok(_)) => CheckStatus::Ok,
        Ok(Err(error)) => {
            tracing::warn!(%error, "readiness: redis check failed");
            CheckStatus::Unavailable
        }
        Err(_) => {
            tracing::warn!("readiness: redis check timed out");
            CheckStatus::Unavailable
        }
    }
}
