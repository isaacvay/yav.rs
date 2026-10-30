<p align="center">
  <img src="assets/svg/yav-monogram-gradient.svg" alt="Yav.rs Logo" width="140" height="140" />
</p>

<h1 align="center">Yav.rs</h1>

<p align="center">
  <strong>A Rust-first Web Framework with a modern developer experience</strong><br>
  <em>Write Rust. Keep HTML and CSS. Build modern interfaces without unnecessary tooling friction.</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/yav"><img src="https://img.shields.io/crates/v/yav.svg?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/yav"><img src="https://img.shields.io/docsrs/yav?style=flat-square" alt="docs.rs"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue?style=flat-square" alt="License"></a>
  <a href="https://github.com/isaacvay/yav.rs"><img src="https://img.shields.io/badge/rust-2021%20edition-black?style=flat-square&logo=rust" alt="Rust Edition"></a>
</p>

---

Yav.rs is a complete web framework that brings the developer experience of Next.js and the integrated UI philosophy of Flutter to Rust. Server-side rendering, file-based routing, a JSX-like component syntax, built-in UI components, and Tailwind CSS integration — all without requiring Node.js.

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
- 🎨 **JSX-like syntax with Native Control Flow** — Declarative UI with `rsx!`, native `if`/`else` and `for` loops, short boolean attributes, class directives (`class:active={is_active}`), and compile-time static subtree folding (`Node::Static`)
- 🎬 **Zero-JS Motion & Spring Physics** — Damped harmonic oscillator solver (`solveSpring`), gestures, scroll reveals, scroll-spy, count-up numbers, animated gauges, and syntax-preserving typewriter code blocks (<2KB WAAPI runtime, SSR tree-shaken)
- 🎯 **Cursor Tracking & Reactive 3D Effects** — Native mouse coordinate CSS variables (`--yav-mouse-x/y`), cursor spotlight, 3D tilt plates, magnetic hover attraction, and smooth cursor followers
- 🎭 **8 Built-in Design Themes** — Seamless switching between Editorial Blueprint, Obsidian Minimal, Nordic Paper, Cyber Terminal, Solarized Lab, Glass Aurora, Neo Brutalism, and Sunset Velvet (`ThemeSelector`, theme cookies, zero-FOUC)
- 🖥️ **Server-Side Rendering & Streaming** — HTML-first with instant initial render, async chunk streaming, automatic locale injection, and zero-clone `<head>` hoisting (`<Head>`)
- 📁 **File-based & Reverse Routing** — Next.js-inspired file routing with Route Groups `(auth)`, hierarchical layouts (`route_with_layout`), type-safe reverse routing (`url_for!`), native **View Transitions API**, and predictive prefetching
- 🧩 **Rich UI component library** — 45+ primitives: Drawers, Command Palette (⌘K), Carousels, Popovers, Tooltips, Sliders, Steppers, KPI StatCards, Syntax CodeBlock, and Interactive Themes
- 🎯 **Built-in icons** — 13,000+ icons embedded directly in the binary across three design systems: **Lucide** (Classic), **MynaUI** (Minimal), and **Phosphor** (System)
- 🌊 **Tailwind CSS** — Standalone CLI, zero Node.js / npm dependencies required
- 🗜️ **Asset Pre-Compression & SSG** — Automatic transparent Brotli (`.br`) & Gzip (`.gz`) serving, plus full Static Site Generation via `yav export`
- 📥 **Validated Forms & Uploads** — `FromFormData` trait and derive for typed validation, `UploadedFile` with MIME-type, extension, and file size enforcement
- 🌐 **Async Context, i18n & Micro-Signals** — Scoped request state (`AppState` / `use_state`), XSS-safe translation catalogs (`t!`), and declarative micro-signals (`yav:state`)
- ⚡ **Hot reload, Error Overlay & WebSocket Live Swap** — Live browser reload, compiler error overlay, and real-time DOM pushing with `WsBroadcast`
- 📦 **Single-file manifest** — `package.yavml` configures metadata, server, Tailwind, scripts & deps
- 🚀 **Automatic HTTP & SSR Caching** — In-memory SSR route caching (`get_cached`), Blake3/SHA-256 ETags with 304 Not Modified conditional responses
- 🛡️ **Production-Ready Security** — HMAC-SHA256 session cookies, CSPRNG CSRF token salting, sliding-window rate limiting, and security headers
- 🩺 **Developer Diagnostics & Route Inspection** — `yav doctor` for full system auditing and `yav routes` for tabular inspection of routes and handlers
- ♿ **Accessible** — Semantic HTML, keyboard micro-actions (`yav:hotkey`), and full ARIA compliance

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
| `yav doctor` | Run diagnostic audit on toolchains, tailwind, git, and project health |
| `yav routes` | Inspect file-based route table, HTTP handlers, dynamic parameters, and layouts |
| `yav export` | Static site generation (SSG) - export HTML pages to `dist/` |
| `yav update` | Update the `yav` CLI binary and current project dependencies (`aliases: up, upgrade`) |
| `yav add <component>` | Add UI templates (`drawer`, `command-menu`, `theme-selector`, `motion`, `cursor-spotlight`, etc.) |
| `yav run <script>` | Run a custom script defined in `package.yavml` |
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
- [Zero-JS Declarative Interactivity & Motion Engine](docs/zero-js-interactions-and-motion.md)
- [Multi-Theme System (8 Design Themes)](docs/themes.md)
- [Routing, Layouts & Static Site Generation](docs/routing.md)
- [Components Guide (45+ primitives & RSX Control Flow)](docs/components.md)
- [Icons System (Lucide, MynaUI & Phosphor)](docs/icons.md)
- [Styling & Tailwind CSS Engine](docs/styling.md)
- [Interactivity, HTMX & WebSocket Live Swap](docs/interactivity-and-htmx.md)
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
