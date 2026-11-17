//! OpenTelemetry (OTLP) Distributed Tracing & W3C `traceparent` Propagation.
//!
//! Provides W3C Trace Context Level 1 parity in pure Rust:
//! - Parses and generates W3C `traceparent` (`00-<32-hex-trace-id>-<16-hex-span-id>-<2-hex-flags>`)
//!   headers across microservice boundaries.
//! - [`OtelTracer`] records structured spans (`http.server`, `db.query`, `ai.completion`, `rpc.server`)
//!   with attributes, status codes, and microsecond durations.
//! - Exports OTLP/HTTP JSON payloads to real collectors via [`OtelTracer::export_otlp_http`]
//!   (compatible with Grafana Tempo, Jaeger, Honeycomb, and Datadog).
//! - Also supports Prometheus text exposition format via [`OtelTracer::export_prometheus`].

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_SPANS: usize = 2000;

/// W3C Trace Context (`traceparent` header: `00-<trace_id>-<span_id>-<trace_flags>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceParent {
    /// Version byte (always `0`).
    pub version: u8,
    /// 32-character lowercase hex trace ID (16 bytes).
    pub trace_id: String,
    /// 16-character lowercase hex parent/current span ID (8 bytes).
    pub span_id: String,
    /// Trace flags (`01` = sampled).
    pub flags: u8,
}

impl TraceParent {
    /// Generate a new root W3C `TraceParent` with a fresh 128-bit `trace_id` and 64-bit `span_id`.
    #[must_use]
    pub fn new_root() -> Self {
        let hex = random_hex_64();
        Self {
            version: 0,
            trace_id: hex[0..32].to_string(),
            span_id: hex[32..48].to_string(),
            flags: 1,
        }
    }

    /// Create a child span sharing the same `trace_id` with a fresh 64-bit `span_id`.
    #[must_use]
    pub fn new_child(&self) -> Self {
        let hex = random_hex_64();
        Self {
            version: self.version,
            trace_id: self.trace_id.clone(),
            span_id: hex[0..16].to_string(),
            flags: self.flags,
        }
    }

    /// Parse a W3C `traceparent` header string (e.g. `"00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"`).
    #[must_use]
    pub fn parse(header_val: &str) -> Option<Self> {
        let parts: Vec<&str> = header_val.trim().split('-').collect();
        if parts.len() != 4 {
            return None;
        }
        let version = u8::from_str_radix(parts[0], 16).ok()?;
        let trace_id = parts[1].to_ascii_lowercase();
        let span_id = parts[2].to_ascii_lowercase();
        let flags = u8::from_str_radix(parts[3], 16).ok()?;

        if trace_id.len() != 32
            || !trace_id.chars().all(|c| c.is_ascii_hexdigit())
            || trace_id.chars().all(|c| c == '0')
        {
            return None;
        }
        if span_id.len() != 16
            || !span_id.chars().all(|c| c.is_ascii_hexdigit())
            || span_id.chars().all(|c| c == '0')
        {
            return None;
        }

        Some(Self {
            version,
            trace_id,
            span_id,
            flags,
        })
    }

    /// Format as a standard W3C `traceparent` header string.
    #[must_use]
    pub fn to_header_value(&self) -> String {
        format!(
            "{:02x}-{}-{}-{:02x}",
            self.version, self.trace_id, self.span_id, self.flags
        )
    }
}

fn random_hex_64() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = Sha256::new();
    hasher.update(nanos.to_le_bytes());
    hasher.update(seq.to_le_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Recorded OpenTelemetry span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanRecord {
    /// 32-char hex trace ID.
    pub trace_id: String,
    /// 16-char hex span ID.
    pub span_id: String,
    /// Optional 16-char hex parent span ID.
    pub parent_span_id: Option<String>,
    /// Span operation name (e.g. `"GET /products/:id"`, `"db.select"`, `"ai.chat"`).
    pub name: String,
    /// Span kind (`"SERVER"`, `"CLIENT"`, `"INTERNAL"`).
    pub kind: String,
    /// Start timestamp in Unix nanoseconds.
    pub start_time_unix_nano: u64,
    /// Duration in microseconds.
    pub duration_us: u64,
    /// Status (`"OK"`, `"ERROR"`).
    pub status: String,
    /// Key-value span attributes.
    pub attributes: HashMap<String, String>,
}

/// OpenTelemetry tracer and exporter.
#[derive(Debug, Clone)]
pub struct OtelTracer {
    service_name: String,
    spans: Arc<Mutex<VecDeque<SpanRecord>>>,
    total_spans: Arc<std::sync::atomic::AtomicU64>,
    total_errors: Arc<std::sync::atomic::AtomicU64>,
    total_duration_us: Arc<std::sync::atomic::AtomicU64>,
    /// Instant at which this tracer was created, used for real (non-fabricated) uptime.
    created_at: Instant,
}

impl Default for OtelTracer {
    fn default() -> Self {
        Self::new("yav-app")
    }
}

impl OtelTracer {
    /// Create a new OpenTelemetry tracer for the given service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            spans: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SPANS))),
            total_spans: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_duration_us: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            created_at: Instant::now(),
        }
    }

    /// Total number of spans recorded since this tracer was initialized.
    ///
    /// [`otel_tracing_middleware`] records exactly one `SERVER` span per HTTP request,
    /// so this is also the real request count observed by the built-in dashboard.
    #[must_use]
    pub fn total_spans(&self) -> u64 {
        self.total_spans.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Total number of spans recorded with status `"ERROR"` (HTTP >= 500 in the middleware).
    #[must_use]
    pub fn total_errors(&self) -> u64 {
        self.total_errors.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Cumulative duration of all recorded spans, in microseconds.
    #[must_use]
    pub fn total_duration_us(&self) -> u64 {
        self.total_duration_us
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Seconds elapsed since this tracer was initialized (real uptime, not a constant).
    #[must_use]
    pub fn uptime_secs(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    /// Record a completed span.
    pub fn record_span(&self, span: SpanRecord) {
        self.total_spans
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if span.status == "ERROR" {
            self.total_errors
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        self.total_duration_us
            .fetch_add(span.duration_us, std::sync::atomic::Ordering::Relaxed);

        let mut lock = self.spans.lock().unwrap_or_else(|e| e.into_inner());
        if lock.len() >= MAX_SPANS {
            lock.pop_front();
        }
        lock.push_back(span);
    }

    /// Retrieve all buffered spans.
    #[must_use]
    pub fn recent_spans(&self) -> Vec<SpanRecord> {
        let lock = self.spans.lock().unwrap_or_else(|e| e.into_inner());
        lock.iter().cloned().collect()
    }

    /// Export buffered spans in standard OTLP/HTTP JSON format (`resourceSpans`).
    #[must_use]
    pub fn export_otlp_json(&self) -> serde_json::Value {
        let spans = self.recent_spans();
        let otlp_spans: Vec<serde_json::Value> = spans
            .iter()
            .map(|s| {
                let end_nano = s.start_time_unix_nano + s.duration_us * 1000;
                let attrs: Vec<serde_json::Value> = s
                    .attributes
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!({
                            "key": k,
                            "value": { "stringValue": v }
                        })
                    })
                    .collect();
                serde_json::json!({
                    "traceId": s.trace_id,
                    "spanId": s.span_id,
                    "parentSpanId": s.parent_span_id.clone().unwrap_or_default(),
                    "name": s.name,
                    "kind": s.kind,
                    "startTimeUnixNano": s.start_time_unix_nano.to_string(),
                    "endTimeUnixNano": end_nano.to_string(),
                    "attributes": attrs,
                    "status": { "code": if s.status == "OK" { 1 } else { 2 } }
                })
            })
            .collect();

        serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [{
                        "key": "service.name",
                        "value": { "stringValue": self.service_name }
                    }]
                },
                "scopeSpans": [{
                    "scope": { "name": "yav.otel", "version": env!("CARGO_PKG_VERSION") },
                    "spans": otlp_spans
                }]
            }]
        })
    }

    /// Export span metrics in Prometheus text exposition format.
    #[must_use]
    pub fn export_prometheus(&self) -> String {
        let total = self.total_spans.load(std::sync::atomic::Ordering::Relaxed);
        let errors = self.total_errors.load(std::sync::atomic::Ordering::Relaxed);
        let total_duration_us = self
            .total_duration_us
            .load(std::sync::atomic::Ordering::Relaxed);

        format!(
            "# HELP yav_otel_spans_total Total number of recorded OpenTelemetry spans\n\
             # TYPE yav_otel_spans_total counter\n\
             yav_otel_spans_total{{service=\"{svc}\"}} {total}\n\
             # HELP yav_otel_span_errors_total Total number of error spans\n\
             # TYPE yav_otel_span_errors_total counter\n\
             yav_otel_span_errors_total{{service=\"{svc}\"}} {errors}\n\
             # HELP yav_otel_span_duration_microseconds_sum Cumulative span duration in microseconds\n\
             # TYPE yav_otel_span_duration_microseconds_sum counter\n\
             yav_otel_span_duration_microseconds_sum{{service=\"{svc}\"}} {total_duration_us}\n",
            svc = self.service_name
        )
    }

    /// Clear all buffered spans.
    pub fn clear(&self) {
        let mut lock = self.spans.lock().unwrap_or_else(|e| e.into_inner());
        lock.clear();
    }

    /// Export buffered spans to an OTLP/HTTP collector endpoint via real HTTP POST.
    ///
    /// Sends the OTLP JSON payload (as generated by [`export_otlp_json`]) to a
    /// collector such as Grafana Tempo, Jaeger, Honeycomb, or Datadog using
    /// the standard OTLP/HTTP protocol.
    ///
    /// # Arguments
    /// * `collector_url` - Full URL of the OTLP/HTTP endpoint (e.g. `https://collector.example.com/v1/traces`).
    /// * `timeout_secs` - Request timeout in seconds.
    ///
    /// # Errors
    /// Returns `OtelExportError::HttpError` if the POST fails or the collector
    /// returns a non-2xx status code.
    pub async fn export_otlp_http(
        &self,
        collector_url: &str,
        timeout_secs: u64,
    ) -> Result<(), OtelExportError> {
        let payload = serde_json::to_vec(&self.export_otlp_json())
            .map_err(|e| OtelExportError::SerializationError(e.to_string()))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| OtelExportError::HttpError(e.to_string()))?;

        let resp = client
            .post(collector_url)
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await
            .map_err(|e| OtelExportError::HttpError(e.to_string()))?;

        if resp.status().is_success() {
            // Clear buffered spans after successful export
            self.clear();
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(OtelExportError::HttpError(format!(
                "Collector returned HTTP {status}: {body}"
            )))
        }
    }
}

/// Error returned when OTLP export fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtelExportError {
    /// Failed to serialize the OTLP JSON payload.
    SerializationError(String),
    /// The HTTP POST to the collector failed or returned a non-2xx status.
    HttpError(String),
}

impl std::fmt::Display for OtelExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "OTLP serialization error: {msg}"),
            Self::HttpError(msg) => write!(f, "OTLP HTTP export error: {msg}"),
        }
    }
}

impl std::error::Error for OtelExportError {}

/// Global singleton OpenTelemetry tracer.
pub fn global_tracer() -> &'static OtelTracer {
    static TRACER: OnceLock<OtelTracer> = OnceLock::new();
    TRACER.get_or_init(|| OtelTracer::new("yav-server"))
}

/// Axum middleware that extracts or generates W3C `traceparent` headers,
/// attaches [`TraceParent`] to request extensions, echoes `traceparent` on the response,
/// and records an HTTP server [`SpanRecord`] in [`global_tracer()`].
pub async fn otel_tracing_middleware(mut req: Request, next: Next) -> Response {
    let incoming_tp = req
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(TraceParent::parse);

    let (trace_parent, parent_span_id) = match incoming_tp {
        Some(parent) => {
            let p_id = parent.span_id.clone();
            (parent.new_child(), Some(p_id))
        }
        None => (TraceParent::new_root(), None),
    };

    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let start_unix_nano = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let start_instant = Instant::now();

    req.extensions_mut().insert(trace_parent.clone());

    let mut resp = next.run(req).await;
    let duration_us = start_instant.elapsed().as_micros() as u64;
    let status_code = resp.status().as_u16();

    if let Ok(hv) = HeaderValue::from_str(&trace_parent.to_header_value()) {
        resp.headers_mut()
            .insert(HeaderName::from_static("traceparent"), hv);
    }

    let mut attrs = HashMap::new();
    attrs.insert("http.method".to_string(), method.clone());
    attrs.insert("http.target".to_string(), path.clone());
    attrs.insert("http.status_code".to_string(), status_code.to_string());

    global_tracer().record_span(SpanRecord {
        trace_id: trace_parent.trace_id,
        span_id: trace_parent.span_id,
        parent_span_id,
        name: format!("{method} {path}"),
        kind: "SERVER".to_string(),
        start_time_unix_nano: start_unix_nano,
        duration_us,
        status: if status_code >= 500 {
            "ERROR".to_string()
        } else {
            "OK".to_string()
        },
        attributes: attrs,
    });

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_w3c_traceparent_parse_and_child_propagation() {
        let raw = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let tp = TraceParent::parse(raw).expect("valid traceparent");
        assert_eq!(tp.version, 0);
        assert_eq!(tp.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(tp.span_id, "00f067aa0ba902b7");
        assert_eq!(tp.flags, 1);
        assert_eq!(tp.to_header_value(), raw);

        let child = tp.new_child();
        assert_eq!(child.trace_id, tp.trace_id);
        assert_ne!(child.span_id, tp.span_id);
        assert_eq!(child.span_id.len(), 16);
    }

    #[test]
    fn test_otlp_json_and_prometheus_export() {
        let tracer = OtelTracer::new("billing-service");
        let tp = TraceParent::new_root();
        let mut attrs = HashMap::new();
        attrs.insert("db.system".to_string(), "postgresql".to_string());

        tracer.record_span(SpanRecord {
            trace_id: tp.trace_id,
            span_id: tp.span_id,
            parent_span_id: None,
            name: "SELECT * FROM invoices".to_string(),
            kind: "CLIENT".to_string(),
            start_time_unix_nano: 1_700_000_000_000_000_000,
            duration_us: 450,
            status: "OK".to_string(),
            attributes: attrs,
        });

        let otlp = tracer.export_otlp_json();
        let otlp_str = otlp.to_string();
        assert!(otlp_str.contains("billing-service"));
        assert!(otlp_str.contains("SELECT * FROM invoices"));

        let prom = tracer.export_prometheus();
        assert!(prom.contains("yav_otel_spans_total{service=\"billing-service\"} 1"));
        assert!(prom
            .contains("yav_otel_span_duration_microseconds_sum{service=\"billing-service\"} 450"));
    }

    #[tokio::test]
    async fn test_otlp_http_export_returns_error_on_unreachable_collector() {
        let tracer = OtelTracer::new("test-service");
        let tp = TraceParent::new_root();
        tracer.record_span(SpanRecord {
            trace_id: tp.trace_id,
            span_id: tp.span_id,
            parent_span_id: None,
            name: "test.span".to_string(),
            kind: "INTERNAL".to_string(),
            start_time_unix_nano: 1_700_000_000_000_000_000,
            duration_us: 100,
            status: "OK".to_string(),
            attributes: HashMap::new(),
        });

        // Try exporting to an unreachable endpoint (port 1 → connection refused)
        let result = tracer
            .export_otlp_http("http://127.0.0.1:1/v1/traces", 2)
            .await;
        assert!(result.is_err(), "should fail on unreachable collector");
        match result {
            Err(OtelExportError::HttpError(_)) => {}
            other => panic!("expected HttpError, got {other:?}"),
        }

        // Spans should NOT be cleared on failure
        assert_eq!(tracer.recent_spans().len(), 1);
    }
}
