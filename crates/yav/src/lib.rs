//! # Yav — A Rust-first web framework
//!
//! Yav provides an integrated experience for building modern web applications
//! with Rust, inspired by the developer experience of Next.js and the
//! integrated UI philosophy of Flutter.
//!
//! ## Quick Start
//!
//! ```bash
//! cargo install yav-cli
//! yav new my-app
//! cd my-app
//! yav dev
//! ```
//!
//! ## Writing Components
//!
//! ```ignore
//! use yav::prelude::*;
//!
//! fn home_page() -> Node {
//!     rsx! {
//!         <div class="container">
//!             <h1>"Welcome to Yav"</h1>
//!             <p>"Build modern web apps with Rust."</p>
//!         </div>
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! - **JSX-like syntax** via the `rsx!` macro
//! - **Server-side rendering** with HTML-first output
//! - **File-based routing** inspired by Next.js
//! - **Built-in UI components** with Tailwind CSS styling
//! - **Icon system** with 80+ built-in Lucide icons
//! - **Tailwind CSS** integration without Node.js
//! - **Development server** with hot reload
//! - **Production builds** with asset optimization

/// Core types: Node, Element, Attribute, rendering, escaping.
pub use yav_core as core;

/// Hidden re-export for proc macro hygiene — allows `rsx!` to reference
/// `::yav::yav_core::Node` in downstream crates that don't directly depend on `yav-core`.
#[doc(hidden)]
pub extern crate yav_core;

/// Procedural macros: `rsx!`, `#[component]`, `auto_routes!`, `include_rsx!`, `include_yav!`, `include_routes!`, `yav_component!`.
pub use yav_macros::{
    auto_routes, component, include_routes, include_rsx, include_yav, rsx, yav_component,
};

/// File-based routing system.
pub use yav_router as router;

/// HTTP server and SSR engine.
pub use yav_server as server;

/// Official UI component library.
pub use yav_ui as ui;

/// Built-in icon system.
pub use yav_icons as icons;

/// Build pipeline and Tailwind integration.
pub use yav_build as build;

/// Convenience prelude — import everything you commonly need.
///
/// ```ignore
/// use yav::prelude::*;
/// ```
pub mod prelude {
    // Core types
    pub use yav_core::{
        document_shell, document_shell_with_lang, escape_attribute, escape_html, escape_url,
        provide_context, render_to_buf, render_to_string, use_context, use_context_or_default,
        Attribute, AttributeValue, Children, Component, ContextMap, Element, I18n, IntoNode,
        Layout, Node, PageMeta,
    };

    // Macros
    pub use yav_macros::{
        auto_routes, component, include_routes, include_rsx, include_yav, rsx, yav_component,
    };

    // Routing
    pub use yav_router::prelude::*;

    // Server, Forms, Flash, SEO, Streaming, Session & Telemetry helpers
    pub use yav_server::{
        csrf_field, html_response, json_response, not_found, redirect, scalar_ui_handler,
        sse_channel, sse_response, streaming_html, swagger_ui_handler, verify_csrf_token,
        ActionResponse, ActionResult, ApiEndpoint, Flash, FlashLevel, FlashMessage, FormAction,
        FormErrors, HealthStatus, HtmlFragment, HtmlPage, HtmxRequest, Loader, MetricsSnapshot,
        OpenApiSpec, RateLimitConfig, RateLimitStatus, RateLimiter, RobotsTxt, RouteContext,
        ServerMetrics, Session, SessionConfig, SessionSecret, Sitemap, SitemapEntry, SseMsg,
        StreamingHtml, Suspense, UploadedFile,
    };

    // UI Components, SEO Metadata, Toast, Islands, and Progressive Navigation
    pub use yav_ui::prelude::*;

    // Built-in Icon System
    pub use yav_icons::{render_icon, Icon, IconFamily, IconSize, IconStyle};
}
