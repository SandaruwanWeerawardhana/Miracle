//! Provider abstraction. Business logic depends on this trait only; concrete
//! implementations (e.g. a local Sri Lankan gateway, an international card
//! processor) live in their own files and are selected by configuration.

use std::time::Duration;

use async_trait::async_trait;
use axum::http::HeaderMap;

use crate::shared::money::Money;

pub struct PaymentIntent {
    /// Our payment ID, echoed back by the provider in webhooks.
    pub reference: String,
    pub amount: Money,
    pub description: String,
    pub customer_email: String,
    pub return_url: String,
}

pub struct CheckoutSession {
    pub provider_reference: String,
    pub redirect_url: String,
    pub expires_in: Duration,
}

/// A verified, provider-neutral webhook event.
pub struct GatewayEvent {
    pub provider_event_id: String,
    pub event_type: String,
    /// Our payment reference, when the event concerns a payment.
    pub payment_reference: Option<String>,
    pub provider_reference: Option<String>,
    pub amount: Option<Money>,
    pub succeeded: bool,
    pub raw_payload: serde_json::Value,
}

#[async_trait]
pub trait PaymentGateway: Send + Sync + 'static {
    /// Stable identifier stored in `payments.provider`, e.g. `"payhere"`.
    fn provider(&self) -> &'static str;

    async fn create_checkout(&self, intent: PaymentIntent) -> anyhow::Result<CheckoutSession>;

    /// Verifies the signature over the RAW body before anything is parsed or stored.
    fn verify_webhook(
        &self,
        headers: &HeaderMap,
        raw_body: &[u8],
    ) -> Result<GatewayEvent, super::PaymentError>;

    async fn refund(&self, provider_reference: &str, amount: Money) -> anyhow::Result<()>;
}
