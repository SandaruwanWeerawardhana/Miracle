//! Payments, payment schedules and payment gateway integration.
//!
//! * `domain.rs`   — payment types/statuses and errors
//! * `gateway.rs`  — [`PaymentGateway`] provider abstraction
//! * `webhooks.rs` — idempotent recording of inbound gateway events
//!
//! No HTTP routes yet. TODO:
//! * `POST /payments` (record offline payment, requires `Idempotency-Key`),
//! * `POST /payments/{id}/confirm` (`payment.confirm`), refunds (`payment.refund`),
//! * `POST /payments/webhooks/{provider}` once a gateway is selected, with a
//!   `GatewayRegistry` in `AppState`.

mod domain;
mod gateway;
mod webhooks;

pub use domain::{PaymentError, PaymentId, PaymentStatus, PaymentType, ScheduleStatus};
pub use gateway::{CheckoutSession, GatewayEvent, PaymentGateway, PaymentIntent};
pub use webhooks::{RecordedEvent, record_gateway_event};
