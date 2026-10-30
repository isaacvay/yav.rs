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

---

## 7. Reverse Routing with `url_for!`

Rather than hard-coding URL strings throughout your views, use the type-safe `url_for` helper to interpolate route parameters:

```rust
use yav::router::url_for;

// Single dynamic parameter
let user_url = url_for("/users/:id", &[("id", "42")]);
assert_eq!(user_url, "/users/42");

// Wildcard catch-all parameter
let doc_url = url_for("/docs/*slug", &[("slug", "getting-started/quickstart")]);
assert_eq!(doc_url, "/docs/getting-started/quickstart");

// Multiple parameters
let post_url = url_for("/posts/[year]/[slug]", &[("year", "2026"), ("slug", "release-v014")]);
assert_eq!(post_url, "/posts/2026/release-v014");
```

---

## 8. Programmatic Route Layouts (`route_with_layout`)

You can register route endpoints with an explicit wrapper layout function directly on `YavApp`:

```rust
use yav::prelude::*;
use axum::http::Method;

fn admin_layout(child: Node) -> Node {
    rsx! {
        <div class="admin-shell flex">
            <aside class="sidebar w-64 bg-zinc-900 text-white p-6">
                <h2>"Admin Panel"</h2>
            </aside>
            <main class="flex-1 p-8">
                {child}
            </main>
        </div>
    }
}

async fn dashboard_page() -> HtmlPage {
    HtmlPage::new(rsx! { <h1>"Dashboard Metrics"</h1> }).title("Admin Dashboard")
}

let app = YavApp::new()
    .route_with_layout("/admin/dashboard", Method::GET, dashboard_page, admin_layout);
```

---

## 9. SSR Route Caching (`get_cached` & `route_cached`)

Accelerate high-traffic or expensive server-rendered pages with in-memory SSR caching:

```rust
use yav::prelude::*;
use std::time::Duration;

let app = YavApp::new()
    // Cache the /explore page HTML for 60 seconds
    .get_cached("/explore", Duration::from_secs(60), || async {
        HtmlPage::new(rsx! {
            <h1>"Catalogue Exploratoire"</h1>
            <p>"Contenu calculé une seule fois toutes les 60 secondes."</p>
        })
    });
```

* Pages served from `SsrCache` return instantly with zero redundant CPU rendering.
* Automatically includes appropriate `ETag` and `Last-Modified` cache headers.

---

## 10. Request-Scoped App State (`AppState` & `use_state`)

Share global or request-scoped dependency injection across your handlers and components during SSR without passing props down the entire tree:

```rust
use yav::prelude::*;

#[derive(Clone)]
struct SiteConfig {
    site_title: String,
    support_email: String,
}

let app = YavApp::new()
    .state(SiteConfig {
        site_title: "Yav Documentation".to_string(),
        support_email: "contact@yav.rs".to_string(),
    })
    .get("/contact", || async {
        let config = use_state::<SiteConfig>().expect("SiteConfig injected");
        HtmlPage::new(rsx! {
            <h1>{config.site_title}</h1>
            <p>"Contactez-nous à: " {config.support_email}</p>
        })
    });
```

---

## 11. Static Site Generation (SSG) with `export_static`

In addition to dynamic SSR, `YavApp` can statically export every registered page into pre-rendered static HTML files:

```rust
let app = YavApp::new()
    .get("/", home_handler)
    .get("/about", about_handler)
    .get("/pricing", pricing_handler);

let out_dir = std::path::Path::new("dist");
let count = app.export_static(out_dir, "https://yav.rs").await?;
println!("Successfully pre-rendered {count} static HTML pages to dist/");
```

Alternatively, invoke `yav export` from the CLI:
```bash
yav export --output dist --base-url https://yav.rs
```
