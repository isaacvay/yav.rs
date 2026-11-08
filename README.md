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
- 📁 **File-based & Reverse Routing** — Next.js-inspired file routing with Route Groups `(auth)`, Parallel Routes `@slot`, Intercepting Routes `(.)`/`(..)`/`(...)`, Optional Catch-All `[[...slug]]`, hierarchical layouts, type-safe reverse routing (`url_for!`), native **View Transitions API**, and predictive prefetching
- 🧩 **60+ Enterprise UI Components & Charts** — Drawers, Command Palette (⌘K), Carousels, Popovers, Tooltips, Sliders, Steppers, KPI StatCards, SVG Charts (`LineChart`, `BarChart`, `DonutChart`, `Sparkline`), `DataGrid`, `KanbanBoard`, `FlowCanvas` (with live Bézier edge rerouting), `VirtualList`, `RichTextEditor`, `AiChat`, `<Image />` (with CLS prevention, on-the-fly `/_yav/image` optimization, and SVG LQIP blur placeholders), `DatePicker`, `Combobox`, `InputOtp`, `FileDropzone`, `Sheet`, `ResizablePanels`, `Kbd`, and Interactive Themes
- 🤝 **Real-Time Collaboration, CRDTs & Statecharts** — Conflict-free replicated registers/maps (`LwwRegister`, `LwwMap`), multiplayer cursor/selection `PresenceRoom`, `LexoRank` $O(1)$ fractional ordering, `StateMachine` (XState equivalent), and `SagaWorkflow` compensating transactions
- 🤖 **Native AI SDK, Streaming UI & Hybrid RAG** — Multi-turn `Conversation`, JSON-Schema `ToolRegistry`, `stream_ui_fragment` / `stream_text_sse`, dense cosine `VectorIndex`, and Reciprocal Rank Fusion (`reciprocal_rank_fusion`) with BM25 `SearchIndex`
- 🧠 **Model Context Protocol (MCP) Server** — Expose live tools, resources, and prompt templates to AI agents (Claude Desktop, Cursor, Copilot) via JSON-RPC 2.0 (`McpServer`)
- 🌓 **Partial Prerendering (PPR) & Component Cache** — Next.js 15-style hybrid static shell with dynamic streaming slots (`PprPage`, `PprSlot`) and microsecond-level component fragment cache (`ComponentCache`)
- 🔑 **WebAuthn / Passkeys Biometric Auth** — Native passwordless FIDO2 authentication (Touch ID, Face ID, Windows Hello, YubiKey) with challenge verification and client scripts (`PasskeyManager`)
- 🔭 **OpenTelemetry (OTLP) & Distributed Tracing** — W3C `traceparent` context propagation, span hierarchy, OTLP JSON telemetry, and Prometheus metrics exporter (`OtelTracer`)
- 🧪 **Full-Stack E2E TestClient & DOM Assertions** — Fast in-memory integration testing with automatic cookie jar persistence, session management, and CSS-like DOM assertions (`TestClient`, `DomAssert`)
- 📄 **Vector PDF 1.4 & RFC 4180 CSV Exporter** — Server-side vector PDF document generation and OWASP formula-injection-safe CSV export (`PdfDocument`, `CsvBuilder`)
- 🛠️ **In-App DevTools & Live Telemetry** — Interactive floating HUD displaying active routes, registered routes, SSR microsecond latency, cache status, and live Core Web Vitals telemetry (`<DevTools />`)
- 📅 **Calendar & Appointment Scheduler** — Server-rendered monthly calendar grid and time-slot booking UI with timezone support (`<Calendar />`, `<Scheduler />`)
- 📊 **Reactive Spreadsheet & Formula Engine** — Excel-like data grid with live formula evaluation (`=SUM()`, `=AVG()`, `=MIN()`, `=MAX()`), cell formatting, and CSV export (`<Spreadsheet />`)
- 🔍 **Git Code Diff Viewer & Custom Media Player** — LCS-powered side-by-side / unified code diffs (`<DiffViewer />`) and accessible HTML5 media player with chapters and subtitles (`<MediaPlayer />`)
- ⚡ **Optimistic UI Mutations & Automatic Rollbacks** — 0ms perceived latency (`yav:optimistic-toggle`, `yav:optimistic-inc`, `yav:optimistic-set`, `yav:rollback-toast`) with automatic client snapshot restoration and toast alerts on server failure
- 📡 **Reactive Live Queries & CDC Server-Push** — Change Data Capture event broadcasting over SSE (`LiveQueryManager`, `LiveEvent`, `live_query_sse_handler`) with targeted DOM morphing
- 🛡️ **Native PoW Anti-Bot Shield & Honeypots** — High-security CAPTCHA-less bot mitigation using client Web Worker Proof-of-Work hashing (`PowShield`, `pow_client_script`) and invisible spam traps (`Honeypot`)
- 🌐 **Zero-Dependency GraphQL Engine & GraphiQL Explorer** — Fully typed GraphQL schema, query executor, and embedded GraphiQL 3.x playground (`GraphQlSchema`, `graphql_handler`, `graphiql_html`)
- 💶 **Geolocation, Multi-Currency & EU VAT Engine** — Automatic edge proxy geo headers resolution (`extract_geo_context`), exact integer multi-currency billing, and penny-accurate European Union VAT across all 27 member states (`VatRule`)
- 🏛️ **Micro-Frontends & Module Federation Gateway** — Orchestrate distributed services with declarative reverse proxying, path rewriting, authentication forwarding, and parallel slot stitching (`FederationGateway`)
- 🏷️ **Pure Rust Vector Barcode 128 & QR Code Engine** — Server-rendered crisp vector SVGs for ISO/IEC 18004 QR codes and Code 128 barcodes (`Barcode128`, `QrCode`) with zero C libraries
- 🔒 **Automated Security Auditor & W3C CSP Reporter** — Real-time OWASP compliance scoring (A+ to F) for HTTP headers and ring-buffered W3C CSP violation reporting (`SecurityAuditor`, `CspReporter`, `csp_report_handler`)
- 💻 **Web ANSI Terminal & Dynamic Form Builder** — In-browser terminal emulator with 16-color ANSI SGR formatting and live streaming (`<Terminal />`), plus instant form generation from JSON Schema (`<DynamicForm />`)
- ✍️ **Dual-Pane Live Markdown Editor** — Split-view Markdown authoring with live preview, word counts, and formatting toolbar (`<MarkdownEditor />`)
- 📈 **Advanced SVG Charts: Heatmap, Sankey & Radar** — Zero-JS vector activity calendar heatmaps (`<Heatmap />`), Sankey flow diagrams (`<SankeyChart />`), and multi-axis spider radar charts (`<RadarChart />`)
- 📱 **Local-First & Offline Sync Protocol** — Seamless offline mutation queuing in IndexedDB with background synchronization, logical clocks, and optimistic UI updates (`OfflineMutation`, `SyncBatch`, `SyncResponse`)
- 🏝️ **Zero-Overhead WebAssembly Client Islands** — Load and hydrate compiled WebAssembly modules directly into DOM containers without JS bundlers (`<WasmIsland />`)
- ⚡ **Circuit Breaker, Bulkhead & Concurrency Limiting** — Proactive resilience engine protecting downstream APIs and connection pools from cascading failures (`CircuitBreaker`, `AdaptiveConcurrencyLimiter`, `Bulkhead`)
- 📮 **Transactional Outbox Pattern & Event Bus** — Guarantee zero data loss and dual-write consistency between DB transactions and distributed brokers (`TransactionalOutbox`)
- 🌍 **Multi-Region Database Replica Pool & Router** — Geo-distributed read replica load-balancing with automatic Read-Your-Own-Writes session stickiness (`DbReplicaPool`)
- 🖼️ **Pure-Rust WebP / AVIF & Image Pipeline** — Zero-dependency format sniffing, metadata extraction, bilinear downsampling, and WebP container packing (`ImagePipeline`, `WebPEncoder`)
- 🌳 **Interactive Org Chart & Family Tree SVG Graph** — Hierarchical organization charts with orthogonal SVG connectors, department pills, and avatars (`<OrgChart />`)
- 🎵 **Interactive Audio Synthesizer & Waveform Visualizer** — Peak amplitude bars, dual-tone progress clipping, scrubbing playhead, and timecodes (`<AudioWaveform />`)
- 🔏 **Zero-Dependency Vector PDF Signer & Watermarking** — Tamper-evident HMAC-SHA256 digital signature stamping and diagonal vector watermarking (`PdfSigner`, `PdfWatermark`)
- 📜 **Native OpenTelemetry Logtail Live Inspector** — Terminal-grade real-time log inspector with level filtering (`INFO`, `WARN`, `ERROR`) and distributed trace links (`<LogViewer />`)
- 🎨 **Collaborative Whiteboard & Drawing Canvas** — Chaikin line-smoothed pen strokes, vector shapes, sticky notes, and multiplayer presence (`<Whiteboard />`)
- 🔍 **Natural Language AI Filter Bar** — Conversational prompt to structured criteria chip converter with instant DataGrid/Table binding (`<AiFilterBar />`)
- ⚡ **Multi-Tier Distributed Cache & XFetch** — Probabilistic early cache recomputation algorithm avoiding cache stampedes and dogpiling under massive concurrent load (`DistributedCache`, `MemoryL1Store`)
- 🔒 **Distributed Lock & Leader Election Protocol** — Cluster-wide atomic lease coordination and dynamic leader election for resilient singleton background workers (`DistributedLock`, `LeaderElector`, `LockCoordinator`)
- 🛡️ **ReBAC / Relationship-Based Access Control** — Fine-grained Zanzibar-style hierarchical graph authorization with direct assignments and transitive role implications (`RebacGraph`)
- 📊 **Prometheus Metrics & Grafana-Ready Dashboard** — Built-in server telemetry dashboard with live SVG gauges, throughput charts, and percentile histograms (`/metrics/dashboard`)
- 🔍 **Phonetic & Typo-Tolerant Fuzzy Search** — Damerau-Levenshtein edit distance, American Soundex phonetic encoding, and intelligent "Did you mean?" query suggestions (`FuzzySearchIndex`, `damerau_levenshtein`, `soundex`)
- 🗺️ **Slippy Tile Vector Map & Geospatial Data Visualizer** — Pure SVG Web Mercator vector map with GPS pin clusters, choropleth country/state polygons, and hover tooltips (`<VectorMap />`)
- 📅 **Interactive Gantt Chart & Project Roadmap** — SVG project timeline with dependency arrows, milestone markers, and live progress bars (`<GanttChart />`)
- 📑 **Vector PDF Document Reader & Annotator** — Zero-dependency vector PDF reader with page navigation controls, zoom toolbar, and SVG annotation highlights (`<PdfViewer />`)
- 🚀 **Spotlight Guided Tour & Onboarding Walkthrough** — Interactive step-by-step onboarding walkthrough with cutout dark masks, element spotlighting, and local storage state persistence (`<SpotlightTour />`)
- 🎙️ **Voice Memo & Live Audio Recorder** — Microphone capture component with animated frequency waveforms, timer indicators, playback preview, and direct upload bindings (`<MediaRecorder />`)
- 🧊 **Pure-Rust 3D Wireframe & Mesh Visualizer** — Pure-Rust 3D linear algebra (Euler rotations, perspective projections) rendered directly into SVG wireframes and shaded polygon meshes with mouse-drag orbit (`<ModelViewer3D />`)
- 💻 **Interactive Code Sandbox & REPL** — Split-pane live code editor with line numbers gutter, tab indentation, copy button, Ctrl+Enter hotkey runner, and remote API sandbox integration (`<CodePlayground />`)
- ⚡ **TurboStream Multi-Zone DOM Mutations** — Stream targeted HTML fragments over SSE/HTTP to append, prepend, replace, or remove DOM containers without page reload (`TurboStream`, `TurboMessage`, `TurboAction`)
- 🏛️ **Event Sourcing & CQRS Aggregate Engine** — Immutable event streams with strict optimistic version concurrency, aggregate state rehydration, and read-model projections (`EventStore`, `DomainEvent`, `Projection`)
- 📁 **Resumable Chunked Uploads (Tus Protocol v1.0.0)** — Network interruption recovery, partial binary chunk concatenation, and checksum validation (`ChunkedUploader`, `UploadSession`)
- 🔔 **RFC 8291/8292 Web Push & VAPID Authorization** — Zero-dependency Web Push notification dispatching with VAPID JWT generation (`WebPushNotification`, `VapidSigner`, `PushSubscription`)
- 🎭 **Mock API Server & Realistic Data Faker** — Route matching, latency simulation, and deterministic fake data generator for isolated frontend testing (`MockServer`, `MockEndpoint`, `MockFaker`)
- 🌐 **Static i18n Key Extractor & Translation Linter** — Static analysis scanning templates for `t!("key")` invocations, detecting missing and orphaned translation keys (`scan_source_code`, `lint_catalog`)
- 🌳 **Interactive JSON / YAML Tree Explorer** — Collapsible syntax-highlighted tree viewer with depth limits, item counts, and JSONPath copy buttons (`<JsonTreeViewer />`)
- ⬡ **Hexagonal Grid & Isometric Spatial Mapping** — Pure-SVG pointy-topped hex grid with axial coordinates, Manhattan distance calculation, and interactive click handlers (`<HexGrid />`)
- ✍️ **Vector Handwritten Signature Pad** — Zero-JS canvas touch and mouse signature capture with clear button and base64 hidden form synchronization (`<SignaturePad />`)
- ⏳ **Chronological Timeline & Audit Trail** — Vertical status timeline with connecting lines, timestamp tags, actor pills, and rich descriptions (`<Timeline />`)
- 🔀 **Before / After Image Comparison Slider** — Interactive dual-image split-view slider with draggable divider handle and custom labels (`<ImageCompareSlider />`)
- 🍪 **GDPR & ePrivacy Cookie Consent Preference Center** — Accessible cookie banner with category preference toggles, localStorage persistence, and DOM dispatch events (`<CookieConsent />`)
- 🏢 **Enterprise Auth, Multi-Tenancy, ABAC & Billing** — JWT HS256, OAuth2 PKCE, PBKDF2-SHA256, TOTP 2FA, `FeatureFlag` rollout engine, `TenantContext`, ABAC `PolicyEngine`, SHA-256 hash-chained `AuditTrail`, exact integer `Money` with penny-allocation, Stripe/Svix `WebhookVerifier`, `IdempotencyStore`, S3/R2 `PresignedUrlSigner`, Geospatial `GeoPolygon`/`SpatialIndex`, and pluggable `ServiceAdapters`
- 🎯 **Built-in icons** — 13,000+ icons embedded directly in the binary across three design systems: **Lucide** (Classic), **MynaUI** (Minimal), and **Phosphor** (System)
- 🌊 **Tailwind CSS** — Standalone CLI, zero Node.js / npm dependencies required
- 🗜️ **Asset Pre-Compression, Minifier & SSG** — Automatic transparent Brotli (`.br`) & Gzip (`.gz`) serving, built-in HTML/CSS minifier, GFM Markdown + syntax highlighting, and full Static Site Generation with minified pages via `yav export`
- 📥 **Compile-Time Macros (`#[derive(FromFormData, ApiSchema)]`, `#[server]`, `auto_routes!`)** — Zero-boilerplate form validation, multi-format body ingestion (JSON, Multipart, URL-encoded), OpenAPI 3.1 + TypeScript interface generation, RPC Server Actions with auto-wiring, and `typed_routes!` reverse routing
- 🌐 **Async Context, i18n & Fine-Grained Reactive Signals** — Scoped request state (`AppState` / `use_state`), XSS-safe translation catalogs (`t!`), `Signal<T>` / `Computed<T>` reactive primitives, non-destructive DOM morphing (`window.__yavMorph`), and automated CSP nonce injection into `<script nonce="...">` tags
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
- [Components Guide (58 primitives & RSX Control Flow)](docs/components.md)
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
