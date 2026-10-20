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
            let fut = f(req);
            Box::pin(async move { fut.await.into_response() })
        }))
    }

    /// Create a handler from a function taking no arguments.
    pub fn from_zero_arg<F, Fut, R>(f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse + 'static,
    {
        BoxedHandler(Arc::new(move |_req| {
            let fut = f();
            Box::pin(async move { fut.await.into_response() })
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

/// Trait for types convertible into a [`BoxedHandler`].
pub trait IntoBoxedHandler: Send + Sync + 'static {
    /// Convert `self` into a [`BoxedHandler`].
    fn into_boxed_handler(self) -> BoxedHandler;
}

impl IntoBoxedHandler for BoxedHandler {
    fn into_boxed_handler(self) -> BoxedHandler {
        self
    }
}

impl<F, Fut, R> IntoBoxedHandler for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse + 'static,
{
    fn into_boxed_handler(self) -> BoxedHandler {
        BoxedHandler::new(self)
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
    pub fn new(path: impl Into<String>, method: Method, handler: impl IntoBoxedHandler) -> Self {
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
            session_secret,
            session_config: crate::session::SessionConfig::default(),
            rate_limiter: None,
            metrics: crate::health::ServerMetrics::new(),
            fs_router: None,
            extensions: Vec::new(),
            i18n: None,
            not_found_handler: None,
        }
    }

    /// Set whether automatic ETag generation and 304 Not Modified caching is enabled.
    pub fn etag(mut self, enabled: bool) -> Self {
        self.etag = enabled;
        self
    }

    /// Enable or disable CORS headers.
    pub fn cors(mut self, enabled: bool) -> Self {
        self.cors = enabled;
        self
    }

    /// Set custom session signing secret key.
    pub fn session_secret(mut self, secret: impl Into<Vec<u8>>) -> Self {
        self.session_secret = Some(crate::session::SessionSecret::new(secret));
        self
    }

    /// Configure session settings (cookie name, TTL, path, SameSite, etc.).
    pub fn session(mut self, config: crate::session::SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// Set the port number.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the host address.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Configure the directory to serve static files from at `/public`.
    pub fn static_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(dir.into());
        self
    }

    /// Configure in-memory sliding window rate limiting.
    pub fn rate_limit(mut self, config: crate::rate_limit::RateLimitConfig) -> Self {
        self.rate_limiter = Some(crate::rate_limit::RateLimiter::new(config));
        self
    }

    /// Attach an existing [`RateLimiter`](crate::rate_limit::RateLimiter) instance.
    pub fn rate_limiter(mut self, limiter: crate::rate_limit::RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Provide a custom [`ServerMetrics`](crate::health::ServerMetrics) telemetry tracker.
    pub fn metrics(mut self, metrics: crate::health::ServerMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Register shared application state or database pool accessible to all handlers via `axum::extract::Extension<T>`.
    pub fn state<T: Clone + Send + Sync + 'static>(mut self, state: T) -> Self {
        self.extensions.push(Arc::new(move |router| {
            router.layer(axum::Extension(state.clone()))
        }));
        self
    }

    /// Register a custom extension or service layer on the router.
    pub fn extension<T: Clone + Send + Sync + 'static>(self, ext: T) -> Self {
        self.state(ext)
    }

    /// Register a custom middleware function (e.g. `|req, next| async { next.run(req).await }`).
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

    /// Configure a custom 404 Not Found fallback handler.
    pub fn not_found_handler(mut self, handler: impl IntoBoxedHandler) -> Self {
        self.not_found_handler = Some(handler.into_boxed_handler());
        self
    }

    /// Attach a [`FileSystemRouter`](yav_router::fs_router::FileSystemRouter) to the application for route reflection and discovery.
    pub fn with_fs_router(mut self, fs_router: yav_router::fs_router::FileSystemRouter) -> Self {
        self.fs_router = Some(fs_router);
        self
    }

    /// Attach an [`I18n`](yav_core::I18n) catalog for request locale resolution and translations.
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
            session_secret: self.session_secret.clone(),
            session_config: self.session_config.clone(),
            rate_limiter: self.rate_limiter.clone(),
            metrics: self.metrics.clone(),
            fs_router: self.fs_router.clone(),
            extensions: Vec::new(),
            i18n: self.i18n.clone(),
            not_found_handler: self.not_found_handler.clone(),
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
        self.extensions.extend(configured.extensions);
        self
    }

    /// Add a route with a custom path, HTTP method, and asynchronous handler.
    pub fn route(
        mut self,
        path: impl Into<String>,
        method: Method,
        handler: impl IntoBoxedHandler,
    ) -> Self {
        self.routes.push(AppRoute::new(path, method, handler));
        self
    }

    /// Register a `GET` route.
    pub fn get(self, path: impl Into<String>, handler: impl IntoBoxedHandler) -> Self {
        self.route(path, Method::GET, handler)
    }

    /// Register a `POST` route.
    pub fn post(self, path: impl Into<String>, handler: impl IntoBoxedHandler) -> Self {
        self.route(path, Method::POST, handler)
    }

    /// Register a `PUT` route.
    pub fn put(self, path: impl Into<String>, handler: impl IntoBoxedHandler) -> Self {
        self.route(path, Method::PUT, handler)
    }

    /// Register a `DELETE` route.
    pub fn delete(self, path: impl Into<String>, handler: impl IntoBoxedHandler) -> Self {
        self.route(path, Method::DELETE, handler)
    }

    /// Register a `PATCH` route.
    pub fn patch(self, path: impl Into<String>, handler: impl IntoBoxedHandler) -> Self {
        self.route(path, Method::PATCH, handler)
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

    /// Convert the configured `YavApp` into an Axum [`Router`](axum::Router).
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

        // Built-in health checks (/healthz, /readyz, and /metrics) if not overridden
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

        for (path, method_handlers) in routes_by_path {
            let mut method_router = MethodRouter::new();
            for (method, handler) in method_handlers {
                let filter = method_to_filter(&method);
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
            router = router.layer(tower_http::cors::CorsLayer::permissive());
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
        let secret = self.session_secret.unwrap_or_else(|| {
            if crate::session::is_production_mode() {
                panic!("FATAL: SESSION_SECRET must be explicitly set when running in production mode! Refusing to start with the default development secret.");
            }
            crate::session::SessionSecret::new(crate::session::DEFAULT_DEV_SESSION_SECRET)
        });
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
            .layer(axum::middleware::from_fn(security_headers_middleware))
            .layer(axum::middleware::from_fn(request_logger_middleware))
            .layer(axum::Extension(self.metrics.clone()));

        if let Some(ref i18n) = self.i18n {
            router = router.layer(axum::Extension(i18n.clone()));
        }

        // Apply custom state layers and extensions
        for ext in self.extensions {
            router = ext(router);
        }

        router
    }

    /// Start listening on the configured host and port with a custom shutdown signal.
    pub async fn serve_with_shutdown<F>(self, shutdown_signal: F) -> std::io::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        };
        let app = self.into_router();

        tracing::info!("Starting Yav server on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::html_response;
    use axum::http::StatusCode;
    use tower::ServiceExt;

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

    #[test]
    fn test_built_in_healthz_routes() {
        let app = YavApp::new();
        let _router = app.into_router();
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
}
