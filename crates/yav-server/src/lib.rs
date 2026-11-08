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
//! - [`app`] — [`YavApp`] builder, routing, static export (`export_static`), state, and server configuration
//! - [`cache`] — [`SsrCache`] and route-level caching (`get_cached`, `route_cached`)
//! - [`forms`] — [`FormData`], [`FormErrors`], [`FromFormData`] typed validation, and [`CsrfToken`] with CSPRNG salting
//! - [`ws`] — [`WsBroadcast`] real-time HTML fragment swapping over WebSockets
//! - [`session`] — [`Session`] cryptographically signed HMAC-SHA256 sessions
//! - [`flash`] — [`Flash`] one-time signed notification cookies
//! - [`rate_limit`] — [`RateLimiter`] sliding-window DDoS protection
//! - [`handler`] — Request handler helpers, [`HtmlPage`], and [`HtmlFragment`]
//! - [`middleware`] — Security headers, CSRF, and request logging
//! - [`openapi`] — OpenAPI 3.1 specification generation, Scalar and Swagger UI endpoints
//! - [`static_files`] — Static asset resolution, pre-compressed Brotli/Gzip serving, and cache headers
//! - [`streaming`] — Streaming HTML and async chunk transfer
//! - [`error`] — [`YavError`] and HTML error page rendering

pub mod action;
pub mod adapters;
pub mod ai;
pub mod app;
pub mod auth;
pub mod barcode;
pub mod billing;
pub mod cache;
pub mod db;
pub mod distributed_cache;
pub mod distributed_lock;
pub mod email;
pub mod error;
pub mod event_sourcing;
pub mod export_docs;
pub mod federation;
pub mod flags;
pub mod flash;
pub mod forms;
pub mod geo;
pub mod geo_ip;
pub mod graphql;
pub mod handler;
pub mod health;
pub mod image_opt;
pub mod image_pipeline;
pub mod jobs;
pub mod live_query;
pub mod mcp;
pub mod metrics_dashboard;
pub mod middleware;
pub mod og;
pub mod openapi;
pub mod otel;
pub mod outbox;
pub mod passkeys;
pub mod pdf_signer;
pub mod ppr;
pub mod rate_limit;
pub mod rebac;
pub mod replica_pool;
pub mod resilience;
pub mod resumable_upload;
pub mod security_audit;
pub mod seo;
pub mod session;
pub mod shield;
pub mod sse;
pub mod static_files;
pub mod storage;
pub mod streaming;
pub mod suspense;
pub mod telemetry;
pub mod testing;
pub mod turbo_stream;
pub mod web_push;
pub mod ws;

// Route Context, Loaders & Actions
pub use action::{ActionValidatorFn, BoxLoaderFn, FormAction, Loader, RouteContext};

// Pluggable Infrastructure Adapters (Database, Cache, Storage, LLM, Mail)
pub use adapters::{
    CacheAdapter, DatabaseAdapter, EmbeddedDatabaseAdapter, LlmAdapter, LocalLlmAdapter,
    MailAdapter, MemoryCacheAdapter, MemoryMailAdapter, MemoryStorageAdapter, ServiceAdapters,
    StorageAdapter,
};

// AI SDK, Streaming UI, Tool Calling & Hybrid RAG
pub use ai::{
    cosine_similarity, reciprocal_rank_fusion, stream_text_sse, stream_ui_fragment, AiTool,
    ChatMessage, ChatRole, Conversation, ToolRegistry, VectorIndex,
};

// Core YavApp types
pub use app::{AppRoute, BoxedHandler, IntoBoxedHandler, YavApp};

// Authentication, JWT, OAuth2/PKCE, Password Hashing & TOTP 2FA
pub use auth::{
    base64url_decode, base64url_encode, constant_time_eq, AuthUser, JwtClaims, JwtManager,
    OAuthConfig, OAuthProvider, PasswordHasher, Totp,
};

// Billing, Exact Money, Webhook Verifier & Idempotency
pub use billing::{
    Currency, IdempotencyStore, IdempotentRecord, Money, UsageMeter, WebhookVerifier,
};

// Enterprise Feature Flags, Multi-Tenancy, ABAC Policies & Hash-Chained Audit Log
pub use flags::{
    AuditEvent, AuditTrail, FeatureFlag, FlagEngine, PolicyEngine, PolicyResource, PolicySubject,
    TenantContext,
};

// Geospatial, Geofencing & Spatial Indexing
pub use geo::{GeoPoint, GeoPolygon, SpatialIndex, SpatialMatch};

// Object Storage & Presigned Upload URLs
pub use storage::{ObjectMeta, ObjectStorage, PresignedUrlSigner};

// SSR Cache & ISR
pub use cache::{CacheEntry, CachePolicy, CacheStats, SsrCache};

// Database Migrator, Parameterized Query Builder & Embedded Store
pub use db::{
    AppliedMigration, EmbeddedStore, Migration, MigrationStatus, Migrator, Order, QueryBuilder,
};

// Transactional Email Engine (RSX -> HTML + Plain Text)
pub use email::{extract_plain_text_from_node, EmailMessage, Mailer};

// Dynamic Image Optimization Endpoint (`/_yav/image`)
pub use image_opt::{
    generate_blur_svg_data_uri, image_optimizer_handler, optimized_image_url,
    resolve_safe_asset_path, ImageOptParams, ImageOptimizerConfig,
};

// Background Job Queue & Cron Scheduler
pub use jobs::{JobId, JobInfo, JobQueue, JobStatus};

// WebSockets & Real-time Fragment Broadcasting
pub use ws::{ws_node_frame, ws_swap_frame, WsBroadcast};

// Error types
pub use error::{render_error_page, YavError};

// Flash messages
pub use flash::{Flash, FlashLevel, FlashMessage, FLASH_COOKIE_NAME};

// Form validation & server actions
pub use forms::{
    csrf_field, verify_csrf_token, ActionResponse, ActionResult, CsrfToken, FormData, FormErrors,
    FromFormData, UploadedFile, CSRF_FIELD_NAME, CSRF_HEADER_NAME,
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
    security_headers_middleware, CspNonce, SecurityHeadersConfig,
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

// OpenGraph dynamic SVG image generation
pub use og::{generate_og_svg, og_image_handler, og_image_url, OgImageParams};

// Real-time Core Web Vitals telemetry
pub use telemetry::{
    classify_rating, global_vitals, vitals_handler, vitals_summary_handler, yav_vitals_script,
    MetricStats, VitalsCollector, VitalsSummary, WebVitalMetric,
};

// Partial Prerendering (PPR) & Component-Level Caching
pub use ppr::{global_component_cache, ppr_placeholder, ComponentCache, PprPage, PprSlot};

// WebAuthn / Passkeys Biometric Authentication
pub use passkeys::{
    yav_passkey_script, PasskeyAuthOptions, PasskeyCredential, PasskeyManager,
    PasskeyRegistrationOptions,
};

// OpenTelemetry (OTLP) Distributed Tracing & W3C traceparent
pub use otel::{global_tracer, otel_tracing_middleware, OtelTracer, SpanRecord, TraceParent};

// Full-Stack E2E TestClient & DOM Assertions
pub use testing::{DomAssert, DomTag, TestClient, TestResponse};

// Model Context Protocol (MCP) Server
pub use mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpPrompt, McpResource, McpServer};

// Vector PDF 1.4 & RFC 4180 CSV/TSV Document Exporters
pub use export_docs::{csv_response, pdf_response, CsvBuilder, PdfDocument, PdfTableRow};

// Real-Time Live Queries & CDC Server-Push
pub use live_query::{
    global_live_queries, live_query_sse_handler, LiveEvent, LiveQueryManager, LiveQueryStreamParams,
};

// Cryptographic Proof-of-Work Anti-Bot & Honeypot
pub use shield::{pow_client_script, Honeypot, PowChallenge, PowShield};

// Zero-Dependency GraphQL Schema & Playground
pub use graphql::{
    graphiql_html, graphql_handler, FieldResolver, GraphQlError, GraphQlField, GraphQlObject,
    GraphQlRequest, GraphQlResponse, GraphQlSchema, GraphQlType,
};

// Geolocation, Currency & European VAT Engine
pub use geo_ip::{extract_geo_context, GeoIpContext, TaxBreakdown, VatRule};

// Micro-Frontends & Module Federation Gateway
pub use federation::{FederationGateway, FederationRoute};

// Pure Rust Vector Barcode & QR Code Generator
pub use barcode::{Barcode128, QrCode};

// Automated Security Auditor & CSP Violation Telemetry
pub use security_audit::{
    csp_report_handler, global_csp_reporter, CspReporter, CspViolation, SecurityAuditReport,
    SecurityAuditor, SecurityGrade,
};

// Circuit Breaker, Bulkhead & Adaptive Concurrency Limiting
pub use resilience::{
    AdaptiveConcurrencyLimiter, BreakerState, Bulkhead, CircuitBreaker, CircuitBreakerConfig,
    ConcurrencyGuard,
};

// Transactional Outbox Pattern & Reliable Event Bus
pub use outbox::{OutboxMessage, OutboxStats, OutboxStatus, TransactionalOutbox};

// Multi-Region Database Replica Pool & Router
pub use replica_pool::{DbReplicaPool, ReplicaNode, RouteTarget};

// Pure-Rust WebP / AVIF & Image Pipeline
pub use image_pipeline::{
    detect_image_format, parse_image_metadata, ImageFormat, ImageMetadata, RawRgbImage, WebPEncoder,
};

// Zero-Dependency Vector PDF Signer & Watermarking
pub use pdf_signer::{PdfSignature, PdfSigner, PdfWatermark};

// Multi-Tier Distributed Cache & XFetch
pub use distributed_cache::{CacheItem, DistributedCache, MemoryL1Store};

// Distributed Mutex Locks & Leader Election
pub use distributed_lock::{DistributedLock, LeaderElector, LockCoordinator};

// Relationship-Based Access Control (ReBAC)
pub use rebac::{RebacGraph, RelationshipTuple};

// Metrics Dashboard & Monitoring Visualizer
pub use metrics_dashboard::{metrics_dashboard_handler, render_metrics_dashboard_html};

// TurboStream Multi-Zone DOM Mutations
pub use turbo_stream::{TurboAction, TurboMessage, TurboStream, YAV_TURBO_STREAM_SCRIPT};

// Event Sourcing & CQRS Projections
pub use event_sourcing::{DomainEvent, EventStore, EventStoreError, Projection};

// Resumable Uploads & Tus Protocol v1.0.0
pub use resumable_upload::{
    parse_tus_metadata, ChunkedUploader, TusError, UploadSession, TUS_RESUMABLE_VERSION,
};

// WebPush & VAPID Authorization
pub use web_push::{PushSubscription, VapidSigner, WebPushNotification};

// Axum re-exports for framework convenience
pub use axum::{
    extract::{Path, Query, Request},
    http::{HeaderMap, Method, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, patch, post, put, Router},
};

/// Convenience prelude module bringing common server types into scope.
pub mod prelude {
    pub use crate::action::{ActionValidatorFn, BoxLoaderFn, FormAction, Loader, RouteContext};
    pub use crate::adapters::{
        CacheAdapter, DatabaseAdapter, EmbeddedDatabaseAdapter, LlmAdapter, LocalLlmAdapter,
        MailAdapter, MemoryCacheAdapter, MemoryMailAdapter, MemoryStorageAdapter, ServiceAdapters,
        StorageAdapter,
    };
    pub use crate::ai::{
        cosine_similarity, reciprocal_rank_fusion, stream_text_sse, stream_ui_fragment, AiTool,
        ChatMessage, ChatRole, Conversation, ToolRegistry, VectorIndex,
    };
    pub use crate::app::{AppRoute, BoxedHandler, YavApp};
    pub use crate::auth::{
        AuthUser, JwtClaims, JwtManager, OAuthConfig, OAuthProvider, PasswordHasher, Totp,
    };
    pub use crate::barcode::{Barcode128, QrCode};
    pub use crate::billing::{
        Currency, IdempotencyStore, IdempotentRecord, Money, UsageMeter, WebhookVerifier,
    };
    pub use crate::cache::{CacheEntry, CachePolicy, CacheStats, SsrCache};
    pub use crate::db::{EmbeddedStore, Migration, Migrator, Order, QueryBuilder};
    pub use crate::distributed_cache::{CacheItem, DistributedCache, MemoryL1Store};
    pub use crate::distributed_lock::{DistributedLock, LeaderElector, LockCoordinator};
    pub use crate::email::{EmailMessage, Mailer};
    pub use crate::error::YavError;
    pub use crate::event_sourcing::{DomainEvent, EventStore, EventStoreError, Projection};
    pub use crate::export_docs::{
        csv_response, pdf_response, CsvBuilder, PdfDocument, PdfTableRow,
    };
    pub use crate::federation::{FederationGateway, FederationRoute};
    pub use crate::flags::{
        AuditEvent, AuditTrail, FeatureFlag, FlagEngine, PolicyEngine, PolicyResource,
        PolicySubject, TenantContext,
    };
    pub use crate::flash::{Flash, FlashLevel, FlashMessage};
    pub use crate::forms::{
        ActionResponse, ActionResult, CsrfToken, FormData, FormErrors, UploadedFile,
    };
    pub use crate::geo::{GeoPoint, GeoPolygon, SpatialIndex, SpatialMatch};
    pub use crate::geo_ip::{extract_geo_context, GeoIpContext, TaxBreakdown, VatRule};
    pub use crate::graphql::{
        graphiql_html, graphql_handler, GraphQlError, GraphQlField, GraphQlObject, GraphQlRequest,
        GraphQlResponse, GraphQlSchema, GraphQlType,
    };
    pub use crate::handler::{
        html_response, json_response, not_found, redirect, safe_redirect, HtmlFragment, HtmlPage,
        HtmxRequest,
    };
    pub use crate::health::{healthz, readyz, ServerMetrics};
    pub use crate::image_opt::{
        generate_blur_svg_data_uri, image_optimizer_handler, optimized_image_url,
    };
    pub use crate::image_pipeline::{
        detect_image_format, parse_image_metadata, ImageFormat, ImageMetadata, RawRgbImage,
        WebPEncoder,
    };
    pub use crate::jobs::{JobId, JobQueue, JobStatus};
    pub use crate::live_query::{
        global_live_queries, live_query_sse_handler, LiveEvent, LiveQueryManager,
        LiveQueryStreamParams,
    };
    pub use crate::mcp::{
        JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpPrompt, McpResource, McpServer,
    };
    pub use crate::metrics_dashboard::{metrics_dashboard_handler, render_metrics_dashboard_html};
    pub use crate::middleware::SecurityHeadersConfig;
    pub use crate::og::{generate_og_svg, og_image_handler, og_image_url, OgImageParams};
    pub use crate::openapi::{scalar_ui_handler, swagger_ui_handler, OpenApiSpec};
    pub use crate::otel::{
        global_tracer, otel_tracing_middleware, OtelTracer, SpanRecord, TraceParent,
    };
    pub use crate::outbox::{OutboxMessage, OutboxStats, OutboxStatus, TransactionalOutbox};
    pub use crate::passkeys::{
        yav_passkey_script, PasskeyAuthOptions, PasskeyCredential, PasskeyManager,
        PasskeyRegistrationOptions,
    };
    pub use crate::pdf_signer::{PdfSignature, PdfSigner, PdfWatermark};
    pub use crate::ppr::{
        global_component_cache, ppr_placeholder, ComponentCache, PprPage, PprSlot,
    };
    pub use crate::rate_limit::{RateLimitConfig, RateLimiter};
    pub use crate::rebac::{RebacGraph, RelationshipTuple};
    pub use crate::replica_pool::{DbReplicaPool, ReplicaNode, RouteTarget};
    pub use crate::resilience::{
        AdaptiveConcurrencyLimiter, BreakerState, Bulkhead, CircuitBreaker, CircuitBreakerConfig,
        ConcurrencyGuard,
    };
    pub use crate::resumable_upload::{
        parse_tus_metadata, ChunkedUploader, TusError, UploadSession, TUS_RESUMABLE_VERSION,
    };
    pub use crate::security_audit::{
        csp_report_handler, global_csp_reporter, CspReporter, CspViolation, SecurityAuditReport,
        SecurityAuditor, SecurityGrade,
    };
    pub use crate::seo::{RobotsTxt, Sitemap, SitemapEntry};
    pub use crate::session::{SameSite, Session, SessionConfig, SessionSecret};
    pub use crate::shield::{pow_client_script, Honeypot, PowChallenge, PowShield};
    pub use crate::sse::{sse_channel, sse_response, SseMsg};
    pub use crate::static_files::{mime_type_for_path, resolve_static_path, serve_static_file};
    pub use crate::storage::{ObjectMeta, ObjectStorage, PresignedUrlSigner};
    pub use crate::streaming::{streaming_html, StreamingHtml};
    pub use crate::suspense::Suspense;
    pub use crate::telemetry::{
        classify_rating, global_vitals, vitals_handler, vitals_summary_handler, yav_vitals_script,
        MetricStats, VitalsCollector, VitalsSummary, WebVitalMetric,
    };
    pub use crate::testing::{DomAssert, DomTag, TestClient, TestResponse};
    pub use crate::turbo_stream::{
        TurboAction, TurboMessage, TurboStream, YAV_TURBO_STREAM_SCRIPT,
    };
    pub use crate::web_push::{PushSubscription, VapidSigner, WebPushNotification};
    pub use crate::ws::{ws_node_frame, ws_swap_frame, WsBroadcast};

    pub use axum::{
        extract::{Path, Query, Request},
        http::{HeaderMap, Method, StatusCode},
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
