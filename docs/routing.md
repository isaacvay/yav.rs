# Routing

Yav provides a powerful hybrid routing system combining **Next.js-inspired file-based routing conventions** with **type-safe Axum HTTP handlers** and automatic compile-time route discovery.

---

## 1. File-Based Routing Conventions

Routes are defined by the directory and file structure under `src/routes/`.

### Route Mapping Convention

| File Path | URL Route | Type | Description |
|-----------|-----------|------|-------------|
| `src/routes/index.rs` or `page.rs` | `/` | Static | Root application page |
| `src/routes/about.rs` | `/about` | Static | Static page |
| `src/routes/dashboard/index.rs` | `/dashboard` | Static | Nested route |
| `src/routes/dashboard/settings.rs` | `/dashboard/settings` | Static | Sub-page |
| `src/routes/(auth)/login.rs` | `/login` | Group | Route group (parentheses ignored in URL) |

### Dynamic Route Parameters

Yav supports two syntax conventions for dynamic parameters:

#### A. Rust-Friendly Identifier Convention (Recommended)
Because Rust module names cannot contain brackets `[` or `]`, Yav natively supports trailing underscores:

| File Path | URL Pattern | Extracted Param | Axum Pattern |
|-----------|-------------|-----------------|--------------|
| `src/routes/users/_id_.rs` | `/users/:id` | `id = "42"` | `/users/{id}` |
| `src/routes/posts/_slug_.rs` | `/posts/:slug` | `slug = "hello-world"` | `/posts/{slug}` |
| `src/routes/docs/__slug__.rs` | `/docs/*slug` | `slug = "api/auth"` | `/docs/{*slug}` |

#### B. Next.js Bracket Convention (with `#[path]`)
If you prefer bracket file names like `[id].rs`, declare the module in `src/routes/mod.rs` with the `#[path]` attribute (automatically done by `yav generate route`):

```rust
// In src/routes/users/mod.rs:
#[path = "[id].rs"]
pub mod id;
pub use id::*;
```

### Route Priority
When multiple routes could match a request URL, precedence is evaluated as:
1. **Static segments** — exact literal matches first (`/users/profile`)
2. **Dynamic segments** — single-parameter matches (`/users/:id`)
3. **Catch-all wildcards** — rest-of-path matches (`/docs/*slug`)

---

## 2. Automatic Compile-Time Route Wiring (`auto_routes!`)

Instead of writing repetitive `.get(...)` calls for every page, Yav provides the `auto_routes!` macro. It scans `src/routes/` at compile time and automatically attaches all discovered handlers to your `YavApp`:

```rust
// src/main.rs
mod components;
mod layouts;
mod routes;

use yav::prelude::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 1. Initialize YavApp
    let app = yav::server::YavApp::new().static_dir("public");

    // 2. Automatically discover and wire all handlers in src/routes/
    let app = yav::auto_routes!(app);

    println!("🚀 Yav server running on http://{}:{}", app.host, app.port);
    app.serve().await.unwrap();
}
```

### Supported Handler Conventions in Route Files

In each route file under `src/routes/`, the `auto_routes!` macro detects:

- `pub async fn handler(_req: Request) -> HtmlPage` — Default GET handler
- `pub async fn get(_req: Request) -> HtmlPage` — Explicit HTTP GET handler
- `pub async fn post(_req: Request) -> HtmlPage` — Form mutation / HTTP POST handler
- `pub async fn put(_req: Request) -> HtmlPage` — HTTP PUT handler
- `pub async fn delete(_req: Request) -> HtmlPage` — HTTP DELETE handler
- Function named after the file stem (e.g. `pub async fn index(...)` or `pub async fn about(...)`)

---

## 3. Dynamic Parameter & Query Extraction

Extract parameters using either `RouteContext` or Axum's typed extractors:

```rust
use yav::prelude::*;
use axum::extract::{Request, Path, Query};
use serde::Deserialize;

// Method 1: Via RouteContext (recommended for Yav applications)
pub async fn project_detail(req: Request) -> HtmlPage {
    let ctx = RouteContext::from_request(&req);
    let id = ctx.param("id").unwrap_or("unknown");
    let search = ctx.query("search");

    HtmlPage::new(rsx! {
        <h1>"Project: " {id}</h1>
    })
}

// Method 2: Via Axum Typed Extractors
#[derive(Deserialize)]
pub struct FilterParams {
    pub page: Option<u32>,
}

pub async fn users_list(
    Path(id): Path<String>,
    Query(filter): Query<FilterParams>,
) -> HtmlPage {
    HtmlPage::new(rsx! {
        <p>"User: " {id} " Page: " {filter.page.unwrap_or(1).to_string()}</p>
    })
}
```

---

## 4. Nested Layouts

Organize layouts hierarchically using composable layout components:

```text
src/
├── layouts/
│   ├── mod.rs
│   ├── app_layout.rs       # Global shell: navbar, footer, theme script
│   └── dashboard_layout.rs # Nested shell: sidebar, status badges
└── routes/
    ├── index.rs            # Wrapped in AppLayout
    └── dashboard/
        ├── index.rs        # Wrapped in DashboardLayout
        └── settings.rs     # Wrapped in DashboardLayout
```

```rust
// src/layouts/dashboard_layout.rs
#[component]
pub fn DashboardLayout(props: DashboardLayoutProps) -> Node {
    rsx! {
        <div class="flex min-h-screen">
            <aside class="w-64 bg-zinc-900 text-white p-4">
                <nav>
                    <a href="/dashboard">"Overview"</a>
                    <a href="/dashboard/settings">"Settings"</a>
                </nav>
            </aside>
            <main class="flex-1 p-8">
                {props.children}
            </main>
        </div>
    }
}
```

---

## 5. Programmatic Routing (`FileSystemRouter` & `YavApp`)

### Inspecting Discovered Routes with `FileSystemRouter`
```rust
use yav::router::fs_router::FileSystemRouter;

// Scan a directory recursively
let fs_router = FileSystemRouter::scan_dir("src/routes").unwrap();

for route in fs_router.routes() {
    println!("Route: {} -> Axum: {}", route.path_pattern, route.axum_pattern());
    if route.is_dynamic() {
        println!("  (Contains dynamic parameters)");
    }
}
```

### Checking Route Existence on `YavApp`
```rust
let app = yav::server::YavApp::new();
let app = yav::auto_routes!(app);

// Check if a path is registered
assert!(app.has_route("/"));
assert!(app.has_route("/about"));
```

---

## 6. Client-Side Progressive Navigation & View Transitions API

Yav combines the SEO and initial load speed of Server-Side Rendering (SSR) with the seamless feel of a Single Page Application (SPA) using modern web standards:

### 1. The `<Link />` Component
Replaces standard `<a>` tags with background prefetching and SPA transition triggers:

```rust
use yav::ui::Link;

rsx! {
    <nav class="flex gap-4">
        <Link href="/" prefetch=true class="nav-link">
            "Accueil"
        </Link>
        <Link href="/catalogue" prefetch=true class="nav-link">
            "Catalogue"
        </Link>
    </nav>
}
```

### 2. Client Router Runtime (`client_router_script()`)
Inject `client_router_script()` into your root document or layout:

```rust
use yav::ui::client_router_script;

rsx! {
    <body class="bg-gray-950 text-white min-h-screen">
        <Navbar />
        <main id="main-content">
            {children}
        </main>
        <Footer />
        {client_router_script()}
    </body>
}
```

### How It Works:
1. **Intelligent Prefetching**: When the user hovers over a link or scrolls it into view, Yav fetches the target page HTML in the background with `fetch(..., { priority: 'low' })` and caches it in memory.
2. **Native View Transitions API**: When clicked, the router uses `document.startViewTransition()` to morph the `<main>` content seamlessly.
3. **Smooth Shared Animations**: Pages transition fluidly with native CSS keyframes (e.g. cross-fade, slide, or zoom) without loading heavy client-side JavaScript routers.
4. **Browser History & Scroll Restoration**: URL updates via `history.pushState` with full backward/forward browser button navigation and scroll preservation.
5. **Zero-JS Graceful Fallback**: If JavaScript is disabled or unsupported, links execute standard HTTP GET navigations.
