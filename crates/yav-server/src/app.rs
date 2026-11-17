//! Application builder and HTTP server configuration.
//!
//! Provides the primary [`YavApp`] struct for defining routes, configuring
//! static asset directories, applying middleware, and launching the Axum server.

use crate::middleware::{etag_middleware, request_logger_middleware, security_headers_middleware};
use axum::{
    extract::Request,
    http::Method,
    response::{IntoResponse, Response},
    routing::{MethodFilter, MethodRouter},
    Router,
};
use std::{collections::BTreeMap, future::Future, path::PathBuf, pin::Pin, sync::Arc};
use yav_build::manifest::PackageYav;

/// Percent-decode a route path. Returns `None` on a malformed escape.
fn percent_decode_path(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Windows reserved device names that must never be used as a path component.
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Build a filesystem directory for a static route, guaranteed to stay inside
/// `out_path`.
///
/// Unlike the old `replace("..", "")` approach this rejects (rather than
/// rewrites) traversal attempts, decodes percent-escapes before validating,
/// handles Windows backslashes and drive-letter prefixes, and filters
/// characters/reserved names that are invalid in Windows filenames.
fn safe_static_output_dir(out_path: &std::path::Path, uri_path: &str) -> Option<PathBuf> {
    let decoded = percent_decode_path(uri_path)?;
    let mut target = out_path.to_path_buf();

    for raw in decoded.split(['/', '\\']) {
        if raw.is_empty() || raw == "." {
            continue;
        }
        // `..` after decoding is a traversal attempt — reject the whole route.
        if raw == ".." {
            return None;
        }
        // Drive letters, NUL and control characters are never valid here.
        if raw.contains(':') || raw.contains('\0') || raw.chars().any(char::is_control) {
            return None;
        }
        let safe: String = raw
            .chars()
            .map(|c| match c {
                '<' | '>' | '"' | '|' | '?' | '*' => '_',
                _ => c,
            })
            .collect();
        // Windows silently strips trailing dots/spaces and treats some names as devices.
        let trimmed = safe.trim_end_matches(['.', ' ']);
        if trimmed.is_empty()
            || WINDOWS_RESERVED_NAMES.contains(&trimmed.to_ascii_lowercase().as_str())
        {
            return None;
        }
        target.push(trimmed);
    }

    if !target.starts_with(out_path) {
        return None;
    }
    Some(target)
}

/// A pinned, boxed future yielding a [`Response`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed, thread-safe asynchronous HTTP handler function.
#[derive(Clone)]
pub struct BoxedHandler(Arc<dyn Fn(Request) -> BoxFuture<'static, Response> + Send + Sync>);

impl BoxedHandler {
    /// Create a new boxed handler from an async function or closure taking [`Request`] and returning an [`IntoResponse`].
    pub fn new<F, Fut, R>(f: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        BoxedHandler(Arc::new(move |req| {
            let mut ctx_map = crate::action::RouteContext::from_request(&req).ssr_context_map();
            if let Some(chain) = req.extensions().get::<yav_core::SsrLayoutChain>() {
                ctx_map.insert(chain.clone());
            }
            let fut = yav_core::with_context_map(ctx_map.clone(), || f(req));
            Box::pin(async move {
                yav_core::with_context_map_async(ctx_map, fut)
                    .await
                    .into_response()
            })
        }))
    }

    /// Create a handler from a function taking no arguments.
    pub fn from_zero_arg<F, Fut, R>(f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        BoxedHandler(Arc::new(move |req| {
            let mut ctx_map = crate::action::RouteContext::from_request(&req).ssr_context_map();
            if let Some(chain) = req.extensions().get::<yav_core::SsrLayoutChain>() {
                ctx_map.insert(chain.clone());
            }
            let fut = yav_core::with_context_map(ctx_map.clone(), &f);
            Box::pin(async move {
                yav_core::with_context_map_async(ctx_map, fut)
                    .await
                    .into_response()
            })
        }))
    }

    /// Create a handler from an async function or closure taking [`RouteContext`](crate::action::RouteContext).
    pub fn from_ctx<F, Fut, R>(f: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        BoxedHandler(Arc::new(move |req| {
            let ctx = crate::action::RouteContext::from_request(&req);
            let mut ctx_map = ctx.ssr_context_map();
            if let Some(chain) = req.extensions().get::<yav_core::SsrLayoutChain>() {
                ctx_map.insert(chain.clone());
            }
            let fut = yav_core::with_context_map(ctx_map.clone(), || f(ctx));
            Box::pin(async move {
                yav_core::with_context_map_async(ctx_map, fut)
                    .await
                    .into_response()
            })
        }))
    }

    /// Wrap this handler with an SSR layout transformation chain.
    pub fn with_layout<L>(self, layout_fn: L) -> Self
    where
        L: Fn(yav_core::Node) -> yav_core::Node + Send + Sync + 'static,
    {
        let inner = self.0;
        let chain = yav_core::SsrLayoutChain(Arc::new(layout_fn));
        BoxedHandler(Arc::new(move |mut req| {
            req.extensions_mut().insert(chain.clone());
            let mut ctx_map = crate::action::RouteContext::from_request(&req).ssr_context_map();
            ctx_map.insert(chain.clone());
            let fut = yav_core::with_context_map(ctx_map.clone(), || inner(req));
            Box::pin(async move { yav_core::with_context_map_async(ctx_map, fut).await })
        }))
    }

    /// Create a handler from an async function accepting 1 Axum extractor (`Path<T>`, `Query<T>`, `Session`, `RouteContext`, `FormData`, etc.).
    pub fn from_extractor1<F, Fut, R, T1, M1>(f: F) -> Self
    where
        F: Fn(T1) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
        T1: axum::extract::FromRequest<(), M1> + Send + 'static,
        M1: 'static,
    {
        BoxedHandler(Arc::new(move |req| {
            let f = f.clone();
            let mut ctx_map = crate::action::RouteContext::from_request(&req).ssr_context_map();
            if let Some(chain) = req.extensions().get::<yav_core::SsrLayoutChain>() {
                ctx_map.insert(chain.clone());
            }
            Box::pin(async move {
                let t1 = match T1::from_request(req, &()).await {
                    Ok(v) => v,
                    Err(rej) => return rej.into_response(),
                };
                let fut = yav_core::with_context_map(ctx_map.clone(), || f(t1));
                yav_core::with_context_map_async(ctx_map, fut)
                    .await
                    .into_response()
            })
        }))
    }

    /// Create a handler from an async function accepting 2 Axum extractors (`T1` from request parts, `T2` from request).
    pub fn from_extractor2<F, Fut, R, T1, T2, M2>(f: F) -> Self
    where
        F: Fn(T1, T2) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
        T1: axum::extract::FromRequestParts<()> + Send + 'static,
        T2: axum::extract::FromRequest<(), M2> + Send + 'static,
        M2: 'static,
    {
        BoxedHandler(Arc::new(move |req| {
            let f = f.clone();
            let mut ctx_map = crate::action::RouteContext::from_request(&req).ssr_context_map();
            if let Some(chain) = req.extensions().get::<yav_core::SsrLayoutChain>() {
                ctx_map.insert(chain.clone());
            }
            Box::pin(async move {
                let (mut parts, body) = req.into_parts();
                let t1 = match T1::from_request_parts(&mut parts, &()).await {
                    Ok(v) => v,
                    Err(rej) => return rej.into_response(),
                };
                let req = Request::from_parts(parts, body);
                let t2 = match T2::from_request(req, &()).await {
                    Ok(v) => v,
                    Err(rej) => return rej.into_response(),
                };
                let fut = yav_core::with_context_map(ctx_map.clone(), || f(t1, t2));
                yav_core::with_context_map_async(ctx_map, fut)
                    .await
                    .into_response()
            })
        }))
    }

    /// Wrap this handler with a scoped route/directory middleware (`_middleware.rs`).
    pub fn with_middleware<M, Fut>(self, mw: M) -> Self
    where
        M: Fn(Request, BoxedHandler) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let next = self.clone();
        BoxedHandler(Arc::new(move |req| {
            let mw = mw.clone();
            let next = next.clone();
            Box::pin(async move { mw(req, next).await })
        }))
    }

    /// Execute the handler for an incoming request.
    pub async fn call(&self, req: Request) -> Response {
        (self.0)(req).await
    }
}

impl std::fmt::Debug for BoxedHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedHandler").finish()
    }
}

/// Marker type for [`IntoBoxedHandler`] implementations taking a [`Request`].
pub struct RequestHandlerMarker;

/// Marker type for [`IntoBoxedHandler`] implementations taking zero arguments.
pub struct ZeroArgHandlerMarker;

/// Trait for types convertible into a [`BoxedHandler`].
pub trait IntoBoxedHandler<M = RequestHandlerMarker>: Send + Sync + 'static {
    /// Convert `self` into a [`BoxedHandler`].
    fn into_boxed_handler(self) -> BoxedHandler;
}

impl IntoBoxedHandler<RequestHandlerMarker> for BoxedHandler {
    fn into_boxed_handler(self) -> BoxedHandler {
        self
    }
}

impl<F, Fut, R> IntoBoxedHandler<RequestHandlerMarker> for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse + 'static,
{
    fn into_boxed_handler(self) -> BoxedHandler {
        BoxedHandler::new(self)
    }
}

impl<F, Fut, R> IntoBoxedHandler<ZeroArgHandlerMarker> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse + 'static,
{
    fn into_boxed_handler(self) -> BoxedHandler {
        BoxedHandler::from_zero_arg(self)
    }
}

/// Represents a route definition within the application.
pub struct AppRoute {
    /// The URL pattern for this route (e.g., `"/users"` or `"/users/{id}"`).
    pub path: String,
    /// The HTTP method for this route (e.g., `GET`, `POST`).
    pub method: Method,
    /// The boxed handler responsible for processing requests to this route.
    pub handler: BoxedHandler,
}

impl AppRoute {
    /// Create a new application route, automatically normalizing `:param` and `*wildcard` to Axum syntax.
    pub fn new<M>(
        path: impl Into<String>,
        method: Method,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        let raw_path: String = path.into();
        let normalized = yav_router::route::to_axum_pattern(&raw_path);
        Self {
            path: normalized,
            method,
            handler: handler.into_boxed_handler(),
        }
    }
}

impl std::fmt::Debug for AppRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppRoute")
            .field("path", &self.path)
            .field("method", &self.method)
            .finish()
    }
}

/// The core application builder and server for the Yav web framework.
///
/// Follows the builder pattern to configure routes, static file serving,
/// host, and port settings.
///
/// # Examples
/// ```rust,no_run
/// use yav_server::{YavApp, html_response, StatusCode};
///
/// let app = YavApp::new()
///     .port(3000)
///     .static_dir("public")
///     .get("/", |_req| async {
///         html_response(StatusCode::OK, "<h1>Hello, Yav!</h1>")
///     });
///
/// let router = app.into_router();
/// ```
pub struct YavApp {
    /// Registered application routes.
    pub routes: Vec<AppRoute>,
    /// Optional directory path from which static files are served at `/public`.
    pub static_dir: Option<PathBuf>,
    /// Port number the HTTP server will bind to.
    pub port: u16,
    /// Host IP address or hostname the server will bind to.
    pub host: String,
    /// Whether automatic HTTP ETag and 304 Not Modified caching is enabled.
    pub etag: bool,
    /// Whether CORS is enabled.
    pub cors: bool,
    /// Optional explicit list of allowed CORS origins (e.g. `["https://example.com"]`).
    pub cors_origins: Vec<String>,
    /// Session signing secret key.
    pub session_secret: Option<crate::session::SessionSecret>,
    /// Session cookie and lifecycle configuration.
    pub session_config: crate::session::SessionConfig,
    /// Optional rate limiter configuration.
    pub rate_limiter: Option<crate::rate_limit::RateLimiter>,
    /// Server metrics collector.
    pub metrics: crate::health::ServerMetrics,
    /// Discovered file-system routes.
    pub fs_router: Option<yav_router::fs_router::FileSystemRouter>,
    /// Custom state and layer transformers.
    pub extensions: Vec<Arc<dyn Fn(Router) -> Router + Send + Sync>>,
    /// Internationalization dictionary catalog.
    pub i18n: Option<Arc<yav_core::I18n>>,
    /// Custom 404 Not Found fallback handler.
    pub not_found_handler: Option<BoxedHandler>,
    /// Whether HTTP response compression (gzip, brotli) is enabled.
    pub compression: bool,
    /// Shared SSR state context map populated by `.state(T)` and accessible via `use_state::<T>()`.
    pub state_context: yav_core::ContextMap,
    /// In-memory SSR & ISR response cache.
    pub ssr_cache: crate::cache::SsrCache,
    /// CSRF protection policy. **Enabled by default** since v0.1.5; disable it
    /// explicitly with [`CsrfConfig::disabled`](crate::middleware::CsrfConfig::disabled)
    /// via [`YavApp::csrf_protection`].
    pub csrf: crate::middleware::CsrfConfig,
    /// Whether to install the automatic per-request OpenTelemetry span layer.
    /// Off by default; enable with [`YavApp::otel`].
    pub otel: bool,
}

impl Default for YavApp {
    fn default() -> Self {
        Self::new()
    }
}

impl YavApp {
    /// Create a new `YavApp` instance.
    ///
    /// Automatically loads `package.yavml` (or `yav.toml`) from the current directory if it exists,
    /// respecting environment overrides (`PORT`, `HOST`, `SESSION_SECRET`).
    /// Automatically defaults `static_dir` to `manifest.build.public_dir` (e.g. `"public"`) if that
    /// directory exists on disk.
    /// Falls back to default settings (host: `127.0.0.1`, port: `3000`) if
    /// no config file is found.
    #[must_use]
    pub fn new() -> Self {
        let manifest = match PackageYav::discover() {
            Ok(m) => m,
            Err(e) => {
                let path_yavml = std::path::Path::new("package.yavml");
                let path_yav = std::path::Path::new("package.yav");
                let path_toml = std::path::Path::new("yav.toml");
                if !path_yavml.exists() && !path_yav.exists() && !path_toml.exists() {
                    PackageYav::default()
                } else {
                    eprintln!("\n\x1b[1;31m❌ [YavApp] Fatal configuration error in package.yavml:\x1b[0m\n  {e}\n");
                    panic!("Failed to initialize YavApp: error in package.yavml: {e}");
                }
            }
        };

        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(manifest.server.port);

        let host = std::env::var("HOST").unwrap_or(manifest.server.host);

        let cors = manifest.server.cors;

        let session_secret = std::env::var("SESSION_SECRET")
            .or_else(|_| std::env::var("YAV_SECRET"))
            .ok()
            .map(crate::session::SessionSecret::new);

        let default_public = std::path::Path::new(&manifest.build.public_dir);
        let static_dir = if default_public.is_dir() {
            Some(default_public.to_path_buf())
        } else {
            None
        };

        Self {
            routes: Vec::new(),
            static_dir,
            port,
            host,
            etag: true,
            cors,
            cors_origins: Vec::new(),
            session_secret,
            session_config: crate::session::SessionConfig::default(),
            rate_limiter: None,
            metrics: crate::health::ServerMetrics::new(),
            fs_router: None,
            extensions: Vec::new(),
            i18n: None,
            not_found_handler: None,
            compression: manifest.server.compression,
            state_context: yav_core::ContextMap::new(),
            ssr_cache: crate::cache::SsrCache::new(),
            csrf: crate::middleware::CsrfConfig::default(),
            otel: false,
        }
    }

    /// Enable CSRF protection for state-changing requests.
    ///
    /// Once enabled, every POST/PUT/PATCH/DELETE must carry the session's CSRF
    /// token either in the `x-csrf-token` header or in the `_csrf` form field.
    /// Render the token into forms with
    /// [`crate::forms::CsrfToken::from_session`] / `hidden_input()`.
    /// Use [`crate::middleware::CsrfConfig::exempt`] for webhook endpoints.
    #[must_use]
    pub fn csrf_protection(mut self, config: crate::middleware::CsrfConfig) -> Self {
        self.csrf = config;
        self
    }

    /// Set whether automatic HTTP response compression (gzip, brotli) is enabled.
    #[must_use]
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Set whether automatic ETag generation and 304 Not Modified caching is enabled.
    #[must_use]
    pub fn etag(mut self, enabled: bool) -> Self {
        self.etag = enabled;
        self
    }

    /// Enable or disable CORS headers.
    #[must_use]
    pub fn cors(mut self, enabled: bool) -> Self {
        self.cors = enabled;
        self
    }

    /// Enable CORS and restrict allowed origins to an explicit allowlist (e.g. `["https://app.example.com"]`).
    #[must_use]
    pub fn cors_origins(mut self, origins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.cors = true;
        self.cors_origins = origins.into_iter().map(Into::into).collect();
        self
    }

    /// Set custom session signing secret key.
    #[must_use]
    pub fn session_secret(mut self, secret: impl Into<Vec<u8>>) -> Self {
        self.session_secret = Some(crate::session::SessionSecret::new(secret));
        self
    }

    /// Configure session settings (cookie name, TTL, path, SameSite, etc.).
    #[must_use]
    pub fn session(mut self, config: crate::session::SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// Set the port number.
    #[must_use]
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the host address.
    #[must_use]
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Configure the directory to serve static files from at `/public`.
    #[must_use]
    pub fn static_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(dir.into());
        self
    }

    /// Configure in-memory sliding window rate limiting.
    #[must_use]
    pub fn rate_limit(mut self, config: crate::rate_limit::RateLimitConfig) -> Self {
        self.rate_limiter = Some(crate::rate_limit::RateLimiter::new(config));
        self
    }

    /// Attach an existing [`RateLimiter`](crate::rate_limit::RateLimiter) instance.
    #[must_use]
    pub fn rate_limiter(mut self, limiter: crate::rate_limit::RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Provide a custom [`ServerMetrics`](crate::health::ServerMetrics) telemetry tracker.
    #[must_use]
    pub fn metrics(mut self, metrics: crate::health::ServerMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Install automatic OpenTelemetry instrumentation for every HTTP request.
    ///
    /// When enabled, [`crate::otel::otel_tracing_middleware`] is layered onto the
    /// router. It reads or creates a W3C `traceparent`, echoes it on the
    /// response, and records exactly one `SERVER` span per request in
    /// [`crate::otel::global_tracer()`] with method, path, status and duration.
    /// This is per-request HTTP instrumentation only: database calls, AI helpers
    /// and outbound HTTP requests still need explicit spans, and exporting
    /// remains a manual `export_otlp_http` / `export_prometheus` call.
    ///
    /// Off by default so existing applications keep their exact header set.
    #[must_use]
    pub fn otel(mut self, enabled: bool) -> Self {
        self.otel = enabled;
        self
    }

    /// Register shared application state or database pool accessible to all handlers via `axum::extract::Extension<T>`
    /// as well as inside any SSR component via [`yav_core::use_state`] / [`yav_core::use_context`].
    #[must_use]
    pub fn state<T: Clone + Send + Sync + 'static>(mut self, state: T) -> Self {
        self.state_context.insert(state.clone());
        self.extensions.push(Arc::new(move |router| {
            router.layer(axum::Extension(state.clone()))
        }));
        self
    }

    /// Register a custom extension or service layer on the router.
    #[must_use]
    pub fn extension<T: Clone + Send + Sync + 'static>(self, ext: T) -> Self {
        self.state(ext)
    }

    /// Register a custom middleware function (e.g. `|req, next| async { next.run(req).await }`).
    #[must_use]
    pub fn middleware<F, Fut>(mut self, middleware_fn: F) -> Self
    where
        F: Fn(axum::extract::Request, axum::middleware::Next) -> Fut
            + Clone
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = axum::response::Response> + Send + 'static,
    {
        self.extensions.push(Arc::new(move |router| {
            router.layer(axum::middleware::from_fn(middleware_fn.clone()))
        }));
        self
    }

    /// Configure custom HTTP security headers and Content-Security-Policy (CSP).
    #[must_use]
    pub fn security_headers(self, config: crate::middleware::SecurityHeadersConfig) -> Self {
        self.state(config)
    }

    /// Configure a custom 404 Not Found fallback handler.
    #[must_use]
    pub fn not_found_handler<M>(mut self, handler: impl IntoBoxedHandler<M>) -> Self {
        self.not_found_handler = Some(handler.into_boxed_handler());
        self
    }

    /// Configure a custom 404 Not Found fallback handler wrapped with an SSR layout hierarchy.
    #[must_use]
    pub fn not_found_handler_with_layout<M, L>(
        mut self,
        handler: impl IntoBoxedHandler<M>,
        layout_fn: L,
    ) -> Self
    where
        L: Fn(yav_core::Node) -> yav_core::Node + Send + Sync + 'static,
    {
        self.not_found_handler = Some(handler.into_boxed_handler().with_layout(layout_fn));
        self
    }

    /// Attach a [`FileSystemRouter`](yav_router::fs_router::FileSystemRouter) to the application for route reflection and discovery.
    #[must_use]
    pub fn with_fs_router(mut self, fs_router: yav_router::fs_router::FileSystemRouter) -> Self {
        self.fs_router = Some(fs_router);
        self
    }

    /// Attach an [`I18n`](yav_core::I18n) catalog for request locale resolution and translations.
    #[must_use]
    pub fn i18n(mut self, i18n: yav_core::I18n) -> Self {
        self.i18n = Some(Arc::new(i18n));
        self
    }

    /// Automatically discover routes from the file system (e.g. `"src/routes"`).
    pub fn discover_routes(
        mut self,
        routes_dir: impl AsRef<std::path::Path>,
    ) -> std::io::Result<Self> {
        let router = yav_router::fs_router::FileSystemRouter::scan_dir(routes_dir)?;
        self.fs_router = Some(router);
        Ok(self)
    }

    /// Get all discovered file-system routes if a FileSystemRouter is attached.
    pub fn discovered_routes(&self) -> Option<&[yav_router::fs_router::DiscoveredRoute]> {
        self.fs_router.as_ref().map(|r| r.routes())
    }

    /// Check if a path pattern is already registered in the application routes.
    pub fn has_route(&self, pattern: &str) -> bool {
        let normalized = yav_router::route::to_axum_pattern(pattern);
        self.routes
            .iter()
            .any(|r| r.path == normalized || r.path == pattern)
    }

    /// Register a scoped group of routes sharing a common URL path prefix and shared settings.
    ///
    /// # Example
    /// ```rust,no_run
    /// use yav_server::{YavApp, html_response, StatusCode};
    ///
    /// let app = YavApp::new()
    ///     .group("/admin", |admin| {
    ///         admin
    ///             .get("/", |_req| async { html_response(StatusCode::OK, "Admin Dashboard") })
    ///             .get("/users", |_req| async { html_response(StatusCode::OK, "User Management") })
    ///     });
    /// ```
    pub fn group<F>(mut self, prefix: impl AsRef<str>, configure: F) -> Self
    where
        F: FnOnce(YavApp) -> YavApp,
    {
        let raw_prefix = prefix.as_ref().trim_matches('/');
        let sub = YavApp {
            routes: Vec::new(),
            static_dir: None,
            port: self.port,
            host: self.host.clone(),
            etag: self.etag,
            cors: self.cors,
            cors_origins: self.cors_origins.clone(),
            session_secret: self.session_secret.clone(),
            session_config: self.session_config.clone(),
            rate_limiter: self.rate_limiter.clone(),
            metrics: self.metrics.clone(),
            fs_router: self.fs_router.clone(),
            extensions: Vec::new(),
            i18n: self.i18n.clone(),
            not_found_handler: self.not_found_handler.clone(),
            compression: self.compression,
            state_context: self.state_context.clone(),
            ssr_cache: self.ssr_cache.clone(),
            csrf: self.csrf.clone(),
            otel: self.otel,
        };
        let configured = configure(sub);
        for mut r in configured.routes {
            let clean_path = r.path.trim_start_matches('/');
            r.path = if clean_path.is_empty() {
                format!("/{raw_prefix}")
            } else {
                format!("/{raw_prefix}/{clean_path}")
            };
            self.routes.push(r);
        }
        self.state_context.extend(&configured.state_context);
        self.extensions.extend(configured.extensions);
        self
    }

    /// Add a route with a custom path, HTTP method, and asynchronous handler.
    #[must_use]
    pub fn route<M>(
        mut self,
        path: impl Into<String>,
        method: Method,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        self.routes.push(AppRoute::new(path, method, handler));
        self
    }

    /// Add a route wrapped with an SSR layout transformation hierarchy.
    #[must_use]
    pub fn route_with_layout<M, L>(
        mut self,
        path: impl Into<String>,
        method: Method,
        handler: impl IntoBoxedHandler<M>,
        layout_fn: L,
    ) -> Self
    where
        L: Fn(yav_core::Node) -> yav_core::Node + Send + Sync + 'static,
    {
        let boxed = handler.into_boxed_handler().with_layout(layout_fn);
        self.routes.push(AppRoute::new(path, method, boxed));
        self
    }

    /// Register a `GET` route.
    #[must_use]
    pub fn get<M>(self, path: impl Into<String>, handler: impl IntoBoxedHandler<M>) -> Self {
        self.route(path, Method::GET, handler)
    }

    /// Register a `POST` route.
    #[must_use]
    pub fn post<M>(self, path: impl Into<String>, handler: impl IntoBoxedHandler<M>) -> Self {
        self.route(path, Method::POST, handler)
    }

    /// Register a `PUT` route.
    #[must_use]
    pub fn put<M>(self, path: impl Into<String>, handler: impl IntoBoxedHandler<M>) -> Self {
        self.route(path, Method::PUT, handler)
    }

    /// Register a `DELETE` route.
    #[must_use]
    pub fn delete<M>(self, path: impl Into<String>, handler: impl IntoBoxedHandler<M>) -> Self {
        self.route(path, Method::DELETE, handler)
    }

    /// Register a `PATCH` route.
    #[must_use]
    pub fn patch<M>(self, path: impl Into<String>, handler: impl IntoBoxedHandler<M>) -> Self {
        self.route(path, Method::PATCH, handler)
    }

    /// Register a `GET` route handler that receives [`RouteContext`](crate::action::RouteContext) directly.
    #[must_use]
    pub fn get_ctx<F, Fut, R>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        self.route(path, Method::GET, BoxedHandler::from_ctx(handler))
    }

    /// Register a `POST` route handler that receives [`RouteContext`](crate::action::RouteContext) directly.
    #[must_use]
    pub fn post_ctx<F, Fut, R>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        self.route(path, Method::POST, BoxedHandler::from_ctx(handler))
    }

    /// Register a `PUT` route handler that receives [`RouteContext`](crate::action::RouteContext) directly.
    #[must_use]
    pub fn put_ctx<F, Fut, R>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        self.route(path, Method::PUT, BoxedHandler::from_ctx(handler))
    }

    /// Register a `DELETE` route handler that receives [`RouteContext`](crate::action::RouteContext) directly.
    #[must_use]
    pub fn delete_ctx<F, Fut, R>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        self.route(path, Method::DELETE, BoxedHandler::from_ctx(handler))
    }

    /// Register a `PATCH` route handler that receives [`RouteContext`](crate::action::RouteContext) directly.
    #[must_use]
    pub fn patch_ctx<F, Fut, R>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(crate::action::RouteContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        self.route(path, Method::PATCH, BoxedHandler::from_ctx(handler))
    }

    /// Register a `GET` route accepting any single Axum extractor (`Path<T>`, `Query<T>`, `Session`, `RouteContext`).
    #[must_use]
    pub fn get_with<F, Fut, R, T1, M1>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(T1) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
        T1: axum::extract::FromRequest<(), M1> + Send + 'static,
        M1: 'static,
    {
        self.route(path, Method::GET, BoxedHandler::from_extractor1(handler))
    }

    /// Register a `POST` route accepting any single Axum extractor (`FormData`, `Session`, `RouteContext`, `Path<T>`).
    #[must_use]
    pub fn post_with<F, Fut, R, T1, M1>(self, path: impl Into<String>, handler: F) -> Self
    where
        F: Fn(T1) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
        T1: axum::extract::FromRequest<(), M1> + Send + 'static,
        M1: 'static,
    {
        self.route(path, Method::POST, BoxedHandler::from_extractor1(handler))
    }

    /// Mount the built-in `/_yav/image` dynamic image optimization & blur placeholder endpoint.
    #[must_use]
    pub fn with_image_optimizer(self) -> Self {
        self.get_with("/_yav/image", crate::image_opt::image_optimizer_handler)
    }

    /// Register a cached `GET` route with Incremental Static Regeneration (`stale-while-revalidate`) and TTL.
    ///
    /// Responses are cached in memory for `ttl`. Subsequent requests within `ttl` receive an instant
    /// response with `X-Yav-Cache: HIT`. Expired entries are served immediately with `X-Yav-Cache: STALE`
    /// while a background task regenerates the fresh SSR response.
    #[must_use]
    pub fn get_cached<M>(
        self,
        path: impl Into<String>,
        ttl: std::time::Duration,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        self.get_cached_with_policy(path, crate::cache::CachePolicy::new(ttl), handler)
    }

    /// Register a cached GET route with an explicit [`CachePolicy`] (TTL + SWR toggle).
    pub fn get_cached_with_policy<M>(
        self,
        path: impl Into<String>,
        policy: crate::cache::CachePolicy,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        self.get_cached_with_policy_and_tags(path, policy, Vec::<String>::new(), handler)
    }

    /// Register a cached GET route whose entry can be invalidated by cache tags.
    pub fn get_cached_with_tags<M>(
        self,
        path: impl Into<String>,
        ttl: std::time::Duration,
        tags: Vec<String>,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        self.get_cached_with_policy_and_tags(
            path,
            crate::cache::CachePolicy::new(ttl),
            tags,
            handler,
        )
    }

    /// Core cached-route implementation. Caches are always private, are bypassed for
    /// any request carrying credentials, and never store responses that set cookies.
    pub fn get_cached_with_policy_and_tags<M>(
        self,
        path: impl Into<String>,
        policy: crate::cache::CachePolicy,
        tags: Vec<String>,
        handler: impl IntoBoxedHandler<M>,
    ) -> Self {
        let inner = handler.into_boxed_handler();
        let cache = self.ssr_cache.clone();
        let cached_handler = BoxedHandler(Arc::new(move |req: Request| {
            let inner = inner.clone();
            let cache = cache.clone();
            let tags = tags.clone();
            let ttl = policy.ttl;
            let stale_while_revalidate = policy.stale_while_revalidate;
            let bg_tags = tags.clone();
            Box::pin(async move {
                // Any credential — not just the default `_yav_session` cookie name —
                // makes a response user-specific. Custom cookie names must not leak
                // authenticated SSR pages into a shared cache.
                let has_auth = req
                    .headers()
                    .contains_key(axum::http::header::AUTHORIZATION)
                    || req.headers().contains_key(axum::http::header::COOKIE);
                if has_auth {
                    return inner.call(req).await;
                }

                let lang = req
                    .headers()
                    .get(axum::http::header::ACCEPT_LANGUAGE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                let is_hx = req.headers().get("HX-Request").is_some();
                let path_and_query = match req.uri().query() {
                    Some(q) if !q.is_empty() => format!("{}?{}", req.uri().path(), q),
                    _ => req.uri().path().to_string(),
                };
                let cache_key = format!(
                    "{}|{}|{}",
                    path_and_query,
                    lang,
                    if is_hx { "hx" } else { "full" }
                );

                if let Some(entry) = cache.get(&cache_key) {
                    if entry.is_fresh(ttl) {
                        return entry.into_response_with_status("HIT", ttl);
                    }
                    // Stale-while-revalidate: serve stale immediately and trigger a
                    // background refresh, but only when the policy actually enables it.
                    if stale_while_revalidate && cache.try_begin_revalidation(&cache_key) {
                        let bg_cache = cache.clone();
                        let bg_inner = inner.clone();
                        let bg_key = cache_key.clone();
                        let bg_tags = bg_tags.clone();
                        let mut bg_req = Request::builder()
                            .method(Method::GET)
                            .uri(req.uri().clone())
                            .body(axum::body::Body::empty())
                            .unwrap_or_else(|_| Request::new(axum::body::Body::empty()));
                        *bg_req.headers_mut() = req.headers().clone();
                        *bg_req.extensions_mut() = req.extensions().clone();
                        tokio::spawn(async move {
                            let resp = bg_inner.call(bg_req).await;
                            let status = resp.status();
                            if status.is_success()
                                && !resp.headers().contains_key(axum::http::header::SET_COOKIE)
                            {
                                let headers = resp.headers().clone();
                                if let Ok(bytes) =
                                    axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await
                                {
                                    let entry = crate::cache::CacheEntry::from_response(
                                        status, &headers, bytes,
                                    );
                                    bg_cache.insert_with_tags(bg_key, entry, bg_tags);
                                    return;
                                }
                            }
                            bg_cache.cancel_revalidation(&bg_key);
                        });
                        return entry.into_response_with_status("STALE", ttl);
                    }
                    // SWR disabled (or a refresh is already running): fall through and
                    // render a fresh response synchronously instead of serving stale data.
                }

                let resp = inner.call(req).await;
                let status = resp.status();
                // Never cache a response that establishes a session or rotates a token:
                // replaying Set-Cookie on a HIT would hand one visitor's cookie to another.
                if !status.is_success()
                    || resp.headers().contains_key(axum::http::header::SET_COOKIE)
                {
                    return resp;
                }
                let headers = resp.headers().clone();
                match axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await {
                    Ok(bytes) => {
                        let entry =
                            crate::cache::CacheEntry::from_response(status, &headers, bytes);
                        let out = entry.into_response_with_status("MISS", ttl);
                        cache.insert_with_tags(cache_key, entry, tags);
                        out
                    }
                    Err(_) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Cache buffer error",
                    )
                        .into_response(),
                }
            })
        }));

        self.route(path, Method::GET, cached_handler)
    }

    /// Register an OpenAPI 3.1 specification and automatically mount interactive Scalar and Swagger API documentation.
    ///
    /// Exposes:
    /// - `GET {path}/openapi.json` — raw OpenAPI 3.1 JSON specification
    /// - `GET {path}` — modern Scalar API reference viewer
    /// - `GET {path}/swagger` — classic Swagger UI viewer
    ///
    /// # Example
    /// ```rust,no_run
    /// use yav_server::{YavApp, OpenApiSpec};
    ///
    /// let spec = OpenApiSpec::new("My API", "1.0.0")
    ///     .endpoint("/api/users", "GET", "List all users", "users");
    ///
    /// let app = YavApp::new().openapi("/docs", spec);
    /// ```
    #[must_use]
    pub fn openapi(mut self, path: impl AsRef<str>, spec: crate::openapi::OpenApiSpec) -> Self {
        let base_path = path.as_ref().trim_end_matches('/').to_string();
        let json_path = format!("{base_path}/openapi.json");
        let spec_json = spec.to_json();
        let title = spec.title.clone();

        // 1. JSON spec endpoint
        let json_data = spec_json;
        self = self.get(&json_path, move |_req| {
            let data = json_data.clone();
            async move {
                (
                    axum::http::StatusCode::OK,
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "application/json; charset=utf-8",
                    )],
                    data,
                )
            }
        });

        // 2. Scalar UI at {base_path}
        let scalar_json_path = json_path.clone();
        let scalar_title = title.clone();
        self = self.get(&base_path, move |_req| {
            let p = scalar_json_path.clone();
            let t = scalar_title.clone();
            async move { crate::openapi::scalar_ui_handler(&t, &p) }
        });

        // 3. Swagger UI at {base_path}/swagger
        let swagger_path = format!("{base_path}/swagger");
        let swagger_json_path = json_path;
        let swagger_title = title;
        self = self.get(&swagger_path, move |_req| {
            let p = swagger_json_path.clone();
            let t = swagger_title.clone();
            async move { crate::openapi::swagger_ui_handler(&t, &p) }
        });

        self
    }

    /// Convert the configured `YavApp` into an Axum [`Router`].
    ///
    /// Useful for testing, embedding in other applications, or applying custom layers.
    pub fn into_router(self) -> Router {
        let mut routes_by_path: BTreeMap<String, Vec<(Method, BoxedHandler)>> = BTreeMap::new();

        for route in self.routes {
            routes_by_path
                .entry(route.path)
                .or_default()
                .push((route.method, route.handler));
        }

        let mut router = Router::new();

        // Built-in health checks (/health, /healthz, /readyz, and /metrics) if not overridden
        if !routes_by_path.contains_key("/health") {
            router = router.route("/health", axum::routing::get(crate::health::healthz));
        }
        if !routes_by_path.contains_key("/healthz") {
            router = router.route("/healthz", axum::routing::get(crate::health::healthz));
        }
        if !routes_by_path.contains_key("/readyz") {
            router = router.route("/readyz", axum::routing::get(crate::health::readyz));
        }
        if !routes_by_path.contains_key("/metrics") {
            router = router.route(
                "/metrics",
                axum::routing::get(crate::health::metrics_handler),
            );
        }
        if !routes_by_path.contains_key("/_yav/image") {
            router = router.route(
                "/_yav/image",
                axum::routing::get(crate::image_opt::image_optimizer_handler),
            );
        }
        if !routes_by_path.contains_key("/_yav/og") {
            router = router.route("/_yav/og", axum::routing::get(crate::og::og_image_handler));
        }
        if !routes_by_path.contains_key("/_yav/vitals") {
            router = router.route(
                "/_yav/vitals",
                axum::routing::post(crate::telemetry::vitals_handler)
                    .get(crate::telemetry::vitals_summary_handler),
            );
        }

        for (path, method_handlers) in routes_by_path {
            let mut method_router = MethodRouter::new();
            let has_override_target = method_handlers
                .iter()
                .any(|(m, _)| matches!(*m, Method::PUT | Method::DELETE | Method::PATCH));
            let handlers_arc = Arc::new(method_handlers);

            for (method, handler) in handlers_arc.iter() {
                if *method == Method::POST && has_override_target {
                    continue;
                }
                let filter = method_to_filter(method);
                let h = handler.clone();
                method_router = method_router.on(
                    filter,
                    move |raw_params: axum::extract::RawPathParams, mut req: Request| {
                        let h = h.clone();
                        async move {
                            let mut params_map = std::collections::HashMap::new();
                            for (k, v) in raw_params.iter() {
                                params_map.insert(
                                    k.to_string(),
                                    yav_router::matcher::percent_decode_path_segment(v),
                                );
                            }
                            req.extensions_mut()
                                .insert(crate::action::RouteParams(params_map));
                            h.call(req).await
                        }
                    },
                );
            }

            // Support HTML Form method spoofing (`_method=PUT|DELETE|PATCH` or `X-HTTP-Method-Override`) on POST requests
            if has_override_target {
                let all_handlers = handlers_arc.clone();
                method_router = method_router.on(
                    MethodFilter::POST,
                    move |raw_params: axum::extract::RawPathParams, mut req: Request| {
                        let all_handlers = all_handlers.clone();
                        async move {
                            let mut params_map = std::collections::HashMap::new();
                            for (k, v) in raw_params.iter() {
                                params_map.insert(
                                    k.to_string(),
                                    yav_router::matcher::percent_decode_path_segment(v),
                                );
                            }
                            req.extensions_mut()
                                .insert(crate::action::RouteParams(params_map));

                            let (mut effective_method, mut req) =
                                resolve_post_method_override(req).await;
                            if effective_method != Method::POST {
                                *req.method_mut() = effective_method.clone();
                            }

                            if let Some((_, h)) =
                                all_handlers.iter().find(|(m, _)| *m == effective_method)
                            {
                                return h.call(req).await;
                            }

                            // Method not allowed on this route
                            effective_method = Method::POST;
                            let _ = effective_method;
                            let allow_list = all_handlers
                                .iter()
                                .map(|(m, _)| m.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            (
                                axum::http::StatusCode::METHOD_NOT_ALLOWED,
                                [(axum::http::header::ALLOW, allow_list)],
                                "Method Not Allowed",
                            )
                                .into_response()
                        }
                    },
                );
            }

            router = router.route(&path, method_router);
        }

        // Mount static files under /public if configured
        if let Some(ref static_dir) = self.static_dir {
            router = router.nest(
                "/public",
                crate::static_files::static_router(static_dir.clone()),
            );
        }

        // Fallback for unmatched routes: check if it matches a static asset in static_dir (e.g. /favicon.ico, /robots.txt), otherwise call custom not_found_handler or return standard 404 HTML error page
        let static_dir_fallback = self.static_dir.clone();
        let not_found_handler = self.not_found_handler.clone();
        router = router.fallback(move |req: axum::extract::Request| {
            let static_dir = static_dir_fallback.clone();
            let not_found = not_found_handler.clone();
            async move {
                if let Some(ref dir) = static_dir {
                    let path = req.uri().path();
                    if let Ok(file_path) = crate::static_files::resolve_static_path(dir, path) {
                        if file_path.is_file() {
                            if req.method() != axum::http::Method::GET
                                && req.method() != axum::http::Method::HEAD
                            {
                                return (
                                    axum::http::StatusCode::METHOD_NOT_ALLOWED,
                                    [(axum::http::header::ALLOW, "GET, HEAD")],
                                    "Method Not Allowed",
                                )
                                    .into_response();
                            }
                            let range = req
                                .headers()
                                .get(axum::http::header::RANGE)
                                .and_then(|v| v.to_str().ok());
                            let encoding = req
                                .headers()
                                .get(axum::http::header::ACCEPT_ENCODING)
                                .and_then(|v| v.to_str().ok());
                            if let Ok(mut resp) =
                                crate::static_files::serve_static_file_with_encoding(
                                    dir, path, range, encoding,
                                )
                                .await
                            {
                                if req.method() == axum::http::Method::HEAD {
                                    *resp.body_mut() = axum::body::Body::empty();
                                }
                                return resp;
                            }
                        }
                    }
                }
                if let Some(ref h) = not_found {
                    return h.call(req).await;
                }
                crate::error::YavError::NotFound.into_response()
            }
        });

        // Apply CORS layer if enabled
        if self.cors {
            if self.cors_origins.is_empty() {
                if crate::session::is_production_mode() {
                    // A wildcard CORS policy in production exposes cookies and
                    // authenticated endpoints to any website. Fail closed: a
                    // layer with no allowed origins emits no ACAO header, so
                    // browsers reject cross-origin reads by default.
                    tracing::warn!(
                        "CORS is enabled without an origin allowlist in production; cross-origin requests are rejected. Configure .cors_origins([...]) to allow specific origins."
                    );
                    router = router.layer(
                        tower_http::cors::CorsLayer::new()
                            .allow_methods([
                                Method::GET,
                                Method::POST,
                                Method::PUT,
                                Method::DELETE,
                                Method::PATCH,
                                Method::OPTIONS,
                            ])
                            .allow_headers([
                                axum::http::header::CONTENT_TYPE,
                                axum::http::header::AUTHORIZATION,
                                axum::http::header::ACCEPT,
                            ]),
                    );
                } else {
                    // Local development (HMR, separate dev ports) stays convenient.
                    router = router.layer(tower_http::cors::CorsLayer::permissive());
                }
            } else {
                let parsed_origins: Vec<axum::http::HeaderValue> = self
                    .cors_origins
                    .iter()
                    .filter_map(|o| axum::http::HeaderValue::from_str(o).ok())
                    .collect();
                let cors_layer = tower_http::cors::CorsLayer::new()
                    .allow_origin(parsed_origins)
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::DELETE,
                        Method::PATCH,
                        Method::OPTIONS,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                        axum::http::header::ACCEPT,
                    ]);
                router = router.layer(cors_layer);
            }
        }

        // Apply rate limiter layer if configured
        if let Some(ref limiter) = self.rate_limiter {
            router = router
                .layer(axum::middleware::from_fn(
                    crate::rate_limit::rate_limit_middleware,
                ))
                .layer(axum::Extension(limiter.clone()));
        }

        // Apply SessionSecret and session_middleware so FromRequestParts and handlers automatically commit cookies
        let secret = self
            .session_secret
            .or_else(|| {
                std::env::var("SESSION_SECRET")
                    .or_else(|_| std::env::var("YAV_SECRET"))
                    .ok()
                    .map(|s| crate::session::SessionSecret::new(s.into_bytes()))
            })
            .unwrap_or_else(|| {
                // Never abort the process (release uses `panic = "abort"`). Generate an
                // ephemeral cryptographically-random secret instead: sessions simply do
                // not survive a restart, which is safe and lets the server start.
                let mut bytes = [0u8; 32];
                if getrandom::getrandom(&mut bytes).is_ok() {
                    tracing::warn!(
                        "SESSION_SECRET is not set: using an ephemeral random secret. \
                         Sessions will be invalidated on every restart. Set SESSION_SECRET \
                         (at least 32 bytes) for persistent sessions."
                    );
                    crate::session::SessionSecret::new(bytes.to_vec())
                } else if crate::session::is_production_mode() {
                    // Startup-only fail-fast: no CSPRNG and no configured secret means we
                    // cannot safely sign sessions. This is never reached on a request path.
                    panic!(
                        "FATAL: SESSION_SECRET must be set when the system CSPRNG is unavailable \
                         in production mode."
                    );
                } else {
                    crate::session::SessionSecret::new(crate::session::DEFAULT_DEV_SESSION_SECRET)
                }
            });
        // CSRF validation must sit INSIDE the session layer so that the
        // `Session` extension (holding `_csrf_token`) is already populated.
        if self.csrf.enabled {
            router = router
                .layer(axum::middleware::from_fn(
                    crate::middleware::csrf_middleware,
                ))
                .layer(axum::Extension(self.csrf.clone()));
        }
        router = router
            .layer(axum::middleware::from_fn(
                crate::session::session_middleware,
            ))
            .layer(axum::Extension(self.session_config))
            .layer(axum::Extension(secret));

        // Apply standard security headers, etag caching, and request logging layers
        if self.etag {
            router = router.layer(axum::middleware::from_fn(etag_middleware));
        }

        router = router
            // Reject oversized request bodies before any handler/extractor sees them.
            // Preserves the 2 MiB form limit while giving every route a real global cap.
            .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
            .layer(axum::middleware::from_fn(security_headers_middleware))
            .layer(axum::middleware::from_fn(request_logger_middleware))
            .layer(axum::Extension(self.metrics.clone()))
            .layer(axum::Extension(self.state_context.clone()))
            .layer(axum::Extension(self.ssr_cache.clone()));

        if let Some(ref i18n) = self.i18n {
            router = router.layer(axum::Extension(i18n.clone()));
        }

        if self.compression {
            router = router.layer(tower_http::compression::CompressionLayer::new());
        }

        // Apply custom state layers and extensions
        for ext in self.extensions {
            router = ext(router);
        }

        // Keep tracing outermost so it observes every request, including 404s,
        // and can echo `traceparent` after all inner layers have run.
        if self.otel {
            router = router.layer(axum::middleware::from_fn(
                crate::otel::otel_tracing_middleware,
            ));
        }

        router
    }

    /// Prerender all static `GET` routes registered in this application into standalone `.html` files
    /// inside `output_dir`, along with `404.html`, `sitemap.xml`, and `robots.txt`.
    pub async fn export_static(
        &self,
        output_dir: impl AsRef<std::path::Path>,
        base_url: &str,
    ) -> std::io::Result<usize> {
        let out_path = output_dir.as_ref();
        std::fs::create_dir_all(out_path)?;

        let mut exported = 0usize;
        let mut static_paths = Vec::new();

        for route in &self.routes {
            if route.method != Method::GET
                || route.path.contains('{')
                || route.path.contains(':')
                || route.path.contains('*')
                || route.path == "/health"
            {
                continue;
            }
            let uri_path = if route.path.is_empty() {
                "/".to_string()
            } else {
                route.path.clone()
            };
            let mut req = Request::builder()
                .method(Method::GET)
                .uri(&uri_path)
                .body(axum::body::Body::empty())
                .unwrap_or_else(|_| Request::new(axum::body::Body::empty()));
            req.extensions_mut().insert(self.state_context.clone());
            req.extensions_mut().insert(self.ssr_cache.clone());
            if let Some(ref i18n) = self.i18n {
                req.extensions_mut().insert(i18n.clone());
            }
            let resp = route.handler.call(req).await;
            if let Ok(bytes) = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await {
                let target_file = if uri_path == "/" {
                    out_path.join("index.html")
                } else {
                    // Reject rather than rewrite unsafe route paths. A skipped
                    // route is visible in the returned count; silently mangling
                    // `..` could write outside the export directory on Windows.
                    let Some(dir) = safe_static_output_dir(out_path, &uri_path) else {
                        continue;
                    };
                    std::fs::create_dir_all(&dir)?;
                    dir.join("index.html")
                };
                std::fs::write(&target_file, &bytes)?;
                static_paths.push(uri_path);
                exported += 1;
            }
        }

        if let Some(ref nf) = self.not_found_handler {
            let mut req = Request::builder()
                .method(Method::GET)
                .uri("/404")
                .body(axum::body::Body::empty())
                .unwrap_or_else(|_| Request::new(axum::body::Body::empty()));
            if let Some(ref i18n) = self.i18n {
                req.extensions_mut().insert(i18n.clone());
            }
            let resp = nf.call(req).await;
            if let Ok(bytes) = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await {
                std::fs::write(out_path.join("404.html"), &bytes)?;
            }
        }

        if !static_paths.is_empty() {
            static_paths.sort();
            static_paths.dedup();
            let sitemap_xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{}\n</urlset>\n",
                static_paths
                    .iter()
                    .map(|r| {
                        let full = format!("{}{}", base_url.trim_end_matches('/'), r);
                        format!("  <url><loc>{full}</loc><priority>0.8</priority></url>")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            std::fs::write(out_path.join("sitemap.xml"), sitemap_xml)?;
        }

        Ok(exported)
    }

    /// Start listening on the configured host and port with a custom shutdown signal.
    pub async fn serve_with_shutdown<F>(self, shutdown_signal: F) -> std::io::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if let Ok(export_dir) = std::env::var("YAV_EXPORT_DIR") {
            let base_url = std::env::var("YAV_EXPORT_BASE_URL")
                .unwrap_or_else(|_| "https://example.com".to_string());
            self.export_static(&export_dir, &base_url).await?;
            return Ok(());
        }

        let addr = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        };
        let app = self.into_router();

        tracing::info!("Starting Yav server on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal)
        .await?;

        tracing::info!("Yav server shut down gracefully");
        Ok(())
    }

    /// Start listening on the configured host and port, running the server until SIGINT/SIGTERM or termination.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if binding to the TCP address fails or if an error occurs during serving.
    pub async fn serve(self) -> std::io::Result<()> {
        let shutdown_signal = default_shutdown_signal();
        self.serve_with_shutdown(shutdown_signal).await
    }
}

async fn default_shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut stream) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            stream.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Convert an [`axum::http::Method`] to an [`axum::routing::MethodFilter`].
pub fn method_to_filter(method: &Method) -> MethodFilter {
    match *method {
        Method::GET => MethodFilter::GET,
        Method::POST => MethodFilter::POST,
        Method::PUT => MethodFilter::PUT,
        Method::DELETE => MethodFilter::DELETE,
        Method::PATCH => MethodFilter::PATCH,
        Method::HEAD => MethodFilter::HEAD,
        Method::OPTIONS => MethodFilter::OPTIONS,
        Method::TRACE => MethodFilter::TRACE,
        // Unreachable for standard methods; fallback safely to GET
        _ => MethodFilter::GET,
    }
}

fn parse_override_method(raw: &str) -> Option<Method> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "PUT" => Some(Method::PUT),
        "DELETE" => Some(Method::DELETE),
        "PATCH" => Some(Method::PATCH),
        _ => None,
    }
}

/// Inspects an incoming `POST` request for HTTP method spoofing via:
/// 1. `X-HTTP-Method-Override` header
/// 2. `?_method=PUT|DELETE|PATCH` query parameter
/// 3. `_method=PUT|DELETE|PATCH` field in an `application/x-www-form-urlencoded` form body
async fn resolve_post_method_override(req: Request) -> (Method, Request) {
    // 1. Check X-HTTP-Method-Override header
    if let Some(hdr) = req
        .headers()
        .get("x-http-method-override")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(m) = parse_override_method(hdr) {
            return (m, req);
        }
    }

    // 2. Check ?_method=... in query string
    if let Some(q) = req.uri().query() {
        let query_map = yav_router::parse_query_string(q);
        if let Some(val) = query_map.get("_method") {
            if let Some(m) = parse_override_method(val) {
                return (m, req);
            }
        }
    }

    // 3. Check application/x-www-form-urlencoded body for _method=...
    let is_form = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false);

    if is_form {
        let (parts, body) = req.into_parts();
        if let Ok(bytes) = axum::body::to_bytes(body, 2 * 1024 * 1024).await {
            let mut detected = Method::POST;
            if let Ok(body_str) = std::str::from_utf8(&bytes) {
                let form_map = yav_router::parse_query_string(body_str);
                if let Some(val) = form_map.get("_method") {
                    if let Some(m) = parse_override_method(val) {
                        detected = m;
                    }
                }
            }
            let rebuilt = Request::from_parts(parts, axum::body::Body::from(bytes));
            return (detected, rebuilt);
        } else {
            let rebuilt = Request::from_parts(parts, axum::body::Body::empty());
            return (Method::POST, rebuilt);
        }
    }

    (Method::POST, req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::html_response;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[test]
    fn test_safe_static_output_dir_normal_routes() {
        let root = PathBuf::from("dist");
        assert_eq!(
            safe_static_output_dir(&root, "/about"),
            Some(root.join("about"))
        );
        assert_eq!(
            safe_static_output_dir(&root, "/docs/getting-started"),
            Some(root.join("docs").join("getting-started"))
        );
        assert_eq!(safe_static_output_dir(&root, "/"), Some(root.clone()));
    }

    #[test]
    fn test_safe_static_output_dir_rejects_traversal() {
        let root = PathBuf::from("dist");
        for attack in [
            "/../outside",
            "/docs/../../etc/passwd",
            "/%2e%2e/outside",
            "/%2e%2e%2foutside",
            "/..\\outside",
            "/C:Windows/System32",
            "/foo\\..\\..\\bar",
        ] {
            assert!(
                safe_static_output_dir(&root, attack).is_none(),
                "traversal payload was accepted: {attack}"
            );
        }
    }

    #[test]
    fn test_safe_static_output_dir_rejects_invalid_windows_names() {
        let root = PathBuf::from("dist");
        for bad in ["/con", "/NUL", "/aux", "/%00", "/bad%ZZ", "/%2e%2e"] {
            assert!(
                safe_static_output_dir(&root, bad).is_none(),
                "invalid component was accepted: {bad}"
            );
        }
        // Invalid filename characters are neutralised, trailing dots trimmed.
        assert_eq!(
            safe_static_output_dir(&root, "/file?"),
            Some(root.join("file_"))
        );
        assert_eq!(
            safe_static_output_dir(&root, "/hello.world."),
            Some(root.join("hello.world"))
        );
    }

    #[test]
    fn test_yav_app_builder_defaults() {
        let app = YavApp::new();
        assert_eq!(app.port, 3000);
        assert_eq!(app.host, "127.0.0.1");
        assert!(app.static_dir.is_none());
        assert!(app.routes.is_empty());
    }

    #[test]
    fn test_yav_app_builder_custom() {
        let app = YavApp::new()
            .port(8080)
            .host("0.0.0.0")
            .static_dir("public_assets");

        assert_eq!(app.port, 8080);
        assert_eq!(app.host, "0.0.0.0");
        assert_eq!(app.static_dir, Some(PathBuf::from("public_assets")));
    }

    #[test]
    fn test_app_route_registration() {
        let app = YavApp::new()
            .get("/", |_req| async { html_response(StatusCode::OK, "Home") })
            .post("/submit", |_req| async {
                html_response(StatusCode::CREATED, "Done")
            })
            .put("/update", |_req| async {
                html_response(StatusCode::OK, "Updated")
            })
            .delete("/item", |_req| async {
                html_response(StatusCode::NO_CONTENT, "")
            })
            .patch("/patch", |_req| async {
                html_response(StatusCode::OK, "Patched")
            });

        assert_eq!(app.routes.len(), 5);
        assert_eq!(app.routes[0].path, "/");
        assert_eq!(app.routes[0].method, Method::GET);
        assert_eq!(app.routes[1].path, "/submit");
        assert_eq!(app.routes[1].method, Method::POST);
    }

    #[test]
    fn test_into_router_creates_router() {
        let app = YavApp::new()
            .static_dir("public")
            .get("/", |_req| async { html_response(StatusCode::OK, "Index") })
            .post("/items", |_req| async {
                html_response(StatusCode::CREATED, "Created")
            });

        // into_router must successfully build the router without panicking
        let _router = app.into_router();
    }

    #[test]
    fn test_multiple_methods_on_same_path() {
        let app = YavApp::new()
            .get("/users", |_req| async {
                html_response(StatusCode::OK, "List")
            })
            .post("/users", |_req| async {
                html_response(StatusCode::CREATED, "Create")
            });

        // Must successfully merge multiple methods for the same path
        let _router = app.into_router();
    }

    #[tokio::test]
    async fn test_boxed_handler_execution() {
        let handler =
            BoxedHandler::new(|_req| async { html_response(StatusCode::OK, "hello from handler") });

        let req = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .expect("build request");

        let response = handler.call(req).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert_eq!(body_bytes.as_ref(), b"hello from handler");
    }

    #[tokio::test]
    async fn test_boxed_handler_from_zero_arg() {
        let handler = BoxedHandler::from_zero_arg(|| async {
            html_response(StatusCode::ACCEPTED, "zero args")
        });

        let req = Request::builder()
            .uri("/zero")
            .body(axum::body::Body::empty())
            .expect("build request");

        let response = handler.call(req).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn test_route_normalization_colon_to_curly() {
        let route = AppRoute::new("/users/:id/posts/*slug", Method::GET, |_req| async {
            html_response(StatusCode::OK, "ok")
        });
        assert_eq!(route.path, "/users/{id}/posts/{*slug}");
    }

    #[tokio::test]
    async fn test_built_in_healthz_routes() {
        use tower::ServiceExt;
        let app = YavApp::new();
        let router = app.into_router();

        let req = Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req_z = Request::builder()
            .uri("/healthz")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_z = router.clone().oneshot(req_z).await.unwrap();
        assert_eq!(resp_z.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limiter_middleware_in_app() {
        use tower::ServiceExt;
        let config = crate::rate_limit::RateLimitConfig::new()
            .limit_per_window(1)
            .window_duration(std::time::Duration::from_secs(60));

        let app = YavApp::new().rate_limit(config).get("/test", |_req| async {
            html_response(StatusCode::OK, "allowed")
        });

        let router = app.into_router();

        // 1st request -> 200 OK
        let req1 = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp1 = router.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // 2nd request -> 429 Too Many Requests
        let req2 = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp2 = router.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_built_in_metrics_endpoint() {
        use tower::ServiceExt;
        let app = YavApp::new().get("/ping", |_req| async {
            html_response(StatusCode::OK, "pong")
        });
        let router = app.into_router();

        // Call /ping
        let req_ping = Request::builder()
            .uri("/ping")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_ping = router.clone().oneshot(req_ping).await.unwrap();
        assert_eq!(resp_ping.status(), StatusCode::OK);

        // Call /metrics
        let req_metrics = Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_metrics = router.oneshot(req_metrics).await.unwrap();
        assert_eq!(resp_metrics.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(resp_metrics.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(json["total_requests"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_app_state_and_extension() {
        use tower::ServiceExt;

        #[derive(Clone)]
        struct DbConfig {
            connection_string: String,
        }

        let app = YavApp::new()
            .state(DbConfig {
                connection_string: "postgres://localhost/test".into(),
            })
            .get("/db", |req: Request| async move {
                let db = req
                    .extensions()
                    .get::<DbConfig>()
                    .expect("DbConfig must be in extensions");
                html_response(StatusCode::OK, db.connection_string.clone())
            });

        let router = app.into_router();
        let req = Request::builder()
            .uri("/db")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), b"postgres://localhost/test");
    }

    #[test]
    fn test_app_route_groups() {
        let app = YavApp::new()
            .get("/", |_req| async { html_response(StatusCode::OK, "Home") })
            .group("/api/v1", |api| {
                api.get("/users", |_req| async {
                    html_response(StatusCode::OK, "Users")
                })
                .post("/users", |_req| async {
                    html_response(StatusCode::CREATED, "Create User")
                })
            })
            .group("admin", |admin| {
                admin.get("/", |_req| async {
                    html_response(StatusCode::OK, "Admin Dashboard")
                })
            });

        let paths: Vec<&str> = app.routes.iter().map(|r| r.path.as_str()).collect();
        assert!(paths.contains(&"/"));
        assert!(paths.contains(&"/api/v1/users"));
        assert!(paths.contains(&"/admin"));
    }

    #[tokio::test]
    async fn test_app_openapi_helper() {
        let spec = crate::openapi::OpenApiSpec::new("Test API", "1.0.0").endpoint(
            "/items",
            "GET",
            "List items",
            "items",
        );

        let app = YavApp::new().openapi("/docs", spec);
        let router = app.into_router();

        // 1. Check openapi.json
        let req_json = Request::builder()
            .uri("/docs/openapi.json")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_json = router.clone().oneshot(req_json).await.unwrap();
        assert_eq!(resp_json.status(), StatusCode::OK);
        let body_json = axum::body::to_bytes(resp_json.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(std::str::from_utf8(&body_json)
            .unwrap()
            .contains("Test API"));

        // 2. Check Scalar viewer
        let req_scalar = Request::builder()
            .uri("/docs")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_scalar = router.clone().oneshot(req_scalar).await.unwrap();
        assert_eq!(resp_scalar.status(), StatusCode::OK);

        // 3. Check Swagger viewer
        let req_swagger = Request::builder()
            .uri("/docs/swagger")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_swagger = router.oneshot(req_swagger).await.unwrap();
        assert_eq!(resp_swagger.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_app_i18n_integration() {
        let i18n = yav_core::I18n::new("en")
            .supported_locales(["en", "es"])
            .add_translation("en", "hello", "Hello!")
            .add_translation("es", "hello", "¡Hola!");

        let app = YavApp::new()
            .i18n(i18n)
            .get("/greeting", |req: Request| async move {
                let ctx = crate::action::RouteContext::from_request(&req);
                html_response(StatusCode::OK, ctx.t("hello"))
            });

        let router = app.into_router();

        let req_es = Request::builder()
            .uri("/greeting")
            .header("accept-language", "es-ES,es;q=0.9")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_es = router.oneshot(req_es).await.unwrap();
        assert_eq!(resp_es.status(), StatusCode::OK);
        let body_es = axum::body::to_bytes(resp_es.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(std::str::from_utf8(&body_es).unwrap(), "¡Hola!");
    }

    #[tokio::test]
    async fn test_app_session_auto_commit_and_route_context() {
        let app = YavApp::new()
            .csrf_protection(crate::middleware::CsrfConfig::disabled())
            .session(crate::session::SessionConfig::new().cookie_name("_my_sid"))
            .session_secret(b"yav_app_test_secret_32_bytes_long!".to_vec())
            .post("/api/login", |req: Request| async move {
                let ctx = crate::action::RouteContext::from_request(&req);
                let session = ctx.session().expect("session must be initialized");
                session.set("user", "charlie");
                html_response(StatusCode::OK, "logged in")
            })
            .get("/api/me", |req: Request| async move {
                let ctx = crate::action::RouteContext::from_request(&req);
                let username = ctx
                    .session()
                    .and_then(|s| s.get("user"))
                    .unwrap_or_else(|| "guest".to_string());
                html_response(StatusCode::OK, username)
            });

        let router = app.into_router();

        // 1. Post to login
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/login")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp1 = router.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let set_cookie = resp1
            .headers()
            .get(axum::http::header::SET_COOKIE)
            .expect("Session must auto-commit Set-Cookie header");
        let cookie_str = set_cookie.to_str().unwrap();
        assert!(cookie_str.starts_with("_my_sid="));

        // 2. Query /api/me with cookie
        let cookie_val = cookie_str.split(';').next().unwrap();
        let req2 = Request::builder()
            .uri("/api/me")
            .header(axum::http::header::COOKIE, cookie_val)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp2 = router.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(std::str::from_utf8(&body2).unwrap(), "charlie");
    }

    #[tokio::test]
    async fn test_app_custom_middleware_and_not_found_handler() {
        let app = YavApp::new()
            .middleware(
                |req: axum::extract::Request, next: axum::middleware::Next| async move {
                    let mut resp = next.run(req).await;
                    resp.headers_mut().insert(
                        axum::http::HeaderName::from_static("x-custom-mw"),
                        axum::http::HeaderValue::from_static("applied"),
                    );
                    resp
                },
            )
            .not_found_handler(|_req| async {
                html_response(StatusCode::NOT_FOUND, "<h1>Custom 404</h1>")
            })
            .get("/ok", |_req| async {
                html_response(StatusCode::OK, "all good")
            });

        let router = app.into_router();

        // Check normal route has custom middleware
        let req1 = Request::builder()
            .uri("/ok")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp1 = router.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);
        assert_eq!(
            resp1
                .headers()
                .get("x-custom-mw")
                .unwrap()
                .to_str()
                .unwrap(),
            "applied"
        );

        // Check 404 route uses custom not_found_handler and has middleware
        let req2 = Request::builder()
            .uri("/does-not-exist")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp2 = router.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            resp2
                .headers()
                .get("x-custom-mw")
                .unwrap()
                .to_str()
                .unwrap(),
            "applied"
        );
        let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(std::str::from_utf8(&body2).unwrap(), "<h1>Custom 404</h1>");
    }

    #[tokio::test]
    async fn test_http_method_override_and_ctx_handlers() {
        let app = YavApp::new()
            .csrf_protection(crate::middleware::CsrfConfig::disabled())
            .get("/zero", || async {
                html_response(StatusCode::OK, "zero-arg")
            })
            .get_ctx("/users/:id", |ctx| async move {
                let id = ctx.param("id").unwrap_or("0");
                html_response(StatusCode::OK, format!("user:{id}"))
            })
            .delete_ctx("/items/:id", |ctx| async move {
                let id = ctx.param("id").unwrap_or("0");
                html_response(StatusCode::OK, format!("deleted:{id}"))
            })
            .put_ctx("/items/:id", |ctx| async move {
                let id = ctx.param("id").unwrap_or("0");
                html_response(StatusCode::OK, format!("updated:{id}"))
            });

        let router = app.into_router();

        // 1. Zero-arg handler
        let req_zero = Request::builder()
            .uri("/zero")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_zero = router.clone().oneshot(req_zero).await.unwrap();
        assert_eq!(resp_zero.status(), StatusCode::OK);

        // 2. Context handler
        let req_ctx = Request::builder()
            .uri("/users/42")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_ctx = router.clone().oneshot(req_ctx).await.unwrap();
        let body_ctx = axum::body::to_bytes(resp_ctx.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_ctx[..], b"user:42");

        // 3. Form POST with _method=DELETE
        let req_del = Request::builder()
            .method("POST")
            .uri("/items/99")
            .header(
                axum::http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(axum::body::Body::from("_method=DELETE&confirm=yes"))
            .unwrap();
        let resp_del = router.clone().oneshot(req_del).await.unwrap();
        assert_eq!(resp_del.status(), StatusCode::OK);
        let body_del = axum::body::to_bytes(resp_del.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_del[..], b"deleted:99");

        // 4. POST with X-HTTP-Method-Override: PUT
        let req_put = Request::builder()
            .method("POST")
            .uri("/items/77")
            .header("x-http-method-override", "PUT")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp_put = router.oneshot(req_put).await.unwrap();
        assert_eq!(resp_put.status(), StatusCode::OK);
        let body_put = axum::body::to_bytes(resp_put.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body_put[..], b"updated:77");
    }
}
