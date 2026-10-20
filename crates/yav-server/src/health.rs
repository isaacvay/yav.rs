//! Production health checks and lightweight server telemetry.
//!
//! Provides Kubernetes-compatible `/healthz` (liveness), `/readyz` (readiness),
//! and in-memory SSR metrics for production observability.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};

/// Global in-memory metrics collector for the server.
#[derive(Debug, Clone)]
pub struct ServerMetrics {
    start_time: Instant,
    total_requests: Arc<AtomicU64>,
    total_errors: Arc<AtomicU64>,
    total_render_ms: Arc<AtomicU64>,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerMetrics {
    /// Create a new server metrics tracker starting at the current instant.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: Arc::new(AtomicU64::new(0)),
            total_errors: Arc::new(AtomicU64::new(0)),
            total_render_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment total request count.
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error count.
    pub fn record_error(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record SSR render duration in milliseconds.
    pub fn record_render(&self, duration_ms: u64) {
        self.total_render_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Return a point-in-time metrics snapshot.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let reqs = self.total_requests.load(Ordering::Relaxed);
        let errs = self.total_errors.load(Ordering::Relaxed);
        let ms = self.total_render_ms.load(Ordering::Relaxed);
        let avg_render_ms = if reqs > 0 {
            (ms as f64) / (reqs as f64)
        } else {
            0.0
        };

        MetricsSnapshot {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            total_requests: reqs,
            total_errors: errs,
            avg_render_duration_ms: (avg_render_ms * 100.0).round() / 100.0,
        }
    }
}

/// Point-in-time metrics report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_render_duration_ms: f64,
}

/// Basic liveness status report for `/healthz`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Default liveness check handler returning `{"status": "ok"}` with HTTP 200.
pub async fn healthz() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthStatus {
            status: "ok",
            service: "yav",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

/// Default readiness check handler returning HTTP 200 OK.
pub async fn readyz() -> impl IntoResponse {
    (StatusCode::OK, "ready")
}

/// Metrics handler returning a JSON snapshot of the server metrics.
pub async fn metrics_handler(metrics: axum::extract::Extension<ServerMetrics>) -> Response {
    Json(metrics.snapshot()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = ServerMetrics::new();
        metrics.record_request();
        metrics.record_request();
        metrics.record_error();
        metrics.record_render(20);
        metrics.record_render(30);

        let snap = metrics.snapshot();
        assert_eq!(snap.total_requests, 2);
        assert_eq!(snap.total_errors, 1);
        assert_eq!(snap.avg_render_duration_ms, 25.0);
    }
}
