# Yav — A Rust-first Web Framework

> Write Rust. Keep HTML and CSS. Build modern interfaces without unnecessary tooling friction.

Yav is a complete web framework that brings the developer experience of Next.js and the integrated UI philosophy of Flutter to Rust. Server-side rendering, file-based routing, a JSX-like component syntax, built-in UI components, and Tailwind CSS integration — all without requiring Node.js.

## Quick Start

```bash
# Install the CLI
cargo install yav-cli

# Create a new project
yav new my-app
cd my-app

# Start the development server
yav dev
```

## Features

- 🦀 **Rust-first & Zero-Allocation VDOM** — Write your entire application in Rust with full type-safety and `Cow<'static, str>` zero-copy rendering
- 🎨 **JSX-like syntax** — Declarative UI with the `rsx!` macro and `<> ... </>` fragments
- 🎬 **Motion & Animations** — Framer Motion-inspired animations (`yav-motion`), spring physics, gestures & scroll reveals (<2KB WAAPI runtime)
- 🖥️ **Server-Side Rendering & Streaming** — HTML-first with instant initial render and async chunk streaming
- 📁 **File-based & Route Groups** — Next.js-inspired file routing with Route Groups `(auth)`, hierarchical layouts, native **View Transitions API**, and predictive prefetching
- 🧩 **Rich UI component library** — 40+ primitives: Drawers, Command Palette (⌘K), Carousels, Popovers, Tooltips, Sliders, Steppers, and KPI StatCards
- 🎯 **Built-in icons** — 13,000+ icons embedded directly in the binary across three design systems: **Lucide** (Classic), **MynaUI** (Minimal), and **Phosphor** (System)
- 🌊 **Tailwind CSS** — Standalone CLI, zero Node.js / npm dependencies required
- 🗜️ **Asset Pre-Compression** — Automatic transparent Brotli (`.br`) & Gzip (`.gz`) serving based on client `Accept-Encoding`
- 📥 **Validated File Uploads** — `UploadedFile` with built-in MIME-type, extension, and file size validation
- 🌐 **Async Context & i18n** — Scoped context across async/thread boundaries and pluralization catalogs
- ⚡ **Hot reload & Error Overlay** — Automatic browser refresh and live dark-mode compiler error overlay
- 📦 **Single-file manifest** — `package.yavml` configures metadata, server, Tailwind, scripts & deps
- 🚀 **Automatic HTTP Caching** — Intelligent Blake3/SHA-256 ETags with 304 Not Modified conditional responses
- 🛡️ **Production-Ready Security** — HMAC session cookies, sliding-window rate limiting, and security headers
- ♿ **Accessible** — Semantic HTML and full ARIA compliance

## Writing Components & Motion

```rust
use yav::prelude::*;
use yav::ui::{Motion, MotionState, Transition, Button};

#[component]
pub fn Hero() -> Node {
    let title_motion = Motion::new(rsx! {
        <h1 class="text-5xl font-black text-white">"Vitesse & Élégance"</h1>
    })
    .initial(MotionState::new().opacity(0.0).y(20.0).scale(0.95))
    .animate(MotionState::new().opacity(1.0).y(0.0).scale(1.0))
    .transition(Transition::spring().stiffness(180.0).damping(16.0));

    rsx! {
        <main class="min-h-screen bg-gray-950 p-12 flex flex-col items-center justify-center">
            {title_motion}
            <p class="mt-4 text-gray-400 text-lg">
                "Framework web full-stack ultra-performant propulsé par Rust."
            </p>
            <div class="mt-8 flex gap-4">
                <Button variant="primary">"Explorer le catalogue"</Button>
            </div>
        </main>
    }
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `yav new <name>` | Create a new project with Tailwind & file routing |
| `yav dev` | Start development server with live reload and error overlay |
| `yav add <component>` | Add UI templates (`drawer`, `command-menu`, `stat-card`, `carousel`, etc.) |
| `yav codegen --ts` | Generate TypeScript definitions from Rust structs |
| `yav codegen --openapi`| Export OpenAPI 3.1 specifications |
| `yav build` | Build optimized release binary and purged CSS |
| `yav analyze` | Audit bundle sizes, routes, assets, and performance scorecard |
| `yav pwa` | Scaffold PWA manifest, service worker, and offline page |
| `yav check` | Validate project manifest and code syntax |
| `yav test` | Run workspace test suite |
| `yav ext install` | Install/update Yav extension in VS Code, Cursor, Insiders, and VSCodium |
| `yav ext status` | Check installation status of Yav extension across detected editors |
| `yav deploy <target>`| Generate deployment configs for Fly.io, Shuttle.rs, Railway, and Systemd |
| `yav docker` | Scaffold multi-stage production Docker deployment |
| `yav db` | Database migrations and connection management |

## Architecture

```
crates/
├── yav/          # Umbrella crate (re-exports)
├── yav-core/     # Node types, rendering, escaping
├── yav-macros/   # rsx! and #[component] proc macros
├── yav-router/   # File-based routing
├── yav-server/   # Axum HTTP server + SSR
├── yav-ui/       # UI component library
├── yav-icons/    # Built-in icon system
├── yav-build/    # package.yavml manifest + Tailwind CSS + asset pipeline
└── yav-cli/      # CLI tool
```

## Documentation

Comprehensive guides are available in the [`docs/`](docs/) directory:

- [Architecture & Design Principles](docs/architecture.md)
- [Getting Started](docs/getting-started.md)
- [Routing & Dynamic Routes](docs/routing.md)
- [Components Guide (40+ primitives)](docs/components.md)
- [Icons System (Lucide, MynaUI & Phosphor)](docs/icons.md)
- [Styling & Tailwind CSS Engine](docs/styling.md)
- [Interactivity & Client Islands](docs/interactivity-and-htmx.md)
- [SEO, Open Graph & Structured Data](docs/seo-and-metadata.md)
- [Development Server & HMR](docs/development-server.md)
- [Production Optimization & Deployment](docs/production.md)
- [Security & Threat Mitigation](docs/security.md)
- [CLI Reference](docs/cli.md)
- [Configuration Reference](docs/configuration.md)

## Examples

- **[Enterprise SaaS Dashboard (`examples/dashboard`)](examples/dashboard/)**: Full-featured enterprise application showcasing KPI stat cards, dynamic route parameter resolution (`/projects/:id`), interactive component showroom, searchable icon explorer (Lucide, MynaUI, Phosphor), and CSRF-protected settings form.

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Acknowledgments

- [Lucide Icons](https://lucide.dev/) — ISC License
- [MynaUI Icons](https://mynaui.com/) — MIT License
- [Phosphor Icons](https://phosphoricons.com/) — MIT License
- [Tailwind CSS](https://tailwindcss.com/) — MIT License
- [Axum](https://github.com/tokio-rs/axum) — MIT License

## Author

Crafted by **[Isaac Yav](https://github.com/isaacvay)**.
