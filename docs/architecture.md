# Yav Architecture

## Overview

Yav is organized as a Rust workspace with clear crate boundaries. Each crate has a well-defined responsibility and minimal coupling to other crates.

## Crate Dependency Graph

```
yav (umbrella re-exports)
├── yav-core (foundation: Node, Element, rendering, escaping)
├── yav-macros (proc macros: rsx!, #[component])
├── yav-router (URL matching, route tables)
│   └── yav-core
├── yav-server (Axum HTTP server, SSR, middleware)
│   ├── yav-core
│   └── yav-router
├── yav-icons (Lucide SVG icon catalog)
│   └── yav-core
├── yav-ui (component library: Button, Card, etc.)
│   ├── yav-core
│   └── yav-icons
├── yav-build (package.yavml manifest, Tailwind CLI, asset pipeline)
└── yav-cli (CLI tool: yav new/dev/build)
    ├── yav-core
    ├── yav-server
    ├── yav-build
    └── yav-router
```

## Rendering Pipeline

```
HTTP Request
    │
    ▼
Route Resolution (yav-router)
    │
    ▼
Handler Execution (yav-server)
    │ calls component functions
    ▼
Component Tree (yav-core::Node)
    │ rsx! macro expands at compile time
    ▼
HTML Rendering (yav-core::render_to_string)
    │ escaping, attribute serialization
    ▼
Document Shell (yav-core::document_shell)
    │ wraps in <!DOCTYPE html>, injects CSS/JS
    ▼
HTTP Response (axum::Response)
```

## Key Design Decisions

### 1. Server-Side Rendering First

All pages render to HTML on the server. No WebAssembly is required for basic
pages. This ensures fast initial loads and works without JavaScript.

### 2. Proc Macro for RSX (not a custom compiler)

The `rsx!` macro is a standard Rust procedural macro. This means:
- Works with any Rust editor/IDE
- Standard Rust error messages
- No custom build step
- No `.rsx` file format

The grammar is a restricted but well-documented subset of JSX that can be
parsed from Rust's token stream.

### 3. Tailwind CSS via Standalone CLI

Yav manages the official Tailwind CSS standalone binary. This avoids:
- Node.js dependency
- npm/npx/package.json
- Complex JavaScript toolchain setup

The binary is downloaded once, cached, and version-pinned.

### 4. Progressive Enhancement

The framework works without any client-side JavaScript. For interactivity:
1. HTML forms and links (always work)
2. Small framework-managed JS snippets (optional)
3. Future: partial HTML updates via server-sent events

### 5. Axum as the HTTP Foundation

Axum provides a mature, well-tested HTTP framework. Yav wraps it with
higher-level abstractions but doesn't hide it. Developers can use Axum
types directly when needed.

## Desktop Extensibility

The `Node` type and component model are platform-independent. The rendering
pipeline (`render_to_string`) is specific to HTML output. A future desktop
adapter could:

1. Implement an alternative renderer for `Node` trees
2. Use a WebView to display the HTML output
3. Add desktop-specific component variants

The UI component library uses design tokens that could map to native controls.

## Security Model

- All text content is HTML-escaped by default (`escape_html`)
- Attribute values are escaped (`escape_attribute`)
- URLs are validated against `javascript:` and `data:` schemes
- `Node::Raw` is explicitly opt-in for trusted HTML
- HMAC-SHA256 cryptographically signed session cookies (`Session`)
- Sliding-window in-memory rate limiting against brute-force and DDoS (`RateLimiter`)
- Automatic security headers via middleware (`Content-Security-Policy`, `X-Content-Type-Options`, `X-Frame-Options`)
- Path traversal prevention in static file serving with content-hashed immutable caching

## Advanced Full-Stack Architecture

### 1. Streaming SSR & Chunked Transfer (`StreamingHtml`)
Flushes document `<head>`, CSS, and layout skeleton in under 5ms, streaming dynamic HTML fragments asynchronously over HTTP chunked transfer and swapping placeholders in-place via micro-templates.

### 2. Real-Time Primitives (`SseStream` / `SseMsg`)
Lightweight Server-Sent Events channel abstractions with standard event protocol serialization, reconnection timers, and native compatibility with EventSource and HTMX `hx-ext="sse"`.

### 3. OpenAPI 3.1 & Embedded Interactive Documentation
Automated schema generation for hybrid SSR and REST API applications, with embedded Scalar and Swagger UI endpoints served directly at `/api/docs`.

### 4. Database Scaffolding & Migrations (`yav db`)
CLI workflow for forward and rollback SQL migrations with timestamping and environment configuration.

### 5. Motion Engine Architecture (`yav-motion`)
Server-side declarative animations compiled to compact `data-motion` HTML5 datasets, powered on the client by the browser's native **Web Animations API** (WAAPI).
- **Zero JS Dependencies**: Under 2KB total client footprint without Framer Motion, GSAP, or React.
- **Spring Physics**: Accurate mass/damping/stiffness simulations executed directly on the browser's compositor thread.
- **Micro-Script Hydration**: `motion_script()` initializes elements via `requestAnimationFrame` and `IntersectionObserver` for viewport reveal staggering.

### 6. Static VDOM Folding & Zero-Allocation Rendering (`Node::Static`)
Compile-time static chunk folding in the `rsx!` procedural macro identifies immutable DOM subtrees and packs them into `Node::Static(&'static str)`. During SSR rendering, these branches bypass dynamic attribute serialization and string allocations, writing directly to pooled string buffers.

### 7. Automatic HTTP ETag & Conditional Requests (`compute_etag`)
Built-in caching middleware (`YavApp::etag(true)`) computes deterministic Blake3/SHA-256 ETags over rendered responses. When incoming requests include `If-None-Match`, Yav immediately halts pipeline execution and returns `304 Not Modified`, saving server bandwidth and client battery life.

### 8. View Transitions API & Progressive Client Routing
The client navigation engine (`client_router_script()`) integrates the browser's modern **View Transitions API** (`document.startViewTransition()`):
- **Predictive Prefetching**: Prefetches target HTML over `<Link prefetch=true>` via hover or `IntersectionObserver`.
- **Morphing Page Swaps**: Smooth cross-fades and shared element animations across page changes.
- **100% Graceful Degradation**: Fallback to instant DOM swapping or standard MPA page reloads if JavaScript or View Transitions are unsupported.

