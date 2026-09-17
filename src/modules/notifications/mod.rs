//! Notification delivery (in-app, email, SMS; WhatsApp later).
//!
//! Business modules never call a provider directly. They enqueue a job inside
//! their transaction (`jobs::enqueue(Job::SendEmail { .. })`), and the worker
//! calls into this module.
//!
//! TODO:
//! * `notifications` table for in-app notifications + `GET /api/v1/notifications`.
//! * Real providers behind [`EmailSender`] (SMTP / SES) and an `SmsSender` trait,
//!   selected from configuration and stored in `AppState`.
//! * Template rendering (per language) keyed by `template`.

use async_trait::async_trait;
use serde_json::Value;

use crate::app::AppState;

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

#[async_trait]
pub trait EmailSender: Send + Sync + 'static {
    async fn send(&self, message: EmailMessage) -> anyhow::Result<()>;
}

/// Development sender: logs instead of delivering. Never logs the body, which may
/// contain verification or password-reset links.
pub struct LoggingEmailSender;

#[async_trait]
impl EmailSender for LoggingEmailSender {
    async fn send(&self, message: EmailMessage) -> anyhow::Result<()> {
        tracing::info!(to = %message.to, subject = %message.subject, "email (logging sender)");
        Ok(())
    }
}

pub async fn send_email(
    _state: &AppState,
    to: &str,
    template: &str,
    _variables: &Value,
) -> anyhow::Result<()> {
    // TODO: render `template` with `variables`; use the configured sender from state.
    LoggingEmailSender
        .send(EmailMessage {
            to: to.to_owned(),
            subject: template.to_owned(),
            html_body: String::new(),
            text_body: String::new(),
        })
        .await
}
