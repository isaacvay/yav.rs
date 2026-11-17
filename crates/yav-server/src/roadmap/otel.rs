//! Bridge from the OpenTelemetry SDK to the Yav tracing layer.
//!
//! Compiled only with the `otel-sdk` Cargo feature. The existing zero-SDK
//! helpers in `crate::otel` (W3C `traceparent`, `global_tracer`, OTLP/HTTP JSON
//! export via reqwest) stay untouched; this module adds the real SDK exporter,
//! provider and W3C propagator installation.
#![cfg(feature = "otel-sdk")]

use opentelemetry_otlp::WithExportConfig;

/// Build an OTLP/HTTP span exporter with the SDK's typestate builder.
///
/// The endpoint must be a full OTLP/HTTP traces URL (for example
/// `http://localhost:4318/v1/traces`). Protocol/client features are fixed by
/// the workspace declaration (`http-proto`, `reqwest-client`,
/// `reqwest-rustls`, `trace`).
pub fn otlp_exporter(endpoint: &str) -> Result<opentelemetry_otlp::SpanExporter, String> {
    opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .map_err(|err| err.to_string())
}

/// Build a tracer provider that batches spans to `endpoint` and tags them with
/// `service.name = service_name`.
///
/// Note: in opentelemetry_sdk 0.33 the concrete provider type is named
/// `SdkTracerProvider` (there is no `TracerProvider` alias in `trace`).
pub fn tracer_provider(
    endpoint: &str,
    service_name: &str,
) -> Result<opentelemetry_sdk::trace::SdkTracerProvider, String> {
    let exporter = otlp_exporter(endpoint)?;
    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    Ok(opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build())
}

/// Install the W3C Trace Context propagator globally.
///
/// This makes `traceparent`/`tracestate` injection and extraction work for any
/// code that goes through `opentelemetry::global::get_text_map_propagator`.
///
/// # What is deliberately NOT done here
///
/// This function does not perform full auto-instrumentation. In particular it
/// does not:
///
/// 1. Register the provider with `opentelemetry::global::set_tracer_provider`
///    (call that yourself with a clone of [`tracer_provider`]'s result).
/// 2. Build a `tracing_opentelemetry::OpenTelemetryLayer` and attach it to a
///    `tracing_subscriber::Registry`: the `tracing-subscriber` crate is not a
///    dependency of this crate's `otel-sdk` feature, so a function returning
///    `impl tracing::Subscriber` cannot be provided here. In an application
///    that already depends on it, the remaining steps are:
///
///    ```text
///    let layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("yav"));
///    let subscriber = tracing_subscriber::registry().with(layer);
///    tracing::subscriber::set_global_default(subscriber)?;
///    ```
///
/// 3. Extract/inject context around Axum requests. The existing
///    `crate::otel::otel_tracing_middleware` parses `traceparent` into its own
///    `TraceParent` type; bridging that into SDK `SpanContext` values still
///    requires application-side code.
///
/// Call `provider.shutdown()` on application exit to flush the batch exporter.
pub fn install_propagator() {
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );
}
