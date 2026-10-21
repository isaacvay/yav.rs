//! # 🦀 Yav — Modern Full-Stack Web Framework for Rust
//!
//! **Yav** is a batteries-included, server-side rendered (SSR) web framework combining the developer
//! ergonomics of **Next.js** with the integrated UI philosophy of **Flutter**, powered by **Axum**,
//! **Tokio**, and **Tailwind CSS**.
//!
//! Write your entire web application in type-safe, compiled Rust — without Node.js, npm, or JavaScript build tooling.
//!
//! ---
//!
//! ## 📑 Table of Contents
//!
//! 1. [Quick Start & Architecture](#quick-start--architecture)
//! 2. [Declarative UI with RSX](#declarative-ui-with-rsx)
//! 3. [Component Model & Props](#component-model--props)
//! 4. [HTTP Server & Routing](#http-server--routing)
//! 5. [State, Cookies & Signed Sessions](#state-cookies--signed-sessions)
//! 6. [Form Actions & Security](#form-actions--security)
//! 7. [40+ UI Components & 13,000+ Icons](#40-ui-components--13000-icons)
//! 8. [SEO, Social Cards & Sitemaps](#seo-social-cards--sitemaps)
//! 9. [Interactive API Documentation (OpenAPI / Scalar)](#interactive-api-documentation-openapi--scalar)
//! 10. [Prelude & Re-exports](#modules-and-prelude)
//!
//! ---
//!
//! ## Quick Start & Architecture
//!
//! Add `yav` and `tokio` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! yav = "0.1"
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
//! ## Declarative UI with RSX
//!
//! The [`rsx!`] procedural macro provides an intuitive JSX-like authoring syntax compiled directly
//! into a zero-allocation, immutable Virtual DOM tree ([`core::Node`]):
//!
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let user_name = "Alice";
//! let is_admin = true;
//! let notifications = vec!["Security alert", "Welcome gift"];
//!
//! let view = rsx! {
//!     <section class="dashboard">
//!         <h2>{ format!("Hello, {}!", user_name) }</h2>
//!
//!         // Conditional rendering
//!         { if is_admin {
//!             rsx! { <span class="badge badge-admin">"Admin Panel"</span> }
//!         } else {
//!             Node::Empty
//!         } }
//!
//!         // List iteration
//!         <ul class="notification-list">
//!             { notifications.iter().map(|item| rsx! {
//!                 <li class="notification-item">{ *item }</li>
//!             }).collect::<Vec<_>>() }
//!         </ul>
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
//! ## HTTP Server & Routing
//!
//! Yav's server layer is built directly upon **Axum** and **Tower**, inheriting enterprise-grade concurrency,
//! connection pooling, and asynchronous streaming.
//!
//! ### Handlers & Route Context
//!
//! Handlers receive a [`server::RouteContext`] allowing inspection of URL parameters, query parameters, headers,
//! client IP, and typed dependencies:
//!
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let app = YavApp::new()
//!     // Dynamic URL parameters: /users/:id
//!     .get("/users/:id", |ctx: RouteContext| async move {
//!         let user_id = ctx.param("id").unwrap_or("unknown");
//!         html_response(StatusCode::OK, rsx! {
//!             <div><h1>{ format!("User Profile #{}", user_id) }</h1></div>
//!         })
//!     })
//!     // Query string parsing: /search?q=rust&limit=20
//!     .get("/search", |ctx: RouteContext| async move {
//!         let query = ctx.query("q").unwrap_or("");
//!         let limit = ctx.query_as::<usize>("limit").unwrap_or(10);
//!         json_response(StatusCode::OK, serde_json::json!({
//!             "query": query,
//!             "limit": limit
//!         }))
//!     });
//! ```
//!
//! ---
//!
//! ## State, Cookies & Signed Sessions
//!
//! Yav provides cryptographically signed, tamper-proof session cookies backed by standard **HMAC-SHA256**:
//!
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let session_config = SessionConfig::new()
//!     .cookie_name("my_app_sid")
//!     .same_site(SameSite::Lax)
//!     .secure(true)
//!     .ttl(86400 * 7); // 7 days
//!
//! let app = YavApp::new()
//!     .session_secret("super-secret-key-at-least-32-characters-long")
//!     .session(session_config)
//!     .get("/dashboard", |session: Session| async move {
//!         let user_id = session.get("user_id").unwrap_or_else(|| "guest".to_string());
//!         html_response(StatusCode::OK, format!("Welcome, {user_id}!"))
//!     })
//!     .post("/login", |session: Session| async move {
//!         // Set authenticated state:
//!         session.set("user_id", "user_12345");
//!         redirect("/dashboard")
//!     });
//! ```
//!
//! ---
//!
//! ## Form Actions & Security
//!
//! Progressive form submissions with built-in CSRF token verification and structured error reporting:
//!
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let app = YavApp::new()
//!     // Render form with CSRF protection token
//!     .get("/contact", |_ctx| async {
//!         let csrf = csrf_field("secret_token_123");
//!         html_response(StatusCode::OK, rsx! {
//!             <form method="POST" action="/contact" class="space-y-4">
//!                 { csrf }
//!                 <Input name="email" r#type="email" placeholder="you@domain.com" required=true />
//!                 <Button r#type="submit">"Send Message"</Button>
//!             </form>
//!         })
//!     })
//!     // Handle form action submission
//!     .post("/contact", |ctx: RouteContext| async move {
//!         let form: FormAction = ctx.form().await?;
//!         if !verify_csrf_token(form.get("_csrf"), "secret_token_123") {
//!             return Ok(html_response(StatusCode::FORBIDDEN, "Invalid CSRF token"));
//!         }
//!         redirect("/thank-you")
//!     });
//! ```
//!
//! ---
//!
//! ## 40+ UI Components & 13,000+ Icons
//!
//! Yav embeds a complete UI design system crafted with Tailwind CSS utility classes and full WAI-ARIA
//! accessibility support:
//!
//! - **Primitives**: `Button`, `Input`, `Textarea`, `Select`, `Checkbox`, `Radio`, `Badge`, `Avatar`
//! - **Containers & Layouts**: `Card`, `Container`, `Stack`, `Grid`, `Separator`
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
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let view = rsx! {
//!     <div class="flex items-center gap-3">
//!         // Lucide stroke icons (Classic)
//!         { render_icon("classic:check-circle", 20, "text-emerald-500") }
//!
//!         // MynaUI modern regular & solid icons (Minimal)
//!         { render_icon("minimal:bell", 20, "text-amber-500") }
//!
//!         // Phosphor bold, duotone & fill icons (System)
//!         { render_icon("system:shield-check:bold", 24, "text-indigo-500") }
//!     </div>
//! };
//! ```
//!
//! ---
//!
//! ## SEO, Social Cards & Sitemaps
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
//!
//! let head_html = meta.render_head_tags();
//! ```
//!
//! Generate search-engine compliant `sitemap.xml` and `robots.txt` programmatically:
//!
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! let sitemap = Sitemap::new()
//!     .add_url("https://yav.rs/")
//!     .add_url("https://yav.rs/docs");
//!
//! let robots = RobotsTxt::new()
//!     .disallow("/api/private")
//!     .sitemap("https://yav.rs/sitemap.xml");
//! ```
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
    auto_routes, component, include_routes, include_rsx, include_yav, rsx, yav_component,
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
        provide_context, render_to_buf, render_to_string, use_context, use_context_or_default,
        Attribute, AttributeValue, Children, Component, ContextMap, Element, I18n, IntoNode,
        Layout, Node, PageMeta,
    };

    // Procedural Macros
    pub use yav_macros::{
        auto_routes, component, include_routes, include_rsx, include_yav, rsx, yav_component,
    };

    // Routing types and helpers
    pub use yav_router::prelude::*;

    // Server, Forms, Flash, SEO, Streaming, Session & Telemetry helpers
    pub use yav_server::{
        csrf_field, html_response, json_response, not_found, redirect, scalar_ui_handler,
        sse_channel, sse_response, streaming_html, swagger_ui_handler, verify_csrf_token,
        ActionResponse, ActionResult, ApiEndpoint, Flash, FlashLevel, FlashMessage, FormAction,
        FormErrors, HealthStatus, HtmlFragment, HtmlPage, HtmxRequest, Loader, MetricsSnapshot,
        OpenApiSpec, RateLimitConfig, RateLimitStatus, RateLimiter, RobotsTxt, RouteContext,
        SameSite, ServerMetrics, Session, SessionConfig, SessionSecret, Sitemap, SitemapEntry,
        SseMsg, StreamingHtml, Suspense, UploadedFile, YavApp,
    };

    // Re-export common HTTP status codes for convenient handler responses
    pub use yav_server::StatusCode;

    // UI Components, SEO Metadata, Toast, Islands, and Progressive Navigation
    pub use yav_ui::prelude::*;

    // Built-in Icon System
    pub use yav_icons::{render_icon, Icon, IconFamily, IconSize, IconStyle};
}
