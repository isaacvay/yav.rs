# Yav Architecture

## Overview

Yav is organized as a Rust workspace with clear crate boundaries. Each crate has a well-defined responsibility and minimal coupling to other crates.

## Crate Dependency Graph

```
yav (umbrella re-exports)
├── yav-core (foundation: Node, Element, Signals, StateMachine, CRDTs, Local-First Sync, Wasm Islands, LexoRank, DOM morphing)
├── yav-macros (proc macros: rsx!, #[component], #[server], #[derive(FromFormData, ApiSchema)], auto_routes!, typed_routes!)
├── yav-router (file-based routing, O(L) Radix RouteTrie, parallel slots @slot, intercepting routes (.), optional catch-all [[...slug]])
│   └── yav-core
├── yav-server (Axum HTTP server, SSR, SsrCache ISR, PPR & component cache, passkeys/WebAuthn, OTel tracing, Resilience/CircuitBreaker, Outbox, GeoReplicas, Image Pipeline, PDF Signer, E2E TestClient, MCP server, auth, billing, AI SDK, jobs, adapters, Tus resumable uploads, TurboStream, Event Sourcing, ReBAC, WebPush, Mock API server, i18n linter)
│   ├── yav-core
│   └── yav-router
├── yav-icons (13,000+ embedded icons: Lucide Classic, MynaUI Minimal, Phosphor System)
│   └── yav-core
├── yav-ui (58 enterprise components: Charts, DataGrid, KanbanBoard, FlowCanvas, OrgChart, AudioWaveform, LogViewer, Whiteboard, AiFilterBar, VirtualList, RichTextEditor, AiChat, DevTools, Calendar/Scheduler, Spreadsheet, DiffViewer, MediaPlayer, Image, VectorMap, GanttChart, PdfViewer, SpotlightTour, MediaRecorder, ModelViewer3D, CodePlayground, JsonTreeViewer, HexGrid, SignaturePad, Timeline, ImageCompareSlider, CookieConsent)
│   ├── yav-core
│   └── yav-icons
├── yav-build (package.yavml manifest, Tailwind CLI, asset pipeline, HTML/CSS minifier, search index, PWA generator)
└── yav-cli (CLI developer toolbelt: new, dev, build, export, codegen, db with SQLite/Postgres runners, routes, doctor, analyze, pwa, deploy)
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

### 5. Motion Engine Architecture & Spring Solver
Server-side declarative animations compiled to compact HTML5 datasets, powered on the client by the browser's native **Web Animations API** (WAAPI).
- **Physical Spring Solver**: Accurate harmonic oscillator solver (`solveSpring(stiffness, damping, mass)`) generating natural springs without bulky JS dependencies.
- **Micro-Script Hydration**: Viewport reveal staggering, scroll-spy, animated counters (`CountUp`), expanding gauges (`AnimatedBar`), and syntax-preserving typewriter code blocks.
- **Compile-Time Anti-FOUC**: `yav:initial="opacity:0, y:24"` is pre-compiled directly into inline HTML `style="opacity: 0; transform: translate3d(0px, 24px, 0);"` during SSR to prevent flash of unstyled content before script execution.

### 6. SSR Tree-Shaking Runtimes (`append_tree_shaken_runtimes`)
During HTML document serialization, `yav-core` inspects the DOM string and **tree-shakes client runtimes**:
- Micro-actions runtime is injected only if `yav:toggle`, `yav:copy`, `yav:filter`, etc. are present.
- Cursor tracking runtime is injected only if `yav:cursor`, `<CursorSpotlight>`, `<TiltPlate>`, etc. are present.
- Motion physics runtime is injected only if `yav:animate`, `<Motion>`, `<CountUp>`, etc. are present.
- Signals runtime is injected only if `yav:state`, `yav:bind`, `yav:text` are present.
Pure static pages emit **zero** JavaScript bytes.

### 7. Static VDOM Folding & Zero-Allocation Rendering (`Node::Static`)
Compile-time static chunk folding in the `rsx!` procedural macro identifies immutable DOM subtrees and packs them into `Node::Static(&'static str)`. During SSR rendering, these branches bypass dynamic attribute serialization and string allocations, writing directly to pooled string buffers.

### 8. Automatic HTTP ETag & Conditional Requests (`compute_etag`)
Built-in caching middleware (`YavApp::etag(true)`) computes deterministic SHA-256 ETags over rendered responses. When incoming requests include `If-None-Match`, Yav immediately halts pipeline execution and returns `304 Not Modified`, saving server bandwidth and client battery life.

### 9. View Transitions API & Progressive Client Routing
The client navigation engine (`client_router_script()`) integrates the browser's modern **View Transitions API** (`document.startViewTransition()`):
- **Predictive Prefetching**: Prefetches target HTML over `<Link prefetch=true>` via hover or `IntersectionObserver`.
- **Morphing Page Swaps**: Smooth cross-fades and shared element animations across page changes.
- **100% Graceful Degradation**: Fallback to instant DOM swapping or standard MPA page reloads if JavaScript or View Transitions are unsupported.

### 10. Real-Time WebSockets Live Swap (`WsBroadcast`)
In-memory broadcast channel allowing async server events, background tasks, or pub/sub messages to push HTML fragments directly into client DOM selectors without hand-written client WebSocket handlers.

### 11. Fine-Grained Reactive Signals, Statecharts & CRDT Collaboration (`yav-core`)
- **Reactive Signals (`signals.rs`)**: `Signal<T>`, `ReadSignal<T>`, `WriteSignal<T>`, `Computed<T>`, and `SignalScope` paired with non-destructive DOM morphing (`window.__yavMorph`) and CSP-safe expression evaluation.
- **Statecharts & Saga Workflows (`state_machine.rs`)**: Deterministic `StateMachine<S, E, C>` with guards, actions, and transition audit logs, plus `SagaWorkflow<C>` with automatic reverse-order compensating rollbacks.
- **CRDTs, Presence & LexoRank (`collab.rs`)**: Conflict-free `LwwRegister<T>` and `LwwMap<K, V>`, multiplayer `PresenceRoom` cursor/selection tracking, and `LexoRank` $O(1)$ fractional indexing.

### 12. Enterprise SaaS, AI SDK, Fintech Billing & Pluggable Adapters (`yav-server`)
- **Auth & Security (`auth.rs`)**: PBKDF2-SHA256 password hashing, HS256 `JwtManager`, OAuth2 PKCE (`OAuthConfig`), RFC 6238 `Totp` 2FA, and RBAC `AuthUser`.
- **Feature Flags, Multi-Tenancy, ABAC & Audit (`flags.rs`)**: Deterministic SHA-256 percentage rollouts (`FlagEngine`), B2B `TenantContext`, attribute-based `PolicyEngine`, and tamper-evident SHA-256 hash-chained `AuditTrail`.
- **AI SDK & Hybrid RAG (`ai.rs`)**: `Conversation`, JSON-Schema `ToolRegistry`, `stream_ui_fragment` & `stream_text_sse` SSE streaming, cosine `VectorIndex`, and `reciprocal_rank_fusion`.
- **Billing, Storage & Geospatial (`billing.rs`, `storage.rs`, `geo.rs`)**: Exact integer `Money` with penny-preserving `allocate`, Stripe/Svix `WebhookVerifier`, `IdempotencyStore`, `UsageMeter`, S3/R2 `PresignedUrlSigner`, and Haversine `GeoPolygon` / `SpatialIndex`.
- **Pluggable Infrastructure Adapters (`adapters.rs`)**: `DatabaseAdapter`, `CacheAdapter`, `StorageAdapter`, `LlmAdapter`, `MailAdapter`, and `ServiceAdapters` for connecting Postgres/SQLite, Redis, S3/R2, OpenAI/Anthropic, and SMTP/Resend.

### 13. Dynamic Image Optimization & Blur LQIP (`image_opt.rs` & `Image`)
- Server-side endpoint `GET /_yav/image` with path traversal sanitization, SHA-256 content hashes, and `Cache-Control: public, max-age=31536000, immutable`.
- In-memory generation of lightweight SVG Gaussian blur LQIP data URIs (`generate_blur_svg_data_uri`) for instantaneous zero-CLS loading states.
- Client component `Image` with automatic URL rewriting (`.optimize(true)`), responsive `srcset`, and inline blur background integration (`.blur_placeholder(true)`).

### 14. Automated CSP Nonce Injection into Shell Scripts
- Cryptographic CSP nonces (`CspNonce`) generated per request or extracted from upstream proxy headers (`x-nonce`, `csp-nonce`).
- Injected automatically onto all `<script>` tags across the entire rendered document shell (`HtmlPage::render`), securing micro-actions and tree-shaken runtimes under strict `Content-Security-Policy` policies without `unsafe-inline`.

### 15. Server Actions & Compile-Time RPC Auto-Wiring (`yav-macros`)
- `#[server]` transforms async functions into dual-purpose constructs: directly invokable Rust functions in SSR code, and mountable HTTP `POST` endpoints.
- `auto_routes!(app)` automatically scans route files and wires all `#[server]` functions onto the application router without manual routing boilerplate.
- Declarative validation via `#[derive(FromFormData)]` and OpenAPI schema derivation via `#[derive(ApiSchema)]`.

### 16. Multi-Format Request Body Deserialization (`FormData`)
- Transparent ingestion of `application/json`, `multipart/form-data`, and `application/x-www-form-urlencoded` request bodies into a unified typed dictionary.
- Enables single endpoint handlers to serve HTML forms, mobile JSON clients, and file uploads simultaneously.

### 17. Real-Time Node Graph Rerouting (`FlowCanvas` & Runtime)
- Visual workflow builder with interactive pointer-dragged nodes (`FlowNode`) and cubic Bézier curved edges (`FlowEdge`).
- Embedded client runtime listens for node dragging and continuously recomputes SVG path definitions `M x1 y1 C cx1 y1, cx2 y2, x2 y2` and edge label coordinates in real time.

### 18. Optimistic UI Mutations & Automatic Rollbacks (`yav-core`)
- Instant 0ms perceived latency: user clicks (e.g. `yav:optimistic-toggle`, `yav:optimistic-inc`, `yav:optimistic-set`) trigger immediate client DOM mutations.
- The previous state (classes, innerHTML) is stored in a data snapshot on the DOM node.
- If the background fetch request fails (HTTP 4xx/5xx or network drop), the mutation automatically rolls back and an optional toast notification (`yav:rollback-toast`) is displayed.

### 19. Reactive Live Queries & CDC Server-Push (`yav::server::live_query`)
- High-throughput broadcast channels dispatch table-level and row-level invalidations across server instances.
- Connected clients stream CDC events over Server-Sent Events (`GET /_yav/live`) and selectively trigger DOM morphing or component re-rendering without full-page reloads.

### 20. Cryptographic PoW Anti-Bot Shield & Invisible Honeypots (`yav::server::shield`)
- Client-side Proof-of-Work puzzle solving via Web Workers prevents DDoS, credential stuffing, and bot scrapers.
- Invisible trap fields (`Honeypot::render_input`) silently catch and reject automated spam bots.

### 21. Zero-Dependency GraphQL Engine & GraphiQL Explorer (`yav::server::graphql`)
- Fully typed GraphQL schema definition in pure Rust with synchronous and asynchronous field resolvers.
- A small self-contained, hand-written HTML playground (`graphiql_html`) with no
  CDN assets or external JavaScript. It is **not** the GraphiQL 3.x IDE.

### 22. Geolocation Context & European Union VAT Engine (`yav::server::geo_ip`)
- Automatic edge proxy header resolution (`cf-ipcountry`, `x-country-code`, `x-geo-country`).
- Exact penny-rounded multi-currency tax calculation across all 27 EU member states with standard rates.

### 23. Micro-Frontends & Module Federation Gateway (`yav::server::federation`)
- Declarative upstream route mapping with automatic path stripping and authentication forwarding.
- Real HTTP reverse proxying (`proxy_request`) with per-request timeouts and an explicit upstream host allowlist, plus slot stitching with fallback HTML for resilient multi-team architectures. `fetch_and_stitch_slots(&[FederationSlot])` fetches all fragments in parallel via `join_all` and stitches them in declaration order; `fetch_and_stitch_slot` remains the single-slot entry point. Stitching itself is ordered string replacement, not DOM/`<head>` merging.

### 24. Pure Rust Vector Barcode & QR Code Engine (`yav::server::barcode`)
- Full ISO/IEC 18004 QR code generation with automatic version selection, Reed–Solomon error correction and mask optimisation (via the `qrcode` crate), plus Code 128 (Subset B) barcode generation.
- Emits lightweight vector SVGs with zero native C dependencies.

### 25. Automated Security Auditor & W3C CSP Reporting (`yav::server::security_audit`)
- Real-time OWASP compliance scoring and letter grading (A+ to F) for HTTP response headers.
- Built-in CSP violation collector endpoint (`POST /_yav/csp-report`) with in-memory ring buffer telemetry.

### 26. Next-Generation UI Widgets (`yav-ui`)
- **Web ANSI Terminal (`terminal.rs`)**: High-performance browser terminal emulator with 16-color ANSI SGR formatting and live log streaming.
- **Dynamic Form Builder (`form_builder.rs`)**: Runtime generation of accessible, styled forms directly from JSON Schema.
- **Dual-Pane Markdown Editor (`markdown_editor.rs`)**: Split-view editor with live preview, word count, and comprehensive formatting toolbar.
- **Advanced SVG Charts (`charts_advanced.rs`)**: Calendar activity Heatmaps, Sankey flow diagrams, and multi-metric Radar/Spider charts.

### 27. Local-First & Offline Sync Protocol (`yav-core::local_first`)
- Client mutations are queued immediately in browser storage (`IndexedDB` / `localStorage`) via `OfflineMutation`.
- Optimistic client-side application of changes gives 0ms perceived latency even completely offline.
- When network connectivity resumes, the client emits a `SyncBatch` containing queued mutations with logical timestamps.
- The server processes mutations sequentially, resolving conflicts using Last-Write-Wins (LWW) or custom CRDT merge strategies, and returns a `SyncResponse` with applied or rejected identifiers.

### 28. Zero-Overhead WebAssembly Client Islands (`yav-core::wasm`, `<WasmIsland />`)
- Declarative WebAssembly hydration islands embedded directly into Server-Side Rendered HTML:
  ```rust
  use yav::ui::WasmIsland;
  rsx! {
      <WasmIsland 
          module_url="/wasm/image_filter.wasm" 
          container_id="wasm-canvas" 
          export_fn="init_filter" 
          props_json=r#"{"radius": 5}"# 
      />
  }
  ```
- Uses native browser `WebAssembly.instantiateStreaming` with zero JavaScript bundlers or webpack glue code required.
- Falls back gracefully to server-rendered static HTML placeholders if WebAssembly is unsupported.

### 29. Proactive Resilience: Circuit Breaker, Bulkhead & Concurrency Limiting (`yav-server::resilience`)
- **Circuit Breaker (`CircuitBreaker`)**: Three-state state machine (`Closed`, `Open`, `HalfOpen`) tracking consecutive downstream failures and latency spikes. Trips open automatically to fail fast and prevent thread pool exhaustion, gradually recovering during half-open probes.
- **Bulkhead Isolation (`Bulkhead`)**: Partitions available server execution slots across critical services (e.g. payment processing vs. public analytics), guaranteeing that a degradation in non-critical endpoints cannot starve vital operations.
- **Adaptive Concurrency Limiter (`AdaptiveConcurrencyLimiter`)**: Dynamically scales in-flight request limits according to measured round-trip time (RTT) feedback, preventing queue saturation and latency degradation.

### 30. Transactional Outbox Pattern & Event Bus (`yav-server::outbox`)
- Solves the dual-write problem across distributed databases and message brokers (Kafka, RabbitMQ, Redis).
- State mutations and corresponding domain events are written atomically within the same database transaction into an `outbox` table.
- A background worker (`OutboxProcessor`) continuously polls pending outbox entries, publishes them to the message broker with exponential backoff retries, and marks successfully acknowledged messages as dispatched.

### 31. Multi-Region Database Replica Pool & Sticky Router (`yav-server::replica_pool`)
- Transparent connection routing between a single primary database (writes) and multiple geographically distributed read replicas:
  ```rust
  use yav_server::replica_pool::{DbReplicaPool, ReplicaRole};
  let pool = DbReplicaPool::new(primary_connection_string)
      .add_replica("replica-us-east", us_east_conn)
      .add_replica("replica-eu-west", eu_west_conn);
  ```
- **Read-Your-Own-Writes Consistency**: Automatically pins a user session to the primary writer for a configurable time window (e.g. 5 seconds) after any `POST`/`PUT`/`DELETE` mutation, preventing stale read artifacts.

### 32. Pure-Rust WebP Output & Image Pipeline (`yav-server::image_pipeline`)
- Zero native C/C++ dependencies: image transcoding, metadata inspection, and downsampling implemented entirely in pure Rust.
- **AVIF encoding is not supported**: AVIF *input* can be sniffed and its metadata parsed, but every output of `ImagePipeline` is a WebP container. Use an external encoder for AVIF output.
- **Magic Byte Sniffing**: Reliably detects PNG, JPEG, GIF, WebP, and AVIF byte headers.
- **Pure-Rust WebP VP8 Container Encoder**: Direct RIFF WebP container packing and VP8 intra-frame encoding with bilinear downsampling for responsive `srcset` generation.
- Integrated into `GET /_yav/image` with path traversal guards and immutable cache headers.

### 33. Zero-Dependency Vector PDF Signer & Watermarking (`yav-server::pdf_signer`)
- Stamping digital signatures and visual watermarks directly onto PDF 1.4 byte structures:
  ```rust
  use yav_server::pdf_signer::{PdfSigner, PdfWatermark};
  let signed_pdf = PdfSigner::new("Acme Corp")
      .with_reason("Contract Approval")
      .sign_document(&pdf_bytes, &private_hmac_key)?;
  ```
- Embeds tamper-evident SHA-256 content hashes, RFC 3339 timestamps, and diagonal vector watermark overlays (`CONFIDENTIAL`, `DRAFT`) with zero external C dependencies.

### 34. Multi-Tier Distributed Cache & XFetch Stampede Mitigation (`yav-server::distributed_cache`)
- Combines ultra-fast in-memory L1 cache (`MemoryL1Store`) with pluggable distributed L2 cache (Redis, Memcached).
- **XFetch Probabilistic Early Expiration**:
  $$\Delta t \cdot \beta \cdot \ln(\text{random}()) > (\text{expiry} - \text{now})$$
  As an item nears expiration, concurrent readers probabilistically trigger background recomputation before the cache actually expires, completely eliminating cache stampedes and dogpiling under massive concurrent load.

### 35. Distributed Lock & Leader Election Protocol (`yav-server::distributed_lock`)
- Cluster-wide atomic leases (`DistributedLock`) with millisecond TTL and automatic heartbeat renewal.
- Dynamic leader election (`LeaderElector`) ensuring only a single cluster instance executes singleton background workers (e.g. cron schedulers, billing synchronizations).
- Resilient heartbeat leases with automatic failover if the current leader experiences network partition or node failure.

### 36. ReBAC / Zanzibar Relationship-Based Access Control (`yav-server::rebac`)
- Fine-grained graph authorization inspired by Google Zanzibar:
  ```rust
  use yav_server::rebac::RebacGraph;
  let mut graph = RebacGraph::new();
  graph.add_relation("doc:1", "editor", "user:alice");
  graph.add_relation("doc:1", "viewer", "team:marketing");
  graph.add_relation("team:marketing", "member", "user:bob");
  
  // Transitive resolution:
  assert!(graph.check("user:bob", "viewer", "doc:1"));
  ```
- Supports direct assignments, transitive implications, role inheritance, and cycle-safe graph traversals.

### 37. Prometheus Metrics & Grafana-Ready Dashboard (`yav-server::metrics_dashboard`)
- Built-in metrics collection tracking request throughput, SSR render latencies (p50, p95, p99), memory usage, and active SSE/WebSocket streams.
- Live in-app telemetry dashboard served at `/_yav/metrics/dashboard` with responsive SVG gauges, throughput bar charts, and percentile latency distributions with zero external monitoring dependencies required.

### 38. Phonetic & Typo-Tolerant Fuzzy Search (`yav-build::fuzzy_search`)
- Intelligent typo-tolerant indexing supporting:
  - **Damerau-Levenshtein Distance**: Handles character insertions, deletions, substitutions, and adjacent transpositions.
  - **American Soundex**: Phonetic encoding grouping phonetically similar names and words (e.g. "Smith" and "Smyth").
  - **Query Suggestions**: Generates "Did you mean?" suggestions when search queries yield low direct scores.

### 39. Streaming TurboStream Multi-Zone DOM Mutations (`yav-server::turbo_stream`)
- Hotwire-compatible multi-zone DOM updates streamed over HTTP chunked responses or Server-Sent Events.
- Supports 7 atomic actions (`append`, `prepend`, `replace`, `update`, `remove`, `before`, `after`) targeting specific DOM container IDs:
  ```rust
  use yav_server::turbo_stream::{TurboStream, TurboAction};
  let mutation = TurboStream::new(TurboAction::Update, "notification-badge", "<span class='badge'>3</span>");
  ```

### 40. Event Sourcing & CQRS Aggregate Engine (`yav-server::event_sourcing`)
- Immutable append-only domain event streams (`DomainEvent`, `EventStore`).
- State rehydration of domain aggregates with strict optimistic version concurrency checks.
- Asynchronous read-model projections (`Projection`) generating optimized query views decoupled from write models.

### 41. Resumable Chunked Uploads (Tus Protocol v1.0.0 with Disk Spooling) (`yav-server::resumable_upload`)
- Full implementation of the open Tus v1.0.0 protocol:
  - `POST /files` to initialize an upload session with `Upload-Length` and metadata.
  - `PATCH /files/{id}` to send partial byte chunks with `Upload-Offset`.
  - `HEAD /files/{id}` to query current server offset for network resume after disconnects.
- **Disk Spooling**: Chunks are streamed directly to temporary files on disk (`with_spool_dir`), protecting server memory against OOM DoS when handling gigabyte-scale file uploads.

### 42. RFC 8291/8292 WebPush Protocol & VAPID Authorization (`yav-server::web_push`)
- Native browser push notification dispatching with zero external npm or C dependencies:
  - Generates ECDSA-signed VAPID authorization JWT tokens (`VapidSigner`).
  - Encrypts notification payloads using AES-128-GCM framing per RFC 8291.
  - Directly dispatches HTTP/2 push requests to browser push services (Mozilla, Google FCM, Apple APNs).

### 43. Mock API Server & Realistic Data Faker (`yav-build::mock_server`)
- Embedded mock server for rapid prototyping, frontend development, and E2E test fixtures:
  - Declarative route registration with path pattern matching and method filtering.
  - Configurable synthetic latency and failure rates to simulate edge network conditions.
  - Deterministic fake data generator (`MockFaker`) producing realistic names, emails, addresses, UUIDs, dates, and currency values.

### 44. Static i18n Key Extractor & Translation Linter (`yav-build::i18n_lint`)
- Static analysis engine scanning Rust source files for `t!("...")` macro invocations:
  - Identifies missing translation keys across language catalogs (`en.json`, `fr.json`, etc.).
  - Detects dead or orphaned translation keys present in dictionaries but never referenced in code.
  - Generates comprehensive compliance reports and warning tallies for CI/CD pipelines.

### 45. $O(L)$ Radix Route Trie & Dynamic Route Resolution (`yav-router::trie`)
- High-performance Radix Prefix Tree (`RouteTrieNode`, `RouteTrie`) replacing linear route scanning.
- Route lookup complexity is $O(L)$ where $L$ is the number of path segments, completely independent of the total number of registered routes ($N$).
- Deterministic match precedence:
  1. **Static segments** (`/users/profile`)
  2. **Dynamic parameter segments** (`/users/:id`)
  3. **Catch-all wildcard segments** (`/docs/*slug`)
- Dynamic parameters and wildcards are extracted directly during trie traversal with minimal string allocations.

### 46. Database Scaffolding CLI with SQLite & Postgres Runners (`yav-cli::db_cmd`)
- Built-in migration engine supporting both SQLite and PostgreSQL:
  - Reads connection strings directly from `.env` (`DATABASE_URL`).
  - Applies forward migrations (`yav db migrate`) and rolls back previous steps (`yav db rollback`).
  - Computes and verifies SHA-256 checksums in the `_yav_migrations` journal table to guarantee migration history immutability.




