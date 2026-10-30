//! # Yav — Modern Full-Stack Web Framework for Rust (v0.1.4)
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
//! 9. [45+ UI Components & 13,000+ Icons](#45-ui-components--13000-icons)
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
//! yav = "0.1.4"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! Create your first application in `src/main.rs`:
//!
//! ```rust,ignore
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
//! In v0.1.4, you write standard Rust control flow directly in markup without closures:
//!
//! ```rust,ignore
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
//! ```rust,ignore
//! use yav::prelude::*;
//! use yav::ui::{CountUp, AnimatedBar, TypewriterCode, CursorSpotlight, TiltPlate, Magnetic};
//!
//! let view = rsx! {
//!     <div class="space-y-6">
//!         // Cursor tracking, 3D tilt & magnetic attraction
//!         <CursorSpotlight radius_px={450}>
//!             <TiltPlate max_deg={10.0} glare=true class="p-6 rounded-xl border">
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
//! ```rust,ignore
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
//! ```rust,ignore
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
//! ```rust,ignore
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
//! ```rust,ignore
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
//! ## 45+ UI Components & 13,000+ Icons
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
//! ### The 13,000+ SVG Icon Engine
//!
//! Yav features an integrated zero-runtime-dependency icon catalog ([`icons`]) searchable in $O(\log N)$ time:
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
//! ```rust,ignore
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
//! In v0.1.4, pre-render your entire application to static HTML files for CDN deployment:
//!
//! ```rust,ignore
//! let app = YavApp::new().get("/", index_handler).get("/docs", docs_handler);
//! let exported_count = app.export_static(Path::new("dist"), "https://yav.rs").await?;
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
//! ```rust,ignore
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
//! | [`core`] | `yav-core` | Virtual DOM, [`core::Node`], escaping, i18n, and context |
//! | [`macros`] | `yav-macros` | Procedural macros: [`rsx!`], [`#[component]`](component), etc. |
//! | [`router`] | `yav-router` | File-based routing, pattern matcher, and table resolution |
//! | [`server`] | `yav-server` | Axum HTTP server, [`server::YavApp`], sessions, SSR, and middlewares |
//! | [`ui`] | `yav-ui` | 40+ accessible UI components styled with Tailwind CSS |
//! | [`icons`] | `yav-icons` | 13,000+ vector SVG icons across Lucide, MynaUI, and Phosphor |
//! | [`build`] | `yav-build` | Asset hashing, Tailwind compiler integration, and manifest |
//!
//! For standard application development, simply import the [`prelude`]:
//!
//! ```rust
//! use yav::prelude::*;
//! ```

/// Core types: Virtual DOM ([`core::Node`]), HTML escaping, context propagation, and i18n.
pub use yav_core as core;

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
    auto_routes, component, include_routes, include_rsx, include_yav, rsx, t, t_strict,
    typed_routes, yav_component,
};

/// File-based and programmatic routing system with path pattern matching.
pub use yav_router as router;

/// HTTP server engine, Axum SSR integration, sessions, cookies, forms, and middleware.
pub use yav_server as server;

/// Official UI component library with 40+ accessible, pre-styled Tailwind components.
pub use yav_ui as ui;

/// Built-in icon system with 13,000+ SVG icons (Lucide, MynaUI, Phosphor).
pub use yav_icons as icons;

/// Build pipeline, asset fingerprinting, and standalone Tailwind CSS integration.
pub use yav_build as build;

/// Convenience prelude — importing this brings all standard Yav types, macros, and components into scope.
///
/// # Example
/// ```rust
/// use yav::prelude::*;
/// ```
pub mod prelude {
    // Core Virtual DOM, Escaping, Context & I18n
    pub use yav_core::{
        document_shell, document_shell_with_lang, escape_attribute, escape_html, escape_url,
        extract_head_templates, provide_context, render_to_buf, render_to_string, use_context,
        use_context_or_default, use_state, Attribute, AttributeValue, Children, Component,
        ContextMap, Element, Head, HeadProps, I18n, IntoAttributeValue, IntoNode, IntoPropValue,
        Layout, Node, PageMeta, YAV_MICRO_ACTIONS_SCRIPT,
    };

    // Procedural Macros
    pub use yav_macros::{
        auto_routes, component, include_routes, include_rsx, include_yav, rsx, t, t_strict,
        typed_routes, yav_component,
    };

    // Routing types and helpers
    pub use yav_router::prelude::*;

    // Server, Forms, Flash, SEO, Streaming, Session, Cache, WebSockets & Telemetry helpers
    pub use yav_server::{
        csrf_field, html_response, json_response, not_found, redirect, safe_redirect,
        scalar_ui_handler, sse_channel, sse_response, streaming_html, swagger_ui_handler,
        verify_csrf_token, ws_node_frame, ws_swap_frame, ActionResponse, ActionResult, ApiEndpoint,
        CacheEntry, CachePolicy, CspNonce, CsrfToken, Flash, FlashLevel, FlashMessage, FormAction,
        FormData, FormErrors, FromFormData, HealthStatus, HtmlFragment, HtmlPage, HtmxRequest,
        Loader, Method, MetricsSnapshot, OpenApiSpec, RateLimitConfig, RateLimitStatus,
        RateLimiter, RobotsTxt, RouteContext, SameSite, SecurityHeadersConfig, ServerMetrics,
        Session, SessionConfig, SessionSecret, Sitemap, SitemapEntry, SseMsg, SsrCache, StatusCode,
        StreamingHtml, Suspense, UploadedFile, WsBroadcast, YavApp,
    };

    // UI Components, SEO Metadata, Toast, Islands, Multi-Theme System, and Progressive Navigation
    pub use yav_ui::prelude::*;

    // Built-in Icon System
    pub use yav_icons::{render_icon, Icon, IconFamily, IconProps, IconSize, IconStyle};

    // Markdown & Content Engine
    pub use yav_build::{load_markdown_dir, parse_markdown, MarkdownDoc, TocHeading};
}
