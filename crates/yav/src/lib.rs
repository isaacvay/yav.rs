//! # Yav — Modern Full-Stack Web Framework for Rust
//!
//! **Yav** is a batteries-included, server-side rendered (SSR) web framework combining the developer
//! ergonomics of **Next.js** with the integrated UI philosophy of **Flutter**, powered by **Axum**,
//! **Tokio**, and **Tailwind CSS**.
//!
//! Write your entire web application in type-safe, compiled Rust — without Node.js, npm, or JavaScript build tooling.
//!
//! ---
//!
//! ## Table of Contents
//!
//! 1. [Quick Start & Architecture](#quick-start--architecture)
//! 2. [Declarative UI with RSX & Native Control Flow](#declarative-ui-with-rsx--native-control-flow)
//! 3. [Zero-JS Motion, Spring Physics & Cursor Tracking](#zero-js-motion-spring-physics--cursor-tracking)
//! 4. [Multi-Theme Design System (8 Themes)](#multi-theme-design-system-8-themes)
//! 5. [Component Model & Props](#component-model--props)
//! 6. [HTTP Server, File Routing & Reverse Routing](#http-server-file-routing--reverse-routing)
//! 7. [State, Sessions & Typed Form Validation](#state-sessions--typed-form-validation)
//! 8. [Real-Time WebSockets Live Swap](#real-time-websockets-live-swap)
//! 9. [45+ UI Components & Icons](#45-ui-components--icons)
//! 10. [SEO, Sitemaps & Static Site Generation](#seo-sitemaps--static-site-generation)
//! 11. [Interactive API Documentation (OpenAPI / Scalar)](#interactive-api-documentation-openapi--scalar)
//! 12. [Modules and Prelude](#modules-and-prelude)
//!
//! ---
//!
//! ## Quick Start & Architecture
//!
//! Add `yav` and `tokio` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! yav = "0.1.6"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! Create your first application in `src/main.rs`:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = YavApp::new()
//!         .port(3000)
//!         .get("/", |_req| async {
//!             let page = rsx! {
//!                 <div class="min-h-screen bg-slate-950 text-white flex flex-col items-center justify-center p-6">
//!                     <h1 class="text-4xl font-extrabold tracking-tight mb-4">
//!                         "Welcome to Yav.rs"
//!                     </h1>
//!                     <p class="text-slate-400 max-w-md text-center mb-8">
//!                         "Fast, type-safe server-side rendering with compiled Rust and zero JS runtime overhead."
//!                     </p>
//!                     <button class="px-6 py-3 rounded-lg bg-indigo-600 hover:bg-indigo-500 font-semibold shadow-lg">
//!                         "Get Started"
//!                     </button>
//!                 </div>
//!             };
//!             html_response(StatusCode::OK, render_to_string(&page))
//!         });
//!
//!     app.serve().await?;
//!     Ok(())
//! }
//! ```
//!
//! ---
//!
//! ## Declarative UI with RSX & Native Control Flow
//!
//! The [`rsx!`] procedural macro provides an intuitive JSX-like authoring syntax compiled directly
//! into a zero-allocation, immutable Virtual DOM tree ([`core::Node`]).
//!
//! Since v0.1.4 you write standard Rust control flow directly in markup without closures:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! let user_name = "Alice";
//! let is_admin = true;
//! let is_active = true;
//! let notifications = ["Security alert", "Welcome gift"];
//!
//! let view = rsx! {
//!     <section class="dashboard">
//!         <h2>{ format!("Hello, {}!", user_name) }</h2>
//!
//!         // Native if / else control flow in RSX
//!         if is_admin {
//!             <span class="badge badge-admin">"Admin Panel"</span>
//!         } else {
//!             <span class="badge">"Member"</span>
//!         }
//!
//!         // Native for loop iteration
//!         <ul class="notification-list">
//!             for item in notifications.iter() {
//!                 <li class="notification-item">{ *item }</li>
//!             }
//!         </ul>
//!
//!         // Class directives & short boolean attributes
//!         <button
//!             class="btn"
//!             class:active={is_active}
//!             class:border-oxide=true
//!             disabled
//!         >
//!             "Action"
//!         </button>
//!     </section>
//! };
//! ```
//!
//! ### Automatic HTML Escaping & XSS Protection
//!
//! All interpolated strings, expressions, and attribute values within [`rsx!`] are automatically
//! context-aware HTML escaped using [`core::escape_html`] and [`core::escape_attribute`] via high-performance,
//! zero-allocation `Cow<'_, str>` fast paths. Raw unescaped HTML can only be explicitly rendered via [`core::Node::Raw`].
//!
//! ---
//!
//! ## Zero-JS Motion, Spring Physics & Cursor Tracking
//!
//! Yav provides rich client interactivity without hand-written JavaScript. Physical animations are calculated
//! with an exact harmonic oscillator solver (`solveSpring`) and executed on the browser's compositor thread:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//! use yav::ui::{CountUp, AnimatedBar, TypewriterCode, CursorSpotlight, TiltPlate, Magnetic};
//!
//! let view = rsx! {
//!     <div class="space-y-6">
//!         // Cursor tracking, 3D tilt & magnetic attraction
//!         <CursorSpotlight radius_px={450}>
//!             <TiltPlate max_deg={10.0} spotlight=true class="p-6 rounded-xl border">
//!                 <Magnetic strength={0.4}>
//!                     <button class="btn btn-primary">"Magnetic Button"</button>
//!                 </Magnetic>
//!             </TiltPlate>
//!         </CursorSpotlight>
//!
//!         // Viewport-triggered animated counter and gauge
//!         <CountUp to={124500.0} prefix="+" suffix=" req/s" duration_ms={1400} />
//!         <AnimatedBar target_pct={98.5} duration_ms={1200} class="h-2 bg-zinc-800" />
//!
//!         // Syntax-preserving typewriter code animation
//!         <TypewriterCode speed_ms={14} skip_on_click=true>
//!             <div>"fn main() { println!(\"Fast\"); }"</div>
//!         </TypewriterCode>
//!     </div>
//! };
//! ```
//!
//! ---
//!
//! ## Multi-Theme Design System (8 Themes)
//!
//! Yav includes 8 pre-built design themes (`editorial-blueprint`, `obsidian-minimal`, `nordic-paper`,
//! `cyber-terminal`, `solarized-lab`, `glass-aurora`, `light`, `dark`):
//!
//! ```rust,no_run
//! use yav::prelude::*;
//! use yav::ui::{Theme, ThemeSelector};
//!
//! rsx! {
//!     <head>
//!         { Theme::inject_all_themes_css() }
//!     </head>
//!     <body>
//!         <ThemeSelector default_theme="editorial-blueprint" />
//!     </body>
//! };
//! ```
//!
//! ---
//!
//! ## Component Model & Props
//!
//! Yav provides the [`#[component]`](component) attribute macro to define reusable, strongly-typed components:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! struct UserCardProps {
//!     username: String,
//!     email: String,
//!     active: bool,
//! }
//!
//! #[component]
//! fn UserCard(props: UserCardProps) -> Node {
//!     let status_color = if props.active { "text-emerald-500" } else { "text-rose-500" };
//!     rsx! {
//!         <Card class="p-4 shadow-sm border border-slate-800">
//!             <CardHeader>
//!                 <CardTitle>{ props.username }</CardTitle>
//!                 <CardDescription>{ props.email }</CardDescription>
//!             </CardHeader>
//!             <CardContent>
//!                 <p class={format!("text-xs font-semibold uppercase {}", status_color)}>
//!                     { if props.active { "Active Account" } else { "Suspended" } }
//!                 </p>
//!             </CardContent>
//!         </Card>
//!     }
//! }
//! ```
//!
//! ---
//!
//! ## HTTP Server, File Routing & Reverse Routing
//!
//! Yav's server layer is built directly upon **Axum** and **Tower**, inheriting enterprise-grade concurrency,
//! connection pooling, and asynchronous streaming.
//!
//! ### Reverse Routing with `url_for!`
//!
//! Generate parameterized URLs safely:
//!
//! ```rust
//! use yav::router::url_for;
//!
//! assert_eq!(url_for("/users/:id", &[("id", "42")]), "/users/42");
//! assert_eq!(url_for("/docs/*slug", &[("slug", "guide/quickstart")]), "/docs/guide/quickstart");
//! ```
//!
//! ### Handlers & Route Context
//!
//! Handlers receive a [`server::RouteContext`] allowing inspection of URL parameters, query parameters, headers,
//! client IP, and typed dependencies:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! let app = YavApp::new()
//!     // Dynamic URL parameters: /users/:id
//!     .get_ctx("/users/:id", |ctx: RouteContext| async move {
//!         let user_id = ctx.param("id").unwrap_or("unknown");
//!         HtmlPage::new(rsx! {
//!             <div><h1>{ format!("User Profile #{}", user_id) }</h1></div>
//!         })
//!     })
//!     // Query string parsing: /search?q=rust
//!     .get_ctx("/search", |ctx: RouteContext| async move {
//!         let query = ctx.query_val("q").unwrap_or("").to_string();
//!         HtmlPage::new(rsx! {
//!             <p>{ format!("Search results for: {query}") }</p>
//!         })
//!     });
//! ```
//!
//! ---
//!
//! ## State, Sessions & Typed Form Validation
//!
//! Yav provides cryptographically signed session cookies (HMAC-SHA256) and typed form parsing via [`server::FromFormData`]:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//! use yav::server::{FromFormData, FormErrors, FormData};
//!
//! #[derive(Debug)]
//! struct LoginForm {
//!     email: String,
//! }
//!
//! impl FromFormData for LoginForm {
//!     fn from_form(form: &FormData) -> Result<Self, FormErrors> {
//!         let mut errors = FormErrors::new();
//!         let email = form.get_or_empty("email");
//!         errors.require("email", email, "Email is required");
//!         if errors.has_errors() {
//!             Err(errors)
//!         } else {
//!             Ok(Self { email: email.to_string() })
//!         }
//!     }
//! }
//! ```
//!
//! ---
//!
//! ## Real-Time WebSockets Live Swap
//!
//! Push server-rendered HTML fragments to connected clients over WebSockets via [`server::WsBroadcast`]:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//! use yav::server::WsBroadcast;
//!
//! let hub = WsBroadcast::new(32);
//! let badge = rsx! { <span class="badge-online">"Online"</span> };
//! hub.send_swap("#status-badge", "innerHTML", &badge);
//! ```
//!
//! ---
//!
//! ## 45+ UI Components & Icons
//!
//! Yav embeds a complete UI design system crafted with Tailwind CSS utility classes and full WAI-ARIA
//! accessibility support:
//!
//! - **Primitives**: `Button`, `Input`, `Textarea`, `Select`, `Checkbox`, `Radio`, `Badge`, `Avatar`
//! - **Containers & Layouts**: `Card`, `Container`, `Stack`, `Grid`, `Separator`, `CodeBlock`
//! - **Cursor & Motion**: `CursorSpotlight`, `TiltPlate`, `Magnetic`, `CursorFollower`, `Motion`, `CountUp`, `AnimatedBar`, `TypewriterCode`
//! - **Themes**: `ThemeSelector`, `ThemeToggle`, `ThemePreset`
//! - **Navigation**: `Navbar`, `Sidebar`, `Breadcrumbs`, `Tabs`, `Pagination`
//! - **Overlays & Feedback**: `Dialog`, `Dropdown`, `Alert`, `Skeleton`, `ToastContainer`
//! - **Advanced Widgets**: Command Palette (⌘K) via `CommandDialog`, `Drawer`, `Carousel`, `Stepper`
//!
//! All components are re-exported through [`ui`] and the [`prelude`].
//!
//! ### The SVG Icon Engine
//!
//! Yav features an integrated zero-runtime-dependency icon catalog ([`icons`]) searchable in $O(\log N)$ time.
//! Only the **Classic** (Lucide) family (~1,900 variants) is compiled by default; enable the
//! `full-icons` feature (or `minimal-icons` / `system-icons`) to compile all ~13,600 variants.
//! The example below uses all three families and therefore assumes `full-icons`:
//!
//! ```rust
//! use yav::prelude::*;
//!
//! let view = rsx! {
//!     <div class="flex items-center gap-3">
//!         // Declarative <Icon /> component in RSX
//!         <Icon name="classic:check-circle" class="text-emerald-500" />
//!
//!         // MynaUI modern regular & solid icons (Minimal)
//!         { render_icon("minimal:bell", IconSize::Md, Some("text-amber-500"), None) }
//!
//!         // Phosphor bold, duotone & fill icons (System)
//!         { render_icon("system:bold:shield-check", IconSize::Lg, Some("text-indigo-500"), None) }
//!     </div>
//! };
//! ```
//!
//! ---
//!
//! ## SEO, Sitemaps & Static Site Generation
//!
//! Configure complete HTML `<head>` metadata, OpenGraph tags, Twitter cards, and Schema.org JSON-LD structured data
//! via [`core::PageMeta`]:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! let meta = PageMeta::new()
//!     .title("High-Performance Rust SSR | Yav.rs")
//!     .description("Build lightning-fast web applications with Rust and Tailwind CSS.")
//!     .canonical("https://yav.rs")
//!     .open_graph("Yav Framework", "Modern SSR for Rust", "https://yav.rs/og.png")
//!     .twitter_card("summary_large_image");
//! ```
//!
//! Since v0.1.4 you can pre-render your entire application to static HTML files for CDN deployment:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//! use std::path::Path;
//!
//! # #[tokio::main]
//! # async fn main() -> std::io::Result<()> {
//! let app = YavApp::new()
//!     .get("/", |_req| async { html_response(StatusCode::OK, "<h1>Home</h1>") })
//!     .get("/docs", |_req| async { html_response(StatusCode::OK, "<h1>Docs</h1>") });
//!
//! let exported_count = app.export_static(Path::new("dist"), "https://yav.rs").await?;
//! println!("exported {exported_count} pages");
//! # Ok(())
//! # }
//! ```
//!
//! Or use the CLI: `yav export --output dist --base-url https://yav.rs`.
//!
//! ---
//!
//! ## Interactive API Documentation (OpenAPI / Scalar)
//!
//! Expose OpenAPI 3.1 specifications and modern interactive documentation with a single builder method:
//!
//! ```rust,no_run
//! use yav::prelude::*;
//!
//! let spec = OpenApiSpec::new("Yav Storefront API", "1.0.0")
//!     .endpoint("/api/products", "GET", "List all active catalog products", "products")
//!     .endpoint("/api/orders", "POST", "Submit a new customer order", "orders");
//!
//! let app = YavApp::new()
//!     // Automatically mounts:
//!     // - GET /docs             -> Modern Scalar API viewer
//!     // - GET /docs/swagger     -> Classic Swagger UI viewer
//!     // - GET /docs/openapi.json-> OpenAPI 3.1 JSON specification
//!     .openapi("/docs", spec);
//! ```
//!
//! ---
//!
//! ## Modules and Prelude
//!
//! Yav is organized as a modular suite of crates, all accessible directly through this top-level crate:
//!
//! | Module | Underlying Crate | Description |
//! |---|---|---|
//! | [`core`] | `yav-core` | Virtual DOM, [`core::Node`], [`signals`], [`state_machine`], [`collab`] (CRDTs & `LexoRank`), escaping, i18n, and context |
//! | [`macros`] | `yav-macros` | Procedural macros: [`rsx!`], [`#[component]`](component), [`#[derive(FromFormData)]`](FromFormData), [`#[derive(ApiSchema)]`](ApiSchema), [`#[server]`](server) |
//! | [`router`] | `yav-router` | File-based routing, `@slot` parallel routes, `(.)` intercepting routes, `[[...slug]]`, and table resolution |
//! | [`server`] | `yav-server` | Axum HTTP server, [`auth`], [`db`], [`email`], [`jobs`], [`image_opt`], [`flags`], [`ai`], [`billing`], [`storage`], [`geo`], [`adapters`] |
//! | [`ui`] | `yav-ui` | 60+ accessible UI components, [`charts`], [`datagrid`], [`kanban`], [`virtual_list`], [`flow_canvas`], [`rich_editor`] |
//! | [`icons`] | `yav-icons` | ~1,900 Classic icons by default, ~13,600 with `full-icons` (Lucide, MynaUI, Phosphor) |
//! | [`build`] | `yav-build` | Asset hashing, GFM Markdown + syntax highlighting, [`search`] (BM25), [`minifier`], and Tailwind compiler |
//!
//! For standard application development, simply import the [`prelude`]:
//!
//! ```rust
//! use yav::prelude::*;
//! ```

/// Core types: Virtual DOM ([`core::Node`]), HTML escaping, context propagation, and i18n.
pub use yav_core as core;

/// Request-scoped application state container (compatibility alias).
///
/// Yav has no dedicated `AppState` struct: request-scoped state actually lives in a
/// `yav_core::context::ContextMap`, injected via `provide_context` (or `YavApp::state`)
/// and read back with `use_state::<T>()` / `use_context::<T>()`. This alias points the
/// documented `AppState` name at that real container so it is not a phantom type; it
/// does not introduce a second, separate global state store.
pub type AppState = yav_core::context::ContextMap;

/// Hidden re-export for proc macro hygiene — allows `rsx!` to reference
/// `::yav::yav_core::Node` in downstream crates that don't directly depend on `yav-core`.
#[doc(hidden)]
pub extern crate yav_core;

/// Procedural macros: [`rsx!`], [`#[component]`](component), [`auto_routes!`](auto_routes),
/// [`include_rsx!`](include_rsx), [`include_yav!`](include_yav), [`include_routes!`](include_routes),
/// and [`yav_component!`](yav_component).
pub use yav_macros as macros;

// Direct macro re-exports for root-level ergonomics
pub use yav_macros::{
    auto_routes, component, include_routes, include_rsx, include_yav, rsx, server, t, t_strict,
    typed_routes, yav_component, ApiSchema, FromFormData,
};

/// Fine-grained reactive signals (`Signal<T>`, `Computed<T>`, `SignalScope`).
pub use yav_core::signals;

/// Deterministic Statecharts (`StateMachine`) & Saga compensating workflows (`SagaWorkflow`).
pub use yav_core::state_machine;

/// Real-time CRDT collaboration (`LwwRegister`, `LwwMap`), `PresenceRoom`, and `LexoRank`.
pub use yav_core::collab;

/// Local-first and offline-first IndexedDB synchronization protocol (`OfflineMutation`, `SyncBatch`).
pub use yav_core::local_first;

/// Zero-overhead WebAssembly client islands (`WasmIsland`).
pub use yav_core::wasm;

/// File-based and programmatic routing system with path pattern matching.
pub use yav_router as router;

/// HTTP server engine, Axum SSR integration, sessions, cookies, forms, and middleware.
pub use yav_server as server;

/// Feature-gated integrations from the competitive roadmap. The module is
/// always present but only contains submodules for enabled Cargo features.
/// See `docs/opt-in-integrations.md` for the list and their boundaries.
pub use yav_server::roadmap;

/// Integrated authentication, JWT, OAuth2/PKCE, PBKDF2 password hashing, and TOTP 2FA.
pub use yav_server::auth;

/// Integrated SQL migration runner, parameterized query builder, and embedded store.
pub use yav_server::db;

/// Transactional email builder rendering RSX components to HTML + plain-text fallback.
pub use yav_server::email;

/// Background job queue, retry backoff runner, and recurring cron scheduler.
pub use yav_server::jobs;

/// Dynamic image optimization and SVG LQIP blur placeholder generator (`/_yav/image`).
pub use yav_server::image_opt;

/// Dynamic OpenGraph social card image generator (`/_yav/og`).
pub use yav_server::og;

/// Real-time Core Web Vitals telemetry collector and reporter (`/_yav/vitals`).
pub use yav_server::telemetry;

/// Enterprise feature flags, B2B multi-tenancy, ABAC policy engine, and hash-chained audit logs.
pub use yav_server::flags;

/// Native AI SDK, streaming UI/text over SSE, JSON-Schema tool calling, and hybrid RAG.
pub use yav_server::ai;

/// Exact integer money, penny-preserving allocation, Stripe/Svix webhook verification, and idempotency.
pub use yav_server::billing;

/// S3/R2-compatible object storage and HMAC-SHA256 presigned upload/download URLs.
pub use yav_server::storage;

/// Geospatial Haversine distance, polygon geofencing, and proximity spatial indexing.
pub use yav_server::geo;

/// Pluggable infrastructure driver traits (`DatabaseAdapter`, `CacheAdapter`, `StorageAdapter`, `LlmAdapter`, `MailAdapter`).
pub use yav_server::adapters;

/// Partial Prerendering (`PprPage`, `PprSlot`) and server component fragment caching (`ComponentCache`).
pub use yav_server::ppr;

/// Passwordless WebAuthn / Passkeys registration and authentication challenges (`PasskeyManager`).
pub use yav_server::passkeys;

/// OpenTelemetry (OTLP) and W3C TraceContext distributed tracing (`OtelTracer`, `TraceParent`).
pub use yav_server::otel;

/// Full-stack E2E `TestClient` with automatic cookie jar persistence and CSS-like `DomAssert`.
pub use yav_server::testing;

/// Model Context Protocol (MCP) JSON-RPC 2.0 server (`McpServer`, `McpResource`, `McpPrompt`).
pub use yav_server::mcp;

/// Native vector PDF 1.4 document generator (`PdfDocument`) and RFC 4180 CSV/TSV exporter (`CsvBuilder`).
pub use yav_server::export_docs;

/// Reactive live query invalidation and Change Data Capture (CDC) server push (`LiveQueryManager`).
pub use yav_server::live_query;

/// Native Proof-of-Work anti-bot shield (`PowShield`, `PowChallenge`) and invisible honeypots.
pub use yav_server::shield;

/// Zero-dependency GraphQL execution engine, introspection schema, and GraphiQL IDE (`GraphQlSchema`).
pub use yav_server::graphql;

/// IP geolocation context extraction, EU VAT tax rules, and multi-currency pricing (`GeoIpContext`).
pub use yav_server::geo_ip;

/// Micro-frontends gateway, module federation, and parallel remote slot stitching (`FederationGateway`).
pub use yav_server::federation;

/// Pure Rust Vector Barcode 128 and ISO/IEC 18004 QR Code SVG generators (`Barcode128`, `QrCode`).
pub use yav_server::barcode;

/// Automated production security auditor, OWASP header grading, and W3C CSP violation reporting (`SecurityAuditor`, `CspReporter`).
pub use yav_server::security_audit;

/// Circuit breaker, bulkhead and adaptive concurrency limiting (`CircuitBreaker`, `AdaptiveConcurrencyLimiter`).
pub use yav_server::resilience;

/// Transactional outbox pattern and reliable asynchronous event bus (`TransactionalOutbox`).
pub use yav_server::outbox;

/// Multi-region read/write database replica pool and router (`DbReplicaPool`).
pub use yav_server::replica_pool;

/// Pure-Rust WebP container packing and image downsampling (`ImagePipeline`, `WebPEncoder`).
///
/// AVIF encoding is not supported: `image_pipeline` sniffs/parses AVIF input but only
/// produces WebP containers. See the module docs for the exact pipeline stages.
pub use yav_server::image_pipeline;

/// High-level image-processing facade (format sniffing → metadata → downsampling →
/// WebP container framing). Re-exported so `yav::ImagePipeline` resolves.
/// AVIF output is not supported.
pub use yav_server::image_pipeline::ImagePipeline;

/// Zero-dependency vector PDF signer and digital watermarking (`PdfSigner`, `PdfWatermark`).
pub use yav_server::pdf_signer;

/// Multi-tier distributed cache with probabilistic early expiration (XFetch).
pub use yav_server::distributed_cache;

/// Distributed locks and multi-node leader election (`DistributedLock`, `LeaderElector`).
pub use yav_server::distributed_lock;

/// Relationship-based access control (ReBAC) graph (`RebacGraph`).
pub use yav_server::rebac;

/// Built-in Prometheus metrics visualizer and Grafana-ready dashboard (`render_metrics_dashboard_html`).
pub use yav_server::metrics_dashboard;

/// Real-time TurboStream DOM streaming and mutations (`TurboStream`, `TurboAction`).
pub use yav_server::turbo_stream;

/// Append-only Event Sourcing and CQRS projections (`EventStore`, `DomainEvent`).
pub use yav_server::event_sourcing;

/// Resumable chunked uploads conforming to Tus Protocol v1.0.0 (`ChunkedUploader`).
pub use yav_server::resumable_upload;

/// Native WebPush notifications and VAPID signature headers (`WebPushNotification`, `VapidSigner`).
pub use yav_server::web_push;

/// Official UI component library with 60+ accessible, pre-styled Tailwind components.
pub use yav_ui as ui;

/// Server-rendered SVG charts (`LineChart`, `BarChart`, `DonutChart`, `Sparkline`).
pub use yav_ui::charts;

/// Enterprise server-rendered DataGrid with filtering, sorting, selection, and pagination.
pub use yav_ui::datagrid;

/// Drag-and-drop multi-column Kanban board with LexoRank ordering.
pub use yav_ui::kanban;

/// Virtualized scroll list capable of rendering 100,000+ rows at 60 FPS.
pub use yav_ui::virtual_list;

/// Interactive node-graph and workflow canvas with SVG cubic Bézier edges.
pub use yav_ui::flow_canvas;

/// Rich-text block editor (`RichTextEditor`) and AI streaming chat UI (`AiChat`).
pub use yav_ui::rich_editor;

/// Web font optimization and zero-CLS fallback metrics (`GoogleFont`, `LocalFont`).
pub use yav_ui::font;

/// Interactive floating DevTools HUD (`DevTools`) for inspecting routes, signals, SSR timing, and Web Vitals.
pub use yav_ui::devtools;

/// Monthly calendar grid (`Calendar`) and appointment slot picker (`Scheduler`).
pub use yav_ui::calendar;

/// Reactive Excel-like spreadsheet (`Spreadsheet`) with live `=SUM()`, `=AVG()`, `=MIN()`, `=MAX()` formula engine.
pub use yav_ui::spreadsheet;

/// Side-by-side / unified Git code diff viewer (`DiffViewer`) and custom HTML5 media player (`MediaPlayer`).
pub use yav_ui::media_diff;

/// Web ANSI terminal emulator with live log streaming and prompt support (`Terminal`).
pub use yav_ui::terminal;

/// Dynamic form generator from JSON Schema and OpenAPI specifications (`DynamicForm`).
pub use yav_ui::form_builder;

/// Dual-pane Markdown editor with split-view live preview and formatting toolbar (`MarkdownEditor`).
pub use yav_ui::markdown_editor;

/// Advanced SVG charts: Activity calendar Heatmaps, Sankey flow diagrams, and multi-metric Radar charts (`Heatmap`, `SankeyChart`, `RadarChart`).
pub use yav_ui::charts_advanced;

/// Interactive organizational chart and family tree SVG component (`OrgChart`).
pub use yav_ui::org_chart;

/// Interactive audio synthesizer and waveform visualizer (`AudioWaveform`).
pub use yav_ui::audio_waveform;

/// Native OpenTelemetry logtail live inspector (`LogViewer`).
pub use yav_ui::log_viewer;

/// Collaborative whiteboard and vector drawing canvas (`Whiteboard`).
pub use yav_ui::whiteboard;

/// Natural language AI filter bar converting conversational text to structured queries (`AiFilterBar`).
pub use yav_ui::ai_filter_bar;

/// Built-in icon system. Classic (Lucide) ~1,900 icons are compiled by
/// default; enable `full-icons` for the ~13,600-variant Lucide/MynaUI/Phosphor
/// catalog. See [`icons::ICON_COUNT`] for the compiled count.
pub use yav_icons as icons;

/// Build pipeline, asset fingerprinting, full-text search index, minifier, and Tailwind CSS integration.
pub use yav_build as build;

/// Full-text BM25 search indexer with `<mark>` snippet extraction.
pub use yav_build::search;

/// Zero-dependency HTML and CSS production minifier.
pub use yav_build::minifier;

/// Progressive Web App (PWA) `manifest.webmanifest` and offline-first Service Worker (`sw.js`) generator.
pub use yav_build::pwa;

/// Phonetic & fuzzy search engine (`damerau_levenshtein`, `soundex`, `FuzzySearchIndex`).
pub use yav_build::fuzzy_search;

/// Vector map with Mercator projection and GPS markers (`VectorMap`).
pub use yav_ui::vector_map;

/// Interactive Gantt chart and project roadmap (`GanttChart`).
pub use yav_ui::gantt_chart;

/// Vector PDF viewer and annotator (`PdfViewer`).
pub use yav_ui::pdf_viewer;

/// Spotlight guided tour and onboarding walkthrough (`SpotlightTour`).
pub use yav_ui::spotlight_tour;

/// Live audio & voice memo recorder (`MediaRecorder`).
pub use yav_ui::media_recorder;

/// Pure-Rust 3D wireframe and mesh visualizer (`ModelViewer3D`).
pub use yav_ui::model_viewer_3d;

/// Interactive code playground and execution REPL (`CodePlayground`).
pub use yav_ui::code_playground;

/// Interactive JSON and schema tree visualizer (`JsonTreeViewer`).
pub use yav_ui::json_tree;

/// Pure SVG hexagonal coordinate grid and spatial analytics (`HexGrid`).
pub use yav_ui::hex_grid;

/// Handwritten touch and mouse signature capture pad (`SignaturePad`).
pub use yav_ui::signature_pad;

/// Chronological event timeline and audit trail visualizer (`Timeline`).
pub use yav_ui::timeline;

/// Interactive before/after image comparison slider (`ImageCompareSlider`).
pub use yav_ui::image_compare;

/// GDPR and ePrivacy cookie consent banner and preference center (`CookieConsent`).
pub use yav_ui::cookie_consent;

/// Mock API server and deterministic fake data generator (`MockServer`, `MockFaker`).
pub use yav_build::mock_server;

/// Static i18n key extractor and translation catalog linter (`lint_catalog`, `scan_source_code`).
pub use yav_build::i18n_lint;

/// Convenience prelude — importing this brings all standard Yav types, macros, and components into scope.
///
/// # Example
/// ```rust
/// use yav::prelude::*;
/// ```
pub mod prelude {
    // Core Virtual DOM, Escaping, Context, I18n, Signals, Statecharts, Saga & CRDT Collaboration
    pub use yav_core::{
        create_memo, create_signal, document_shell, document_shell_with_lang, escape_attribute,
        escape_html, escape_url, extract_head_templates, provide_context, render_to_buf,
        render_to_string, use_context, use_context_or_default, use_state, Attribute,
        AttributeValue, Children, CollaboratorPresence, Component, Computed, ContextMap, CrdtClock,
        Element, Head, HeadProps, I18n, IntoAttributeValue, IntoNode, IntoPropValue, Layout,
        LexoRank, LwwMap, LwwRegister, Node, PageMeta, PresenceRoom, ReadSignal, SagaOutcome,
        SagaWorkflow, Signal, SignalScope, StateMachine, WriteSignal, YAV_MICRO_ACTIONS_SCRIPT,
        YAV_SIGNALS_RUNTIME_SCRIPT,
    };

    // Procedural Macros (including #[derive(FromFormData)], #[derive(ApiSchema)], #[server])
    pub use yav_macros::{
        auto_routes, component, include_routes, include_rsx, include_yav, rsx, server, t, t_strict,
        typed_routes, yav_component, ApiSchema, FromFormData,
    };

    // Routing types and helpers
    pub use yav_router::prelude::*;

    // Server, Auth, DB, Email, Jobs, Image Optimizer, OG Generator, Telemetry, Flags, AI, Billing, Storage, Geo, PPR, Passkeys, OTel, Testing, MCP, Export Docs, Forms, Flash, SEO, Streaming, Session, Cache, WebSockets
    #[allow(deprecated)] // legacy adapter aliases keep the public API compatible
    pub use yav_server::{
        classify_rating,
        cosine_similarity,
        csp_report_handler,
        csrf_field,
        csv_response,
        // Resilience, Outbox, Replicas, Images, PDF Signer
        detect_image_format,
        extract_geo_context,
        generate_blur_placeholder_from_bytes,
        generate_blur_svg_data_uri,
        generate_blur_svg_document_from_bytes,
        generate_og_svg,
        global_component_cache,
        global_csp_reporter,
        global_live_queries,
        global_tracer,
        global_vitals,
        graphiql_html,
        graphql_handler,
        html_response,
        image_optimizer_handler,
        is_allowlisted_redirect,
        json_response,
        // Server additions: Live Queries, PoW Shield, GraphQL, GeoIP/VAT, Federation, Barcode/QR, Security Audit
        live_query_sse_handler,
        metrics_dashboard_handler,
        not_found,
        og_image_handler,
        og_image_url,
        optimize_image_bytes,
        optimized_image_url,
        otel_tracing_middleware,
        parse_image_metadata,
        parse_tus_metadata,
        pdf_response,
        pow_client_script,
        ppr_placeholder,
        reciprocal_rank_fusion,
        redirect,
        redirect_allowlisted,
        render_metrics_dashboard_html,
        safe_redirect,
        scalar_ui_handler,
        sse_channel,
        sse_response,
        stream_text_sse,
        stream_ui_fragment,
        streaming_html,
        swagger_ui_handler,
        verify_csrf_token,
        vitals_handler,
        vitals_summary_handler,
        ws_node_frame,
        ws_swap_frame,
        yav_passkey_script,
        yav_vitals_script,
        ActionResponse,
        ActionResult,
        AdaptiveConcurrencyLimiter,
        AiTool,
        ApiEndpoint,
        AuditEvent,
        AuditTrail,
        AuthUser,
        Barcode128,
        BreakerState,
        Bulkhead,
        CacheAdapter,
        CacheEntry,
        // Distributed Cache, Mutex Locks, ReBAC & Metrics Dashboard
        CacheItem,
        CachePolicy,
        CacheStats,
        ChatMessage,
        ChatRole,
        ChunkedUploader,
        CircuitBreaker,
        CircuitBreakerConfig,
        ComponentCache,
        ConcurrencyGuard,
        Conversation,
        CspNonce,
        CspReporter,
        CspViolation,
        CsrfConfig,
        CsrfToken,
        CsvBuilder,
        Currency,
        DatabaseAdapter,
        DbReplicaPool,
        DistributedCache,
        DistributedLock,
        DomAssert,
        DomTag,
        DomainEvent,
        EmailMessage,
        EmbeddedDatabaseAdapter,
        EmbeddedStore,
        EventStore,
        EventStoreError,
        FeatureFlag,
        FederationGateway,
        FederationRoute,
        FederationSlot,
        FlagEngine,
        Flash,
        FlashLevel,
        FlashMessage,
        FormAction,
        FormData,
        FormErrors,
        FromFormData,
        GeoIpContext,
        GeoPoint,
        GeoPolygon,
        GraphQlError,
        GraphQlField,
        GraphQlObject,
        GraphQlRequest,
        GraphQlResponse,
        GraphQlSchema,
        GraphQlType,
        HeaderMap,
        HealthStatus,
        Honeypot,
        HtmlFragment,
        HtmlPage,
        HtmxRequest,
        IdempotencyStore,
        IdempotentRecord,
        ImageFormat,
        ImageMetadata,
        JobId,
        JobQueue,
        JobStatus,
        JsonRpcRequest,
        JsonRpcResponse,
        JwtClaims,
        JwtManager,
        JwtValidation,
        LeaderElector,
        LiveEvent,
        LiveQueryManager,
        LiveQueryStreamParams,
        LlmAdapter,
        Loader,
        LocalLlmAdapter,
        LockCoordinator,
        MailAdapter,
        Mailer,
        McpPrompt,
        McpResource,
        McpServer,
        MemoryCacheAdapter,
        MemoryL1Store,
        MemoryMailAdapter,
        MemoryStorageAdapter,
        Method,
        MetricStats,
        MetricsSnapshot,
        Migration,
        Migrator,
        Money,
        OAuthConfig,
        OAuthProvider,
        ObjectMeta,
        ObjectStorage,
        OgImageParams,
        OpenApiSpec,
        Order,
        OtelTracer,
        OutboxMessage,
        OutboxStats,
        OutboxStatus,
        PasskeyAuthOptions,
        PasskeyCredential,
        PasskeyManager,
        PasskeyRegistrationOptions,
        PasswordHasher,
        PdfDocument,
        PdfSignature,
        PdfSigner,
        PdfTableRow,
        PdfWatermark,
        PolicyEngine,
        PolicyResource,
        PolicySubject,
        PowChallenge,
        PowShield,
        PprPage,
        PprSlot,
        PresignedUrlSigner,
        Projection,
        PushSubscription,
        QrCode,
        QueryBuilder,
        RateLimitConfig,
        RateLimitStatus,
        RateLimiter,
        RawRgbImage,
        RebacGraph,
        RelationshipTuple,
        ReplicaNode,
        RobotsTxt,
        RouteContext,
        RouteTarget,
        SameSite,
        SecurityAuditReport,
        SecurityAuditor,
        SecurityGrade,
        SecurityHeadersConfig,
        ServerMetrics,
        ServiceAdapters,
        Session,
        SessionConfig,
        SessionSecret,
        Sitemap,
        SitemapEntry,
        SpanRecord,
        SpatialIndex,
        SpatialMatch,
        SseMsg,
        SsrCache,
        StatusCode,
        StorageAdapter,
        StreamingHtml,
        Suspense,
        TaxBreakdown,
        TenantContext,
        TestClient,
        TestResponse,
        ToolRegistry,
        Totp,
        TraceParent,
        TransactionalOutbox,
        // TurboStream, Event Sourcing, Resumable Uploads & WebPush
        TurboAction,
        TurboMessage,
        TurboStream,
        TusError,
        UploadSession,
        UploadedFile,
        UsageMeter,
        VapidSigner,
        VatRule,
        VectorIndex,
        VitalsCollector,
        VitalsSummary,
        WebPEncoder,
        WebPushNotification,
        WebVitalMetric,
        WebhookVerifier,
        WsBroadcast,
        YavApp,
        TUS_RESUMABLE_VERSION,
        YAV_TURBO_STREAM_SCRIPT,
    };

    // UI Components, Charts, DataGrid, Shadcn Widgets, SEO Metadata, Toast, Islands, Multi-Theme System, DevTools, Calendar, Spreadsheet, DiffViewer, MediaPlayer, 3D Mesh, Vector Map, Gantt, Audio, REPL, Tour, PDF, JSON Tree, HexGrid, SignaturePad, Timeline, ImageCompare, CookieConsent
    pub use yav_ui::prelude::*;

    // Built-in Icon System
    pub use yav_icons::{render_icon, Icon, IconFamily, IconProps, IconSize, IconStyle};

    // Markdown, Syntax Highlighting, Full-Text Search, Fuzzy Search, Mock Server, i18n Linter, Minifier & PWA Generator
    pub use yav_build::{
        damerau_levenshtein, generate_service_worker, generate_webmanifest, highlight_code_html,
        lint_catalog, load_markdown_dir, minify_css, minify_html, parse_markdown,
        pwa_register_script, scan_source_code, soundex, suggest_did_you_mean, FuzzyMatch,
        FuzzySearchIndex, I18nKeyOccurrence, I18nReport, MarkdownDoc, MockEndpoint, MockFaker,
        MockServer, PwaConfig, SearchDoc, SearchHit, SearchIndex, TocHeading,
    };
}

extern crate self as yav;

#[cfg(test)]
mod fullstack_ecosystem_tests {
    use crate::prelude::*;

    #[derive(Debug, PartialEq, FromFormData, ApiSchema)]
    struct SignupPayload {
        #[form(required = "Email is required", email = "Must be a valid email")]
        email: String,
        #[form(required, min_length = 8)]
        password: String,
        #[form(matches = "password", message = "Passwords must match")]
        confirm_password: String,
        #[form(min = 18, max = 120)]
        age: u32,
        newsletter: bool,
    }

    #[server("/api/rpc/create_post")]
    async fn create_post(ctx: RouteContext) -> crate::server::Response {
        let _ = ctx;
        html_response(StatusCode::OK, "<div>Post Created</div>")
    }

    #[test]
    fn test_derive_from_form_data_and_api_schema() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("email".to_string(), "dev@yav.rs".to_string());
        fields.insert("password".to_string(), "supersecret123".to_string());
        fields.insert("confirm_password".to_string(), "supersecret123".to_string());
        fields.insert("age".to_string(), "28".to_string());
        fields.insert("newsletter".to_string(), "true".to_string());

        let form = FormData(fields);
        let parsed = SignupPayload::from_form(&form).expect("Valid form should parse");
        assert_eq!(parsed.email, "dev@yav.rs");
        assert_eq!(parsed.age, 28);
        assert!(parsed.newsletter);

        let schema_json = SignupPayload::openapi_schema_json();
        assert!(schema_json.contains("\"email\""));
        let ts = SignupPayload::ts_interface();
        assert!(ts.contains("export interface SignupPayload"));
    }

    #[test]
    fn test_server_macro_and_fullstack_modules() {
        assert_eq!(create_post_url(), "/api/rpc/create_post");
        let app = register_create_post(YavApp::new());
        assert_eq!(app.routes.len(), 1);
    }

    #[test]
    fn test_complex_saas_enterprise_ecosystem() {
        // 1. CRDT & LexoRank
        let r1 = LexoRank::initial(0);
        let r2 = LexoRank::initial(1);
        let mid = LexoRank::between(Some(&r1), Some(&r2));
        assert!(r1 < mid && mid < r2);

        // 2. Exact Money & Penny Allocation
        let total = Money::usd(100);
        let split = total.allocate(&[1, 1, 1]);
        assert_eq!(split[0].amount_minor, 34);
        assert_eq!(split[1].amount_minor, 33);

        // 3. Feature Flags & Hash-Chained Audit Trail
        let flags = FlagEngine::new();
        flags.register(FeatureFlag::new("ai-copilot", true).rollout(100));
        assert!(flags.is_enabled("ai-copilot", Some("user_1"), None));

        let audit = AuditTrail::new();
        audit.record(
            "user_1",
            "org_1",
            "invoice.paid",
            "inv_99",
            std::collections::BTreeMap::new(),
        );
        assert!(audit.verify_integrity());

        // 4. Geospatial Haversine & Geofencing
        let hq = GeoPoint::new(48.8566, 2.3522);
        let client = GeoPoint::new(48.8600, 2.3500);
        assert!(hq.distance_km(&client) < 2.0);

        // 5. Complex UI Components (Kanban, VirtualList, FlowCanvas, RichTextEditor, AiChat)
        let kanban_html = render_to_string(
            &KanbanBoard::new("board")
                .column(KanbanColumn::new("todo", "Todo").card(KanbanCard::new("1", "Task", &mid)))
                .render(),
        );
        assert!(kanban_html.contains("yav-kanban"));

        let flow_html = render_to_string(
            &FlowCanvas::new("flow")
                .node(FlowNode::new("a", "Start", 10.0, 10.0))
                .node(FlowNode::new("b", "End", 220.0, 10.0))
                .edge(FlowEdge::new("a", "b").animated(true))
                .render(),
        );
        assert!(flow_html.contains("yav-flow-canvas"));
    }

    #[test]
    fn test_next_generation_capabilities_fullstack() {
        // 1. Optimistic UI Mutations script
        assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:optimistic-toggle"));
        assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:rollback-toast"));

        // 2. Reactive Live Queries & CDC
        let live_mgr = LiveQueryManager::new(64);
        let mut rx = live_mgr.subscribe("orders");
        live_mgr.publish(
            "orders",
            LiveEvent::InvalidateTable {
                table: "orders".to_string(),
            },
        );
        let event = rx.try_recv().expect("Event must be received");
        assert_eq!(
            event,
            LiveEvent::InvalidateTable {
                table: "orders".to_string()
            }
        );

        // 3. PoW Anti-Bot Shield & Honeypot
        let shield = PowShield::new("test-secret")
            .with_difficulty(1)
            .with_ttl(60);
        let challenge = shield.create_challenge();
        assert_eq!(challenge.difficulty, 1);
        let mut form_fields = std::collections::HashMap::new();
        assert!(!Honeypot::is_bot(&form_fields, None));
        form_fields.insert("hp_company_website".to_string(), "spam bot".to_string());
        assert!(Honeypot::is_bot(&form_fields, None));

        // 4. GraphQL Schema & Execution
        let query = GraphQlObject::new("Query").field("ping", GraphQlType::String, |_args| {
            Ok(serde_json::Value::String("pong".to_string()))
        });
        let schema = GraphQlSchema::new(query);
        let res = schema.execute("{ ping }", None);
        assert_eq!(res.data.unwrap()["ping"], "pong");

        // 5. GeoIP & Multi-Currency EU VAT
        assert!(VatRule::is_eu("FR"));
        let breakdown = VatRule::calculate_tax(Money::eur(1000), "FR");
        assert_eq!(breakdown.tax_amount.amount_minor, 200);
        assert_eq!(breakdown.gross.amount_minor, 1200);

        // 6. Micro-Frontends & Module Federation
        let gateway = FederationGateway::new().route("/checkout", "http://127.0.0.1:4001");
        let route = gateway.resolve("/checkout/pay").expect("Route resolved");
        assert_eq!(route.prefix, "/checkout");
        assert_eq!(
            gateway.target_url(route, "/checkout/pay"),
            "http://127.0.0.1:4001/checkout/pay"
        );

        // 7. Vector Barcode 128 & QR Code
        let bc_svg = Barcode128::svg("YAV-PROD-2026", 60, 2);
        assert!(bc_svg.contains("<svg"));
        let qr_svg = QrCode::encode("https://yav.rs").svg(200);
        assert!(qr_svg.contains("<svg"));

        // 8. Automated Security Auditor & CSP Reporter
        let headers = HeaderMap::new();
        let report = SecurityAuditor::audit(&headers);
        assert!(report.score < 50); // Missing headers yields low score
        assert_eq!(report.grade, SecurityGrade::F);

        let csp_rep = CspReporter::new(10);
        csp_rep.record(CspViolation {
            document_uri: "https://yav.rs".to_string(),
            referrer: None,
            blocked_uri: "http://malicious.com".to_string(),
            violated_directive: "script-src".to_string(),
            effective_directive: "script-src".to_string(),
            original_policy: "default-src 'self'".to_string(),
            status_code: 200,
        });
        assert_eq!(csp_rep.list().len(), 1);

        // 9. Web ANSI Terminal
        let term_html = render_to_string(&Terminal::new("main-term").prompt("yav$ ").render());
        assert!(term_html.contains("yav-terminal"));
        assert!(term_html.contains("yav$ "));

        // 10. Dynamic Form Builder
        let form_schema = serde_json::json!({
            "type": "object",
            "required": ["username"],
            "properties": {
                "username": { "type": "string", "title": "Username" }
            }
        });
        let dyn_form_html =
            render_to_string(&DynamicForm::new(form_schema, "/api/register").render());
        assert!(dyn_form_html.contains("yav-dynamic-form"));
        assert!(dyn_form_html.contains("username"));

        // 11. Dual-Pane Markdown Editor
        let md_html = render_to_string(
            &MarkdownEditor::new("editor")
                .initial_value("# Live Title")
                .render(),
        );
        assert!(md_html.contains("yav-markdown-editor"));
        assert!(md_html.contains("Live Title"));

        // 12. Advanced Charts: Heatmap, Sankey, Radar
        let heatmap_html = render_to_string(&Heatmap::new("hm").record("2026-01-01", 10).render());
        assert!(heatmap_html.contains("yav-heatmap"));

        let sankey_html = render_to_string(
            &SankeyChart::new("sankey")
                .node("src", "Source", "#6366f1")
                .node("dst", "Destination", "#06b6d4")
                .link("src", "dst", 50.0)
                .render(),
        );
        assert!(sankey_html.contains("yav-sankey"));

        let radar_html = render_to_string(
            &RadarChart::new(
                "radar",
                vec!["perf", "sec", "scale"],
                vec![0.98, 0.99, 0.95],
            )
            .render(),
        );
        assert!(radar_html.contains("yav-radar-chart"));

        // 13. Local-First & Offline Sync
        let mutation = OfflineMutation::new(
            "mut_1",
            "todos",
            MutationType::Insert,
            "{\"title\":\"Buy milk\"}",
            1730000000,
            1,
        );
        assert_eq!(mutation.entity, "todos");
        assert_eq!(mutation.id, "mut_1");
        assert!(!mutation.synced);

        // 14. Zero-Overhead WebAssembly Client Islands
        let wasm_island =
            WasmIsland::new("wasm-canvas", "/wasm/canvas_bg.wasm", "render_frame").render();
        let wasm_html = render_to_string(&wasm_island);
        assert!(wasm_html.contains("wasm-island"));
        assert!(wasm_html.contains("render_frame"));

        // 15. Circuit Breaker & Adaptive Concurrency Limiter
        let cb = CircuitBreaker::with_config("stripe-api", CircuitBreakerConfig::default());
        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(cb.allow_request());

        // 16. Transactional Outbox Pattern
        let outbox = TransactionalOutbox::new();
        let msg_id = outbox.publish("order_created", "order", "1001", "{\"order_id\":1001}");
        let pending = outbox.poll_pending(10);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, msg_id);

        // 17. Multi-Region Read/Write Database Replicas
        let pool = DbReplicaPool::new("postgres://master", "us-east");
        pool.add_replica("rep1", "eu-west", "postgres://eu-rep", 100);
        assert_eq!(pool.route("POST", None, None), RouteTarget::Primary);
        assert!(matches!(
            pool.route("GET", Some("eu-west"), None),
            RouteTarget::Replica { .. }
        ));

        // 18. Pure-Rust WebP / AVIF & Image Pipeline
        let png_bytes = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image_format(png_bytes), ImageFormat::Png);
        let webp_packed = WebPEncoder::pack_vp8_chunk(b"DATA");
        assert_eq!(detect_image_format(&webp_packed), ImageFormat::WebP);

        // 19. Interactive Org Chart & Family Tree SVG Graph
        let org_chart_html = render_to_string(
            &OrgChart::new()
                .id("company-tree")
                .node(OrgNode::new("ceo", "Alice Vance", "CEO"))
                .node(OrgNode::new("lead", "Bob Stone", "VP Tech").parent("ceo"))
                .render(),
        );
        assert!(org_chart_html.contains("company-tree"));
        assert!(org_chart_html.contains("Alice Vance"));

        // 20. Interactive Audio Synthesizer & Waveform Visualizer
        let wave_html = render_to_string(
            &AudioWaveform::new()
                .id("audio-scrubber")
                .duration(180.0)
                .current_time(45.0)
                .render(),
        );
        assert!(wave_html.contains("audio-scrubber"));
        assert!(wave_html.contains("00:45"));

        // 21. Vector PDF Signer & Digital Watermarking
        let key = b"cert-secret-key-2026";
        let hmac_sig = PdfSigner::compute_hmac(key, b"Invoice INV-2026");
        assert!(PdfSigner::verify_hmac(key, b"Invoice INV-2026", &hmac_sig));
        let wm = PdfWatermark::new("PAID & SEALED");
        let wm_ops = wm.to_pdf_stream_operators(595.0, 842.0);
        assert!(wm_ops.contains("PAID & SEALED"));

        // 22. Native OpenTelemetry Logtail Live Inspector
        let log_html = render_to_string(
            &LogViewer::new()
                .id("live-otel-logs")
                .log(LogEntry::new(
                    "2026-11-05 12:00:00",
                    LogLevel::Info,
                    "server",
                    "Started",
                ))
                .render(),
        );
        assert!(log_html.contains("live-otel-logs"));
        assert!(log_html.contains("Started"));

        // 23. Collaborative Whiteboard & Drawing Canvas
        let wb_html = render_to_string(
            &Whiteboard::new()
                .id("team-whiteboard")
                .element(WhiteboardElement::pen(
                    "stroke-1",
                    vec![(10.0, 10.0), (30.0, 40.0)],
                    "#4f46e5",
                ))
                .render(),
        );
        assert!(wb_html.contains("team-whiteboard"));
        assert!(wb_html.contains("<path id=\"stroke-1\""));

        // 24. Natural Language AI Filter Bar
        let ai_bar_html = render_to_string(
            &AiFilterBar::new()
                .id("filter-nl")
                .filter(AiFilterToken::new("status", "=", "active"))
                .render(),
        );
        assert!(ai_bar_html.contains("filter-nl"));
        assert!(ai_bar_html.contains("active"));
    }

    #[test]
    fn test_frontier_twelve_capabilities_fullstack() {
        use std::time::Duration;

        // 1. Multi-Tier Distributed Cache & XFetch
        let cache = DistributedCache::new(100, Duration::from_secs(3600));
        let val = cache.get_or_compute("user:101", || "Alice Vance".to_string());
        assert_eq!(val, "Alice Vance");
        let cached = cache.get("user:101");
        assert_eq!(cached, Some("Alice Vance".to_string()));

        // 2. Distributed Lock & Leader Election Protocol
        let coord = LockCoordinator::new();
        let lock = coord.try_acquire("global_task", "node_1", Duration::from_secs(30));
        assert!(lock.is_some());
        assert!(coord
            .try_acquire("global_task", "node_2", Duration::from_secs(30))
            .is_none());
        let elector = LeaderElector::new(coord.clone(), "cluster_shard", "worker_1");
        assert!(elector.campaign(Duration::from_secs(30)));
        assert!(elector.is_leader());

        // 3. ReBAC / Relationship-Based Access Control
        let rebac = RebacGraph::new();
        rebac.write_tuple("user:alice", "viewer", "doc:design_v2");
        rebac.write_tuple("user:bob", "owner", "doc:design_v2");
        assert!(rebac.check("user:bob", "viewer", "doc:design_v2"));

        // 4. Prometheus Metrics & Dashboard
        let dash_html =
            render_metrics_dashboard_html("Yav App", 3600, 1000, 42.5, 1.2, 3.8, 8.4, 98.5, 45.0);
        assert!(dash_html.contains("Metrics Dashboard"));

        // 5. Phonetic & Fuzzy Search
        assert_eq!(damerau_levenshtein("teh", "the"), 1);
        assert_eq!(soundex("Jackson"), soundex("Jaxon"));
        let mut f_idx = FuzzySearchIndex::new();
        f_idx.add("doc1", "Distributed Cache");
        f_idx.add("doc2", "Relationship Access Control");
        let matches = f_idx.search("Distributd", 2, 10);
        assert!(!matches.is_empty());
        assert_eq!(
            suggest_did_you_mean("distributd", &["distributed", "database"]),
            Some("distributed")
        );

        // 6. Vector Map & Geospatial Pin Visualizer
        let map_html = render_to_string(
            &vector_map()
                .id("office-map")
                .pin(MapPin::new("hq", 48.85, 2.35, "Paris Office"))
                .render(),
        );
        assert!(map_html.contains("office-map"));
        assert!(map_html.contains("Paris Office"));

        // 7. Interactive Gantt Chart
        let gantt_html = render_to_string(
            &gantt_chart()
                .id("proj-plan")
                .task(GanttTask::new("t1", "Architecture", 0, 5))
                .render(),
        );
        assert!(gantt_html.contains("proj-plan"));
        assert!(gantt_html.contains("Architecture"));

        // 8. Vector PDF Document Reader
        let pdf_html = render_to_string(
            &pdf_viewer("/specs/yav-v2.pdf")
                .id("pdf-preview")
                .annotation(PdfAnnotation::highlight(1, 20.0, 30.0, 100.0, 20.0))
                .render(),
        );
        assert!(pdf_html.contains("pdf-preview"));
        assert!(pdf_html.contains("src=\"/specs/yav-v2.pdf#page=1\""));

        // 9. Spotlight Onboarding Tour
        let tour_html = render_to_string(
            &spotlight_tour("app-tour")
                .step(TourStep::new("step-1", "#hero", "Welcome", "Take a tour"))
                .render(),
        );
        assert!(tour_html.contains("app-tour"));
        assert!(tour_html.contains("Take a tour"));

        // 10. Voice Memo & Media Recorder
        let rec_html = render_to_string(
            &media_recorder("memo-rec")
                .label("Customer Audio Feedback")
                .max_duration(60)
                .render(),
        );
        assert!(rec_html.contains("memo-rec"));
        assert!(rec_html.contains("Customer Audio Feedback"));

        // 11. Pure-Rust 3D Model Viewer
        let mesh_html = render_to_string(
            &model_viewer_3d("canvas-3d")
                .mesh(ModelMesh3D::cube(80.0))
                .render(),
        );
        assert!(mesh_html.contains("canvas-3d"));
        assert!(mesh_html.contains("3D MESH"));

        // 12. Interactive Code Playground
        let play_html = render_to_string(
            &code_playground("try-yav")
                .title("Online REPL")
                .language("rust")
                .code("fn main() {}")
                .render(),
        );
        assert!(play_html.contains("try-yav"));
        assert!(play_html.contains("Online REPL"));
    }

    #[test]
    fn test_frontier_twelve_wave_two_capabilities_fullstack() {
        use base64::Engine;
        // 1. Streaming TurboStream Multi-Zone DOM Mutations
        let stream = TurboStream::new()
            .append("feed", "<div>New update</div>")
            .replace("cart", "<span>3</span>");
        assert!(stream
            .render_html()
            .contains("action=\"append\" target=\"feed\""));
        assert!(stream.to_sse_event().contains("event: turbo-stream"));
        assert!(YAV_TURBO_STREAM_SCRIPT.contains("window.applyTurboStream"));

        // 2. Event Sourcing & CQRS Projections
        let es = EventStore::new();
        let ev = es
            .append(
                "order-99",
                0,
                "OrderPlaced",
                serde_json::json!({ "total": 9900 }),
            )
            .expect("First event appended");
        assert_eq!(ev.version, 1);
        assert_eq!(es.get_version("order-99"), 1);

        // 3. Resumable Chunked Uploads (Tus Protocol v1.0.0)
        let uploader = ChunkedUploader::new();
        let s = uploader.create_session("upload-xyz", 10, std::collections::HashMap::new());
        assert_eq!(s.total_size, 10);
        assert_eq!(uploader.append_chunk("upload-xyz", 0, b"12345").unwrap(), 5);
        assert_eq!(uploader.get_offset("upload-xyz"), Some(5));

        // 4. WebPush & VAPID Authorization Headers (real ES256 signing)
        let notif = WebPushNotification::new("Package Dispatched", "Delivery today").ttl(3600);
        assert!(notif.to_json().contains("Package Dispatched"));
        // Generate a real P-256 keypair for the test
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand_core::OsRng);
        let verifying_key = p256::ecdsa::VerifyingKey::from(&signing_key);
        let pub_bytes = p256::EncodedPoint::from(&verifying_key).to_bytes();
        let priv_bytes = signing_key.to_bytes();
        let pub_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&*pub_bytes);
        let priv_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(priv_bytes.as_slice());
        let signer = VapidSigner::new("mailto:ops@yav.rs", &pub_b64, &priv_b64);
        let header = signer
            .create_authorization_header("https://push.services.mozilla.com", 3600)
            .expect("ES256 signing with valid P-256 key must succeed");
        assert!(header.starts_with("vapid t="));

        // 5. Mock Server & Deterministic Data Faker
        let ms = MockServer::new();
        ms.mock(
            "GET",
            "/mock/users",
            200,
            serde_json::json!({ "success": true }),
        );
        assert_eq!(ms.resolve("GET", "/mock/users").unwrap().status, 200);
        assert_eq!(MockFaker::name(0), "Alice Vance");
        assert!(MockFaker::email(0).contains("@example.com"));

        // 6. i18n Static Extractor & Translation Linter
        let code = "let _ = t!(\"auth.login_title\");";
        let occ = scan_source_code("auth.rs", code);
        assert_eq!(occ[0].key, "auth.login_title");
        let report = lint_catalog(&occ, &["auth.login_title"]);
        assert!(report.is_clean());

        // 7. Interactive JSON Tree Explorer
        let json_html = render_to_string(
            &json_tree_viewer("jt", serde_json::json!({ "name": "Yav", "version": 0.2 })).render(),
        );
        assert!(json_html.contains("yav-json-tree"));
        assert!(json_html.contains("name"));

        // 8. Hexagonal Grid & Spatial Coordinates
        let hex_html =
            render_to_string(&hex_grid("hg").tile(HexTile::new(0, 0).label("HQ")).render());
        assert!(hex_html.contains("yav-hex-grid"));
        assert!(hex_html.contains("HQ"));

        // 9. Handwritten Signature Pad
        let sig_html = render_to_string(
            &signature_pad("sig-customer")
                .label("Customer E-Sign")
                .render(),
        );
        assert!(sig_html.contains("yav-signature-pad"));
        assert!(sig_html.contains("Customer E-Sign"));

        // 10. Chronological Timeline & Audit Trail
        let time_html = render_to_string(
            &timeline("tl")
                .item(TimelineItem::new("i1", "Created", "09:00"))
                .render(),
        );
        assert!(time_html.contains("yav-timeline"));
        assert!(time_html.contains("Created"));

        // 11. Image Comparison Slider
        let cmp_html = render_to_string(
            &image_compare_slider("cmp", "/before.jpg", "/after.jpg")
                .labels("Initial", "Enhanced")
                .render(),
        );
        assert!(cmp_html.contains("yav-image-compare"));
        assert!(cmp_html.contains("Initial"));
        assert!(cmp_html.contains("Enhanced"));

        // 12. GDPR Cookie Consent Center
        let cookie_html = render_to_string(
            &cookie_consent("gdpr-notice")
                .title("Confidentialité")
                .render(),
        );
        assert!(cookie_html.contains("yav-cookie-consent"));
        assert!(cookie_html.contains("Confidentialité"));
    }
}
