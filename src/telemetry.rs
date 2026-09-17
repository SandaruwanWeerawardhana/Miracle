//! Tracing subscriber setup. JSON output in deployed environments feeds log
//! aggregation; the pretty format is for local development.
//!
//! TODO: add an OpenTelemetry layer (tracing-opentelemetry + OTLP exporter) when
//! an observability backend is chosen. Request spans already carry `request_id`.

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{LogFormat, TelemetryConfig};

pub fn init(config: &TelemetryConfig) {
    let filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);

    let result = match config.format {
        LogFormat::Json => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .try_init(),
        LogFormat::Pretty => registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .try_init(),
    };

    // Tests may initialise telemetry more than once; that is not an error.
    let _ = result;
}
