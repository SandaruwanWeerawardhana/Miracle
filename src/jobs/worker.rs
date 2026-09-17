use chrono::Utc;
use sqlx::FromRow;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;

use super::{Job, handlers};
use crate::app::AppState;

/// Jobs stuck in RUNNING longer than this (worker crashed) are made available again.
const STALE_LOCK_AFTER_SECONDS: f64 = 15.0 * 60.0;
const MAX_BACKOFF_SECONDS: i64 = 60 * 60;

#[derive(FromRow)]
struct ClaimedJob {
    id: Uuid,
    payload: serde_json::Value,
    attempts: i32,
    max_attempts: i32,
}

/// Polls for due jobs until `shutdown` is cancelled. The job currently being
/// processed is allowed to finish; nothing new is claimed after cancellation.
pub async fn run(state: AppState, shutdown: CancellationToken) {
    tracing::info!("job worker started");
    let poll_interval = state.config.jobs.poll_interval;

    while !shutdown.is_cancelled() {
        let processed = match process_batch(&state).await {
            Ok(count) => count,
            Err(error) => {
                tracing::error!(%error, "job worker failed to claim jobs");
                0
            }
        };

        if processed == 0 {
            tokio::select! {
                () = shutdown.cancelled() => break,
                () = tokio::time::sleep(poll_interval) => {}
            }
        }
    }

    tracing::info!("job worker stopped");
}

async fn process_batch(state: &AppState) -> Result<usize, sqlx::Error> {
    release_stale_locks(state).await?;

    let jobs = sqlx::query_as::<_, ClaimedJob>(
        r#"
        UPDATE background_jobs
        SET status = 'RUNNING', locked_at = now(), attempts = attempts + 1
        WHERE id IN (
            SELECT id FROM background_jobs
            WHERE status = 'PENDING' AND run_at <= now()
            ORDER BY run_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, payload, attempts, max_attempts
        "#,
    )
    .bind(state.config.jobs.batch_size)
    .fetch_all(&state.db)
    .await?;

    let count = jobs.len();
    for job in jobs {
        let span = tracing::info_span!("job", job_id = %job.id, attempt = job.attempts);
        execute(state, job).instrument(span).await?;
    }
    Ok(count)
}

async fn execute(state: &AppState, claimed: ClaimedJob) -> Result<(), sqlx::Error> {
    let outcome = match serde_json::from_value::<Job>(claimed.payload) {
        Ok(job) => handlers::handle(state, job).await,
        Err(error) => Err(anyhow::anyhow!("undecodable job payload: {error}")),
    };

    match outcome {
        Ok(()) => {
            sqlx::query(
                "UPDATE background_jobs SET status = 'SUCCEEDED', completed_at = now(), locked_at = NULL WHERE id = $1",
            )
            .bind(claimed.id)
            .execute(&state.db)
            .await?;
        }
        Err(error) if claimed.attempts >= claimed.max_attempts => {
            tracing::error!(error = %error, "job failed permanently");
            sqlx::query(
                "UPDATE background_jobs SET status = 'DEAD', last_error = $2, locked_at = NULL WHERE id = $1",
            )
            .bind(claimed.id)
            .bind(error.to_string())
            .execute(&state.db)
            .await?;
        }
        Err(error) => {
            let backoff = backoff_seconds(claimed.attempts);
            tracing::warn!(error = %error, retry_in_seconds = backoff, "job failed, will retry");
            sqlx::query(
                r#"
                UPDATE background_jobs
                SET status = 'PENDING', last_error = $2, locked_at = NULL, run_at = $3
                WHERE id = $1
                "#,
            )
            .bind(claimed.id)
            .bind(error.to_string())
            .bind(Utc::now() + chrono::Duration::seconds(backoff))
            .execute(&state.db)
            .await?;
        }
    }
    Ok(())
}

async fn release_stale_locks(state: &AppState) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE background_jobs
        SET status = 'PENDING', locked_at = NULL
        WHERE status = 'RUNNING' AND locked_at < now() - make_interval(secs => $1)
        "#,
    )
    .bind(STALE_LOCK_AFTER_SECONDS)
    .execute(&state.db)
    .await?;
    Ok(())
}

/// Exponential backoff: 30s, 60s, 120s, ... capped at one hour.
fn backoff_seconds(attempt: i32) -> i64 {
    let exponent = u32::try_from(attempt.saturating_sub(1))
        .unwrap_or(0)
        .min(16);
    (30_i64.saturating_mul(2_i64.saturating_pow(exponent))).min(MAX_BACKOFF_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_seconds(1), 30);
        assert_eq!(backoff_seconds(2), 60);
        assert_eq!(backoff_seconds(50), MAX_BACKOFF_SECONDS);
    }
}
