//! Error types and error page rendering for the Yav web framework.
//!
//! Provides the core [`YavError`] enum, HTML error page rendering, and
//! integration with Axum's [`IntoResponse`].

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

/// Framework error type representing standard HTTP errors and internal server issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YavError {
    /// 404 Not Found error.
    NotFound,
    /// 500 Internal Server Error with details.
    Internal(String),
    /// 400 Bad Request error with explanation.
    BadRequest(String),
}

impl fmt::Display for YavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YavError::NotFound => write!(f, "404 Not Found"),
            YavError::Internal(msg) => write!(f, "500 Internal Server Error: {}", msg),
            YavError::BadRequest(msg) => write!(f, "400 Bad Request: {}", msg),
        }
    }
}

impl std::error::Error for YavError {}

impl From<std::io::Error> for YavError {
    fn from(err: std::io::Error) -> Self {
        YavError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for YavError {
    fn from(err: serde_json::Error) -> Self {
        tracing::warn!(error = %err, "Failed to deserialize JSON request payload");
        if crate::session::is_production_mode() {
            YavError::BadRequest("Invalid JSON request payload.".to_string())
        } else {
            YavError::BadRequest(err.to_string())
        }
    }
}

/// Renders a modern, responsive HTML error page wrapped in [`yav_core::document_shell`].
///
/// # Arguments
/// * `status` - The HTTP status code
/// * `title` - Human-readable error title
/// * `message` - Detailed error explanation
pub fn render_error_page(status: StatusCode, title: &str, message: &str) -> String {
    let code = status.as_u16();
    let escaped_title = yav_core::escape_html(title);
    let escaped_message = yav_core::escape_html(message);

    let head_extra = r#"<style>
* { box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    background-color: #f8fafc;
    color: #1e293b;
    margin: 0;
    padding: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
}
.error-card {
    background: #ffffff;
    border-radius: 16px;
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.05), 0 8px 10px -6px rgba(0, 0, 0, 0.05);
    padding: 3rem 2.5rem;
    max-width: 32rem;
    width: 100%;
    text-align: center;
    border: 1px solid #e2e8f0;
}
.error-badge {
    display: inline-block;
    font-size: 0.875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    background-color: #eff6ff;
    color: #2563eb;
    margin-bottom: 1rem;
}
.error-code {
    font-size: 4.5rem;
    font-weight: 900;
    color: #0f172a;
    line-height: 1;
    margin: 0 0 0.75rem 0;
}
.error-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: #334155;
    margin: 0 0 1rem 0;
}
.error-message {
    font-size: 1rem;
    color: #64748b;
    line-height: 1.6;
    margin: 0 0 2rem 0;
    word-break: break-word;
}
.error-action {
    display: inline-block;
    background-color: #2563eb;
    color: #ffffff;
    font-weight: 600;
    font-size: 0.95rem;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    text-decoration: none;
    transition: background-color 0.2s ease, transform 0.1s ease;
}
.error-action:hover {
    background-color: #1d4ed8;
    transform: translateY(-1px);
}
</style>"#;

    let body = format!(
        r#"<div class="error-card">
<div class="error-badge">HTTP Status</div>
<h1 class="error-code">{}</h1>
<h2 class="error-title">{}</h2>
<p class="error-message">{}</p>
<a href="/" class="error-action">Back to Home</a>
</div>"#,
        code, escaped_title, escaped_message
    );

    // `document_shell` already escapes `doc_title`, so pass raw `title` to avoid double escaping
    let doc_title = format!("{} — {}", code, title);
    yav_core::document_shell(&doc_title, head_extra, &body, "")
}

impl IntoResponse for YavError {
    fn into_response(self) -> Response {
        let (status, title, message) = match &self {
            YavError::NotFound => (
                StatusCode::NOT_FOUND,
                "Page Not Found",
                "The requested resource could not be found on this server.",
            ),
            YavError::Internal(msg) => {
                tracing::error!(%msg, "Internal Server Error");
                let display_msg = if cfg!(debug_assertions) && !crate::session::is_production_mode()
                {
                    msg.as_str()
                } else {
                    "An internal server error occurred. Please try again later."
                };
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    display_msg,
                )
            }
            YavError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg.as_str()),
        };

        let html = render_error_page(status, title, message);
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_found_renders_404_html() {
        let err = YavError::NotFound;
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "text/html; charset=utf-8");

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body read");
        let body_str = String::from_utf8(body_bytes.to_vec()).expect("utf8");

        assert!(body_str.contains("404"));
        assert!(body_str.contains("Page Not Found"));
        assert!(body_str.contains("<!DOCTYPE html>"));
    }

    #[tokio::test]
    async fn internal_error_renders_500_html() {
        let err = YavError::Internal("Database connection lost".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "text/html; charset=utf-8");

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body read");
        let body_str = String::from_utf8(body_bytes.to_vec()).expect("utf8");

        assert!(body_str.contains("500"));
        assert!(body_str.contains("Database connection lost"));
    }

    #[tokio::test]
    async fn bad_request_renders_400_html() {
        let err = YavError::BadRequest("Invalid query parameters".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body read");
        let body_str = String::from_utf8(body_bytes.to_vec()).expect("utf8");

        assert!(body_str.contains("400"));
        assert!(body_str.contains("Invalid query parameters"));
    }

    #[test]
    fn error_display_and_traits() {
        let not_found = YavError::NotFound;
        assert_eq!(format!("{}", not_found), "404 Not Found");

        let internal = YavError::Internal("timeout".into());
        assert_eq!(
            format!("{}", internal),
            "500 Internal Server Error: timeout"
        );

        let bad_req = YavError::BadRequest("missing field".into());
        assert_eq!(format!("{}", bad_req), "400 Bad Request: missing field");

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let from_io: YavError = io_err.into();
        assert!(matches!(from_io, YavError::Internal(_)));
    }
}
