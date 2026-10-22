//! # YAV Server
//!
//! HTTP server, server-side rendering (SSR) engine, and middleware for the
//! Yav web framework.
//!
//! Built on top of [Axum](https://docs.rs/axum), [Tokio](https://tokio.rs),
//! and [Tower](https://docs.rs/tower).
//!
//! ## Quick Start
//!
//! ```no_run
//! use yav_server::{YavApp, html_response, StatusCode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = YavApp::new()
//!         .port(3000)
//!         .static_dir("public")
//!         .get("/", |_req| async {
//!             html_response(StatusCode::OK, "<h1>Welcome to Yav</h1>")
//!         });
//!
//!     app.serve().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//! - [`app`] — [`YavApp`] builder and server configuration
//! - [`error`] — [`YavError`] and HTML error page rendering
//! - [`handler`] — Request handler helpers and [`HtmlPage`]
//! - [`middleware`] — Security headers and request logging
//! - [`static_files`] — Static asset resolution and serving

pub mod action;
pub mod app;
pub mod error;
pub mod flash;
pub mod forms;
pub mod handler;
pub mod health;
pub mod middleware;
pub mod openapi;
pub mod rate_limit;
pub mod seo;
pub mod session;
pub mod sse;
pub mod static_files;
pub mod streaming;
pub mod suspense;

// Route Context, Loaders & Actions
pub use action::{ActionValidatorFn, BoxLoaderFn, FormAction, Loader, RouteContext};

// Core YavApp types
pub use app::{AppRoute, BoxedHandler, IntoBoxedHandler, YavApp};

// Error types
pub use error::{render_error_page, YavError};

// Flash messages
pub use flash::{Flash, FlashLevel, FlashMessage, FLASH_COOKIE_NAME};

// Form validation & server actions
pub use forms::{
    csrf_field, verify_csrf_token, ActionResponse, ActionResult, CsrfToken, FormData, FormErrors,
    UploadedFile, CSRF_FIELD_NAME, CSRF_HEADER_NAME,
};

// Response and handler helpers
pub use handler::{
    html_response, is_local_redirect_path, is_safe_redirect_target, json_response, not_found,
    redirect, robots_response, safe_redirect, sitemap_response, HtmlFragment, HtmlPage,
    HtmxRequest,
};

// Health checks and server telemetry
pub use health::{healthz, metrics_handler, readyz, HealthStatus, MetricsSnapshot, ServerMetrics};

// Middleware
pub use middleware::{
    apply_default_security_headers, compute_etag, etag_middleware, request_logger_middleware,
    security_headers_middleware, SecurityHeadersConfig,
};

// OpenAPI & API documentation
pub use openapi::{scalar_ui_handler, swagger_ui_handler, ApiEndpoint, OpenApiSpec};

// Rate limiting & security
pub use rate_limit::{RateLimitConfig, RateLimitStatus, RateLimiter};

// SEO & search engine compliance
pub use seo::{RobotsTxt, Sitemap, SitemapEntry};

// Cryptographically signed sessions
pub use session::{
    compute_hmac_sha256, extract_session_from_cookie, session_middleware, SameSite, Session,
    SessionConfig, SessionSecret, SESSION_COOKIE_NAME,
};

// Server-Sent Events (SSE)
pub use sse::{sse_channel, sse_response, SseMsg, SseStream};

// Static file utilities
pub use static_files::{
    mime_type_for_path, parse_range_header, resolve_static_path, serve_static_file,
    serve_static_file_with_range, static_router,
};

// Streaming SSR & Suspense
pub use streaming::{streaming_html, StreamingHtml};
pub use suspense::Suspense;

// Axum re-exports for framework convenience
pub use axum::{
    extract::{Path, Query, Request},
    http::{Method, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, patch, post, put, Router},
};

/// Convenience prelude module bringing common server types into scope.
pub mod prelude {
    pub use crate::action::{ActionValidatorFn, BoxLoaderFn, FormAction, Loader, RouteContext};
    pub use crate::app::{AppRoute, BoxedHandler, YavApp};
    pub use crate::error::YavError;
    pub use crate::flash::{Flash, FlashLevel, FlashMessage};
    pub use crate::forms::{
        ActionResponse, ActionResult, CsrfToken, FormData, FormErrors, UploadedFile,
    };
    pub use crate::handler::{
        html_response, json_response, not_found, redirect, safe_redirect, HtmlFragment, HtmlPage,
        HtmxRequest,
    };
    pub use crate::health::{healthz, readyz, ServerMetrics};
    pub use crate::middleware::SecurityHeadersConfig;
    pub use crate::openapi::{scalar_ui_handler, swagger_ui_handler, OpenApiSpec};
    pub use crate::rate_limit::{RateLimitConfig, RateLimiter};
    pub use crate::seo::{RobotsTxt, Sitemap, SitemapEntry};
    pub use crate::session::{SameSite, Session, SessionConfig, SessionSecret};
    pub use crate::sse::{sse_channel, sse_response, SseMsg};
    pub use crate::static_files::{mime_type_for_path, resolve_static_path, serve_static_file};
    pub use crate::streaming::{streaming_html, StreamingHtml};
    pub use crate::suspense::Suspense;

    pub use axum::{
        extract::{Path, Query, Request},
        http::{Method, StatusCode},
        response::{Html, IntoResponse, Redirect, Response},
        routing::{delete, get, patch, post, put, Router},
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_exports() {
        use prelude::*;

        let app = YavApp::new().port(4000);
        assert_eq!(app.port, 4000);

        let resp = html_response(StatusCode::OK, "<p>Hi</p>");
        assert_eq!(resp.status(), StatusCode::OK);

        let err = YavError::NotFound;
        assert_eq!(err.to_string(), "404 Not Found");
    }
}
