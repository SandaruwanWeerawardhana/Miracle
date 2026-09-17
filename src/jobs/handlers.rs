use super::Job;
use crate::{app::AppState, modules::notifications};

/// Dispatches a job to the module that owns the work. Keep logic out of this file.
pub async fn handle(state: &AppState, job: Job) -> anyhow::Result<()> {
    match job {
        Job::SendEmail {
            to,
            template,
            variables,
        } => notifications::send_email(state, &to, &template, &variables).await,
        Job::QuotationAccepted {
            quotation_id,
            order_id,
        } => {
            // TODO: notify customer and assigned sales staff; enqueue quotation PDF generation.
            tracing::info!(%quotation_id, %order_id, "quotation accepted job received");
            Ok(())
        }
    }
}
