use yav::prelude::*;
use axum::response::Response;

/// Production health check endpoint returning HTTP 200 with JSON status payload.
/// Compatible with container orchestrators (Kubernetes, Docker), uptime monitors, and cloud hosts.
pub async fn health() -> Response {
    json_response(
        StatusCode::OK,
        &HealthStatus {
            status: "ok",
            service: "yav",
            version: env!("CARGO_PKG_VERSION"),
        },
    )
}
