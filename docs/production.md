# Production Build & Deployment

Yav is engineered to produce self-contained, high-performance production binaries ready for cloud or bare-metal deployment.

## Production Build

To run an optimized production build:

```bash
yav build
```

### What `yav build` Does

1. **Manifest Validation**: Validates `package.yavml` configuration, ports, assets, and script settings.
2. **Tailwind Minification**: Executes standalone Tailwind CLI in production mode (`--minify`) to purge unused utility classes.
3. **Asset Fingerprinting & Pre-Compression**: Copies files from `public/` into the output directory (`dist/`), computes SHA-256 content hashes, and generates pre-compressed `.gz` (Gzip) and `.br` (Brotli) variants in a single in-memory pass.
4. **Release Compilation**: Runs `cargo build --release` with optimization settings.
5. **Manifest Generation**: Writes `dist/asset-manifest.json` mapping logical paths to fingerprinted filenames for long-term browser caching (`HtmlPage` automatically caches and resolves fingerprinted URLs in production via `OnceLock`).

---

## Static Site Generation (SSG)

If your website consists entirely or partially of static content (documentation, blogs, marketing landing pages), you can pre-render every route to static HTML:

```bash
# Export all static routes to dist/
yav export --output dist --base-url https://mon-site.com
```

Programmatically from Rust:
```rust
let app = YavApp::new().get("/", home_handler).get("/docs", docs_handler);
let count = app.export_static(Path::new("dist"), "https://mon-site.com").await?;
```
The resulting `dist/` directory can be deployed directly to Cloudflare Pages, GitHub Pages, Netlify, or Vercel without requiring a running server process.

---

## Partial Prerendering (PPR) & Component Cache (`yav::server::ppr`)

Yav provides Next.js 15-style Partial Prerendering (PPR), combining the ultra-fast TTFB of pre-rendered static HTML shells with server-rendered streaming holes for dynamic user content.

```rust
use yav::server::ppr::{PprPage, PprSlot, ComponentCache, global_component_cache};

// 1. Define static shell with dynamic streaming slot
let page = PprPage::new(
    "<div class=\"layout\"><header>Static Header</header><main><!-- PPR_SLOT:feed --></main></div>"
).with_slot("feed", PprSlot::streaming("/api/user/feed", "<div>Loading personalized feed...</div>"));

let response_html = page.render_shell();
assert!(response_html.contains("yav:ppr-slot=\"feed\""));

// 2. High-performance Component Fragment Cache (TTL-based)
let cache = ComponentCache::new(500); // 500 items max
cache.put("user_card_42", "<div>User 42 Profile</div>", std::time::Duration::from_secs(60));
if let Some(cached_html) = cache.get("user_card_42") {
    // Zero-cost cache hit
}
```

---

## WebAuthn & Passkeys Biometric Auth (`yav::server::passkeys`)

Native, passwordless authentication using FIDO2 / WebAuthn biometric passkeys (Touch ID, Face ID, Windows Hello, YubiKey).

```rust
use yav::server::passkeys::{PasskeyManager, yav_passkey_script};

let manager = PasskeyManager::new("Acme SaaS", "acme.com", "https://acme.com");

// 1. Generate registration challenge for new user
let reg_opts = manager.create_registration_options("user_123", "alice@acme.com", "Alice");

// 2. Client initiates WebAuthn via client-side script:
let script = yav_passkey_script("/api/auth/passkey/register", "/api/auth/passkey/authenticate");

// 3. Generate authentication challenge for login
let auth_opts = manager.create_auth_options();
```

---

## OpenTelemetry (OTLP) & Distributed Tracing (`yav::server::otel`)

Zero-dependency W3C `traceparent` propagation and OTLP / Prometheus metrics exporting for Datadog, Grafana, Honeycomb, and New Relic.

```rust
use yav::server::otel::{OtelTracer, TraceParent, otel_tracing_middleware, global_tracer};

let tracer = OtelTracer::new("yav-production-app");

// Create child span from incoming W3C traceparent header:
let incoming_tp = TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").unwrap();
let span = tracer.start_span("database_query").with_parent(&incoming_tp);
span.end_with_status("ok");

// Export to OTLP JSON (for collector) or Prometheus text format:
let otlp_payload = tracer.export_otlp_json();
let prom_metrics = tracer.export_prometheus();
```

---

## Full-Stack E2E TestClient & DOM Assertions (`yav::server::testing`)

Simulate complete full-stack browser sessions with automatic cookie jar persistence, CSRF tokens, and CSS-like DOM assertions in millisecond test runs.

```rust
use yav::server::testing::{TestClient, DomAssert};
use yav::server::{YavApp, html_response, StatusCode};

let app = YavApp::new().get("/dashboard", |_ctx| async {
    html_response(StatusCode::OK, "<h1 class=\"text-2xl\">Welcome Alice</h1><div id=\"stats\">120 active</div>")
});

let client = TestClient::new(app);
let res = client.get("/dashboard").await;

assert_eq!(res.status, 200);
let dom = res.dom();
dom.assert_text_contains("h1", "Welcome Alice");
dom.assert_text_contains("#stats", "120 active");
```

---

## Model Context Protocol (MCP) Server (`yav::server::mcp`)

Turn your Yav application into an AI Agent Tool provider via the Model Context Protocol (MCP JSON-RPC 2.0). Expose live database records, prompts, and server actions to Claude Desktop, Cursor, and autonomous agents.

```rust
use yav::server::mcp::{McpServer, McpResource, McpPrompt};

let mut mcp = McpServer::new("yav-store-mcp", "1.0.0");
mcp.register_tool("get_product", "Fetch product by ID", serde_json::json!({
    "type": "object",
    "properties": { "id": { "type": "string" } },
    "required": ["id"]
}));

// Mount as Axum / Yav route:
let mcp_handler = mcp.into_axum_handler();
```

---

## Native Vector PDF & CSV Exporter (`yav::server::export_docs`)

Generate pixel-perfect vector PDF 1.4 reports and RFC 4180 CSV exports with OWASP formula-injection protection directly on the server without external binaries.

```rust
use yav::server::export_docs::{PdfDocument, CsvBuilder, pdf_response, csv_response};

// 1. Vector PDF generation:
let pdf_bytes = PdfDocument::new("Facture #INV-2026-001")
    .author("Acme Corp")
    .heading("Facture #INV-2026-001")
    .paragraph("Client: Entreprise ABC")
    .table_header(vec!["Description", "Quantité", "Prix"])
    .table_row(vec!["Abonnement Yav Pro", "1", "49.00 €"])
    .build();

// 2. OWASP-safe CSV generation:
let csv_data = CsvBuilder::new()
    .columns(&["Nom", "Email", "Statut"])
    .row(&["Alice", "alice@example.com", "Actif"])
    .build();
```

---

## Progressive Web App (PWA) (`yav::build::pwa` & `yav pwa`)

Generate offline-first Progressive Web Apps with service workers, asset pre-caching, offline fallback screens, and web manifests:

```bash
# Scaffold PWA assets into public/
yav pwa
```

Or programmatically generate manifests and service workers:
```rust
use yav::build::pwa::{PwaConfig, generate_webmanifest, generate_service_worker, pwa_register_script};

let config = PwaConfig::new("Mon App", "/");
let manifest = generate_webmanifest(&config);
let sw_code = generate_service_worker(&config);
```

---

## In-Memory SSR Route Caching & ISR (`SsrCache`)

For dynamic servers handling high traffic loads, Yav provides bounded LRU in-memory SSR caching with Instant Server-Side Revalidation (ISR):

```rust
use std::time::Duration;
use yav::server::{YavApp, SsrCache, CachePolicy};

let cache = SsrCache::new(500); // Max 500 cached pages (LRU eviction)

let app = YavApp::new()
    // Cache the /blog page HTML for 120 seconds with cache tags
    .get_cached("/blog", Duration::from_secs(120), blog_index_handler);
```

### Tag-Based & Path-Based Revalidation (ISR):
Just like in Next.js ISR, you can invalidate cached pages instantly when data changes in your database or CMS:
```rust
// Invalidate all pages tagged with "posts"
cache.revalidate_tag("posts");

// Invalidate a specific path
cache.revalidate_path("/blog/first-post");
```
- Completely bypasses template rendering, component execution, and database hits while the cache is fresh.
- Automatically generates ETags and serves `304 Not Modified` responses when possible.

---

## Dynamic Image Optimization (`/_yav/image`)

Yav includes a built-in image optimization endpoint that serves resized, format-negotiated, and cached local assets on the fly:
- Endpoint: `GET /_yav/image?url=/hero.jpg&w=800&q=80`
- **Cache-Control**: Emits `public, max-age=31536000, immutable` and SHA-256 `ETag`.
- **Security**: Strict path-traversal prevention (`resolve_safe_asset_path`), rejecting `..`, `\`, and parent directory escapes.
- **SVG LQIP Blur Placeholders**: Supports `?blur=true` or [`generate_blur_svg_data_uri`] to serve ultra-lightweight SVG Gaussian-blur placeholders (~200 bytes) for zero-CLS loading states.

---

## HTML & CSS Minification Pipeline (`yav_build::minifier`)

Yav includes a fast, standalone minifier for production assets:
- **`minify_css`**: Strips whitespace, comments, trailing semicolons, and compacts CSS rules.
- **`minify_html`**: Collapses inter-tag whitespace, removes HTML comments, and compresses text nodes while preserving `<pre>` and `<code>` blocks.
- **Automated Execution**: Applied automatically during `yav build` on all static assets in `public/` and during `yav export` on all pre-rendered HTML pages.

---

## Pluggable Infrastructure Adapters (`ServiceAdapters`)

Isolate your domain logic from external cloud providers with Yav's pluggable service adapter architecture:

| Adapter Trait | In-Memory / Local Implementation | Production Cloud Implementation |
|---------------|----------------------------------|---------------------------------|
| `DatabaseAdapter` | `EmbeddedDatabaseAdapter` (in-memory SQLite/K-V) | PostgreSQL, MySQL, LibSQL |
| `CacheAdapter` | `MemoryCacheAdapter` (bounded LRU) | Redis, Dragonfly, Memcached |
| `StorageAdapter` | `MemoryStorageAdapter` | AWS S3, Cloudflare R2, MinIO |
| `LlmAdapter` | `LocalLlmAdapter` (deterministic mock) | OpenAI, Anthropic, Gemini, Ollama |
| `MailAdapter` | `MemoryMailAdapter` (in-memory outbox) | SMTP, Resend, Postmark, SendGrid |

Bundle all adapters into a single injectable `ServiceAdapters` struct and register it with `YavApp::state(adapters)`.

---

## Background Job Queue & Cron Scheduler (`JobQueue`)

Execute asynchronous background tasks with exponential backoff retries and recurring interval cron schedules:

```rust
use yav_server::jobs::JobQueue;
use std::time::Duration;

let queue = JobQueue::new();

// Enqueue a background task with retry policy
let job_id = queue.enqueue_with_retry(
    "send_welcome_email",
    r#"{"user_id": 42}"#,
    3, // 3 max retries
    Duration::from_secs(5), // exponential retry backoff
);

// Register a recurring scheduled interval
queue.schedule_interval("cleanup_sessions", Duration::from_secs(3600));
```

---

## Transactional Email Engine (`EmailMessage::render_rsx`)

Render type-safe HTML emails using RSX components with automatic plain-text extraction:

```rust
use yav_server::email::{EmailMessage, Mailer};
use yav::prelude::*;

let email = EmailMessage::new("bienvenue@monsite.fr", "user@example.com", "Bienvenue sur Yav !")
    .render_rsx(rsx! {
        <div style="font-family: sans-serif; padding: 20px;">
            <h1>"Bienvenue à bord !"</h1>
            <p>"Votre compte a été activé avec succès."</p>
        </div>
    });

// Automatically produces:
// 1. email.html -> formatted HTML email markup
// 2. email.plain_text -> clean text fallback without HTML tags
```

---

## Fintech Billing, Exact Integer Money & Idempotency

Yav provides zero-floating-point financial accounting primitives:

### Exact Money & Penny Allocation (`Money`)
Stores values in minor units (cents/pence) as 64-bit integers with ISO 4217 currencies, eliminating floating-point rounding errors:
```rust
use yav_server::billing::{Money, Currency};

let price = Money::new(1000, Currency::EUR); // 10.00 EUR
// Allocate among 3 partners preserving exact cents (3.34, 3.33, 3.33):
let shares = price.allocate(&[1, 1, 1])?;
assert_eq!(shares[0].amount(), 334);
assert_eq!(shares[1].amount(), 333);
assert_eq!(shares[2].amount(), 333);
```

### Idempotency Store (`IdempotencyStore`)
Prevents double-charging or duplicate API mutations on transient network retries by caching responses by `Idempotency-Key` header with TTL expiration.

---

## Native AI SDK & Hybrid RAG (`yav_server::ai`)

Integrate LLMs, streaming UI, and dense vector embeddings into your applications:
- **`Conversation`**: Multi-turn chat context tracking with user, assistant, system, and tool call roles.
- **`ToolRegistry`**: Register type-safe functions as JSON-Schema tools for LLM tool calling.
- **Streaming UI**: Stream partial HTML fragments directly into the DOM via SSE (`stream_ui_fragment`) or text tokens (`stream_text_sse`).
- **Hybrid RAG**: Fast cosine similarity `VectorIndex` fused with BM25 keyword search using **Reciprocal Rank Fusion** (`reciprocal_rank_fusion`).

---

## Geospatial Indexing & Geofencing (`yav_server::geo`)

Calculate Haversine surface distances, test whether coordinates fall inside complex geographical boundaries with point-in-polygon ray-casting (`GeoPolygon`), and query nearest neighbors via `SpatialIndex`:
```rust
use yav_server::geo::{GeoPoint, GeoPolygon, SpatialIndex};

let paris = GeoPoint::new(48.8566, 2.3522);
let london = GeoPoint::new(51.5074, -0.1278);
let distance_km = paris.haversine_distance_km(&london);
assert!((distance_km - 343.0).abs() < 5.0);
```

---

## Built-in Markdown Engine (`yav_build::markdown`)

Yav includes a zero-dependency markdown parsing engine tailored for technical documentation and blogs:

```rust
use yav_build::markdown::parse_markdown;

let raw_md = r#"---
title: "Guide de Déploiement"
slug: "deployment-guide"
description: "Déployer Yav en production"
---
# Guide de Déploiement

Déployez votre application avec **Docker** ou **Fly.io**.
"#;

let parsed = parse_markdown(raw_md);
assert_eq!(parsed.title, "Guide de Déploiement");
assert_eq!(parsed.slug, "deployment-guide");
assert!(parsed.html.contains("<strong>Docker</strong>"));
```

---

## Optimistic UI Mutations & Automatic Rollbacks (`yav-core`)

Provide instant 0ms perceived latency for user interactions (likes, bookmarks, counters, toggles). If the server responds with an error (e.g., HTTP 4xx/5xx), the DOM mutation automatically rolls back to its previous state with a toast notification:

```html
<!-- Optimistic class toggle with automatic rollback -->
<button
    yav:post="/api/posts/42/like"
    yav:optimistic-toggle="text-rose-500,bg-rose-50"
    yav:rollback-toast="Échec de la mention j'aime. Veuillez réessayer."
    class="flex items-center gap-2 p-2 rounded-lg border text-slate-400"
>
    <span>❤️</span>
    <span yav:optimistic-inc="#like-count">J'aime</span>
</button>
<span id="like-count" class="font-bold">42</span>
```

---

## Reactive Live Queries & CDC Server-Push (`yav::server::live_query`)

Stream reactive database updates and Change Data Capture (CDC) events to thousands of connected clients over lightweight Server-Sent Events (SSE).

```rust
use yav::server::live_query::{LiveQueryManager, LiveEvent, global_live_queries, live_query_sse_handler};

// 1. Subscribe client to real-time table invalidation channel:
let app = YavApp::new()
    .get("/_yav/live", live_query_sse_handler)
    .post("/api/orders", |_req| async move {
        // Mutate order in DB...
        
        // 2. Broadcast CDC invalidation event across cluster:
        global_live_queries().publish(
            "orders",
            LiveEvent::InvalidateTable { table: "orders".to_string() }
        );
        
        json_response(StatusCode::CREATED, &serde_json::json!({ "status": "ok" }))
    });
```

---

## Cryptographic Proof-of-Work Shield & Honeypots (`yav::server::shield`)

Eliminate automated bot abuse, DDoS attacks, and credential stuffing on public endpoints without degrading human UX or using intrusive third-party CAPTCHAs (Cloudflare Turnstile, reCAPTCHA).

```rust
use yav::server::shield::{PowShield, Honeypot, pow_client_script};

let shield = PowShield::new("production-hmac-secret")
    .with_difficulty(3) // 3 leading hex zeros (~200ms CPU solve on client)
    .with_ttl(300);

// In RSX form:
rsx! {
    <form method="POST" action="/api/login" class="space-y-4">
        // 1. Invisible honeypot trap catching dumb bots:
        {Node::raw(Honeypot::render_input(None))}

        // 2. Client-side invisible Web Worker PoW solver:
        {Node::raw(pow_client_script("/api/pow/challenge"))}

        <input type="email" name="email" required />
        <input type="password" name="password" required />
        <button type="submit">Se connecter</button>
    </form>
}

// In backend POST handler:
if Honeypot::is_bot(&form_data.0, None) {
    return StatusCode::BAD_REQUEST.into_response(); // Silently reject bot
}
```

---

## Zero-Dependency GraphQL Schema & Playground (`yav::server::graphql`)

Expose a compiled, type-safe GraphQL API and a small self-contained, hand-written HTML playground (`graphiql_html`) with zero external runtime dependencies. It is **not** the upstream GraphiQL 3.x IDE and loads no CDN assets.

```rust
use yav::server::graphql::{GraphQlSchema, GraphQlObject, GraphQlType, graphql_handler, graphiql_html};

let query = GraphQlObject::new("Query")
    .field("uptime_seconds", GraphQlType::Int, |_args| {
        Ok(serde_json::json!(42_000))
    })
    .field("version", GraphQlType::String, |_args| {
        Ok(serde_json::json!(env!("CARGO_PKG_VERSION")))
    });

let schema = GraphQlSchema::new(query);

let app = YavApp::new()
    .route_raw("/graphql", Method::POST, graphql_handler(schema.clone()))
    .get("/graphql/explorer", |_req| async {
        html_response(StatusCode::OK, graphiql_html("/graphql"))
    });
```

---

## Geolocation, Multi-Currency & EU VAT Engine (`yav::server::geo_ip`)

Inspect edge proxy geolocation headers (`cf-ipcountry`, `x-country-code`, `x-forwarded-for`) and automatically calculate European Union VAT (Value Added Tax) rules with exact integer money arithmetic.

```rust
use yav::server::geo_ip::{extract_geo_context, VatRule};
use yav::server::billing::Money;

let geo = extract_geo_context(&headers);
// e.g. Country: "FR", Currency: "EUR", is_eu: true

// Calculate penny-accurate European VAT:
let net_price = Money::eur(100_00); // 100.00 EUR
let breakdown = VatRule::calculate_tax(net_price, &geo.country);
// Gross: 120.00 EUR (FR VAT 20%)
```

---

## Micro-Frontends & Module Federation Gateway (`yav::server::federation`)

Orchestrate complex distributed web architectures by stitching independent remote microservices into a unified parent page layout. `fetch_and_stitch_slots` fetches fragments in parallel and stitches them in declaration order, with automatic fallback HTML when an upstream fails.

```rust
use yav::server::federation::{FederationGateway, FederationSlot};

let gateway = FederationGateway::new()
    .route("/checkout", "https://checkout.internal.service")
    .route("/blog", "https://cms.internal.service")
    // Empty by default is an explicit deny: opt in to the upstream hosts.
    .with_allowed_upstream_hosts(["checkout.internal.service", "cms.internal.service"])
    .with_fallback("/checkout", "<div class=\"p-4 text-amber-500\">Système de paiement temporairement indisponible</div>");

let slots = [
    FederationSlot::new("cart", "/checkout/cart", 5),
    FederationSlot::new("posts", "/blog/latest", 5),
];
let shell = "<main><!-- FEDERATION_SLOT:cart --><!-- FEDERATION_SLOT:posts --></main>";
// Both fragments are fetched concurrently, then stitched in declaration order.
let html = gateway.fetch_and_stitch_slots(shell, &slots).await?;
```

---

## Pure Rust Vector Barcode 128 & QR Code Generator (`yav::server::barcode`)

Generate crisp, lightweight vector SVGs for ISO/IEC 18004 QR codes and Code 128 barcodes directly on the server without native C libraries or ImageMagick.

```rust
use yav::server::barcode::{Barcode128, QrCode};

// Code 128 logistics & shipping barcode:
let barcode_svg = Barcode128::svg("TRACK-9400-1000", 60, 2);

// ISO/IEC 18004 QR code for TOTP 2FA or payments:
let qr_svg = QrCode::encode("otpauth://totp/Acme:alice?secret=JBSWY3DPEHPK3PXP").svg(240);
```

---

## Automated Security Auditor & W3C CSP Reporter (`yav::server::security_audit`)

Run automated OWASP/Mozilla Observatory header compliance grading on every deployment and collect real-time client Content Security Policy (CSP) violation reports:

```rust
use yav::server::security_audit::{SecurityAuditor, SecurityGrade, CspReporter, csp_report_handler};

// 1. Audit HTTP headers against strict OWASP rules:
let report = SecurityAuditor::audit(&response_headers);
println!("Security Grade: {:?} (Score: {}/100)", report.grade, report.score);

// 2. Mount W3C CSP violation reporting endpoint:
let app = YavApp::new()
    .post("/_yav/csp-report", csp_report_handler);
```

---

## Circuit Breakers, Bulkheads & Adaptive Concurrency Limiter (`yav::server::resilience`)

Protect external APIs (Stripe, OpenAI, Resend) and downstream microservices from cascading failures:

```rust
use yav::server::resilience::{CircuitBreaker, CircuitBreakerConfig, BreakerState, AdaptiveConcurrencyLimiter};

// 1. Thread-safe Circuit Breaker with Half-Open probe recovery
let breaker = CircuitBreaker::with_config(
    "stripe-payments",
    CircuitBreakerConfig {
        failure_threshold: 5,
        recovery_timeout_secs: 30,
        success_threshold: 2,
    },
);

if breaker.allow_request() {
    match execute_stripe_charge().await {
        Ok(res) => {
            breaker.record_success();
            res
        }
        Err(err) => {
            breaker.record_failure();
            return Err(err);
        }
    }
} else {
    // Fail fast immediately without exhausting worker threads
    return Err("Payment gateway temporarily unavailable");
}

// 2. Adaptive Concurrency Limiting (Vegas TCP algorithm parity)
let limiter = AdaptiveConcurrencyLimiter::new(20, 100);
if let Some(guard) = limiter.try_acquire() {
    // Execute critical query under bounded capacity
}
```

---

## Transactional Outbox Pattern & Reliable Event Bus (`yav::server::outbox`)

Avoid Dual-Write inconsistencies between your database and distributed brokers (Kafka, RabbitMQ, Webhooks):

```rust
use yav::server::outbox::{TransactionalOutbox, OutboxMessage, OutboxStatus};

let outbox = TransactionalOutbox::new();

// Stage event atomically inside your database transaction:
let event_id = outbox.publish(
    "user.created",
    "users",
    "usr_42",
    serde_json::to_string(&user_payload).unwrap(),
);

// Background worker pulls uncommitted or pending messages with exponential retry backoff:
let pending_batch = outbox.poll_pending(50);
for msg in pending_batch {
    match dispatch_to_kafka(&msg).await {
        Ok(_) => outbox.mark_delivered(&msg.id),
        Err(e) => outbox.record_failure(&msg.id, e.to_string()),
    }
}
```

---

## Multi-Region Database Replica Pool & Router (`yav::server::replica_pool`)

Route write mutations to the primary master while distributing read traffic across low-latency geo-replicas with automatic "Read-Your-Own-Writes" session stickiness:

```rust
use yav::server::replica_pool::{DbReplicaPool, RouteTarget};

let pool = DbReplicaPool::new("postgres://master-db.internal", "us-east")
    .with_stickiness_window(5); // 5-second session stickiness after writes

pool.add_replica("eu-1", "eu-central", "postgres://eu-replica.internal", 100);
pool.add_replica("ap-1", "ap-southeast", "postgres://ap-replica.internal", 100);

// Auto-routing:
// - POST/PUT/DELETE -> RouteTarget::Primary
// - GET/HEAD with user header -> nearest low-latency replica
let target = pool.route("GET", Some("eu-central"), Some("session_cookie_token"));
```

---

## Pure-Rust WebP Output & Image Pipeline (`yav::server::image_pipeline`)

Sniff image formats, parse metadata, and resize buffers with zero external C/C++ dependencies. AVIF *input* is detected and parsed, but **AVIF encoding is not supported** — output is always a WebP container:

```rust
use yav::server::image_pipeline::{
    detect_image_format, parse_image_metadata, ImageFormat, RawRgbImage, WebPEncoder
};

let format = detect_image_format(uploaded_bytes);
assert_eq!(format, ImageFormat::WebP);

// Bilinear downsampling of raw pixel frames:
let image = RawRgbImage::from_rgb(1920, 1080, raw_pixels)?;
let thumbnail = image.resize_bilinear(320, 180);

// Pack into compliant WebP container:
let webp_bytes = WebPEncoder::build_lossy_container(320, 180, &thumbnail.data);
```

---

## Cryptographic Vector PDF Signer & Watermarking (`yav::server::pdf_signer`)

Stamp tamper-evident HMAC-SHA256 signatures and translucent diagonal watermarks into PDF documents:

```rust
use yav::server::pdf_signer::{PdfSigner, PdfWatermark, PdfSignature};

// 1. Digitally sign PDF in-memory:
let mut doc = PdfDocument::new("Certified Invoice #INV-2026-0042")
    .paragraph("All contractual deliverables completed.");

let sig = PdfSigner::sign_document(
    &mut doc,
    b"signing-secret-key-32-bytes",
    "Finance Department",
    "Approved for Payment",
    "2026-11-05T12:00:00Z",
);

// 2. Vector watermarking ("CONFIDENTIAL", "COPIE CONFORME", "PAID"):
let wm = PdfWatermark::new("CONFIDENTIAL")
    .opacity(0.15)
    .rotation(45.0)
    .color(0.8, 0.2, 0.2);

let watermarked_pdf = PdfSigner::apply_watermark_to_pdf_stream(&raw_pdf, &wm);
```

---

## Local-First & Offline Sync Protocol (`yav::core::local_first`)

Seamless offline mutation recording via IndexedDB with automatic background synchronization and optimistic UI updates:

```rust
use yav::core::local_first::{
    OfflineMutation, MutationType, SyncBatch, SyncResponse, LocalStoreConfig, YAV_LOCAL_FIRST_SCRIPT
};

// Queue mutation locally on client disconnect:
let mutation = OfflineMutation::new(
    "mut_123",
    "todos",
    MutationType::Insert,
    "{\"title\":\"Prepare deployment\"}",
    1730000000,
    1,
);

// Server resolves incoming batch and detects concurrent version conflicts:
let mut batch = SyncBatch::new("client_session_99");
batch.add(mutation);

let response = SyncResponse::ok(vec!["mut_123"]);
```

---

## Multi-Tier Distributed Cache & XFetch (`yav::server::distributed_cache`)

Prevent cache stampedes and dogpiling under massive concurrent load using the optimal probabilistic early expiration algorithm (XFetch: `Δ * β * ln(rand())`):

```rust
use std::time::Duration;
use yav::server::distributed_cache::{DistributedCache, MemoryL1Store};

// 1. Initialize distributed cache with L1 capacity and default TTL:
let cache = DistributedCache::new(50_000, Duration::from_secs(3600))
    .with_beta(1.2); // Aggressive pre-recomputation under heavy spikes

// 2. Safely read or compute with auto-measured execution time:
let user_profile = cache.get_or_compute("user:profile:10042", || {
    // Heavy DB query or slow external REST API call
    fetch_user_from_postgres(10042)
});
```

---

## Distributed Lock & Leader Election Protocol (`yav::server::distributed_lock`)

Coordinate background cron jobs, database migrations, and singleton workers across multi-node clusters:

```rust
use std::time::Duration;
use yav::server::distributed_lock::{DistributedLock, LockCoordinator, LeaderElector};

let coordinator = LockCoordinator::new();

// 1. Exclusive mutex lock with automatic TTL expiration:
if let Some(lock) = coordinator.try_acquire("nightly_invoice_batch", "worker_node_a", Duration::from_secs(60)) {
    // Safely execute singleton task
    coordinator.release(&lock);
}

// 2. High-availability Leader Election for cluster workers:
let elector = LeaderElector::new(coordinator.clone(), "cluster_scheduler", "node_pod_42");
if elector.campaign(Duration::from_secs(30)) {
    // This node is now the designated cluster leader
    assert!(elector.is_leader());
}
```

---

## Relationship-Based Access Control (ReBAC / Zanzibar) (`yav::server::rebac`)

Model fine-grained hierarchical permissions, direct assignments, and transitive role implications (inspired by Google Zanzibar):

```rust
use yav::server::rebac::{RebacGraph, RelationshipTuple};

let rebac = RebacGraph::new();

// 1. Assign relationships:
rebac.write_tuple("user:alice", "editor", "folder:finance");
rebac.write_tuple("folder:finance", "parent", "doc:q4_budget");

// 2. Verify direct, inherited, or implied permissions:
assert!(rebac.check("user:alice", "viewer", "doc:q4_budget"));
```

---

## Prometheus Metrics & Grafana-Ready Dashboard (`yav::server::metrics_dashboard`)

Mount an out-of-the-box, zero-dependency HTML telemetry dashboard with live SVG gauges and latency percentiles:

```rust
use yav::server::metrics_dashboard::{metrics_dashboard_handler, render_metrics_dashboard_html};

let app = YavApp::new()
    .get("/metrics/dashboard", metrics_dashboard_handler);
```

---

## Phonetic & Typo-Tolerant Fuzzy Search (`yav::build::fuzzy_search`)

Provide instant typo tolerance and phonetic soundex matching for documentation, products, and user search:

```rust
use yav::build::fuzzy_search::{
    damerau_levenshtein, soundex, suggest_did_you_mean, FuzzySearchIndex
};

// 1. Phonetic matching (handles accents, misspellings):
assert_eq!(soundex("Jackson"), soundex("Jaxon"));

// 2. Typo-tolerant multi-document search:
let mut index = FuzzySearchIndex::new();
index.add("doc_cache", "High performance distributed cache and invalidation");
let hits = index.search("distributd", 2, 5);

// 3. Intelligent "Did you mean?" suggestions:
let suggestion = suggest_did_you_mean("posgres", &["postgres", "redis", "mysql"]);
assert_eq!(suggestion, Some("postgres"));
```

---

## Streaming TurboStream Multi-Zone DOM Mutations (`yav::server::turbo_stream`)

Broadcast targeted multi-element HTML fragment mutations over Server-Sent Events (SSE) or HTTP responses, allowing zero-JS instant UI updates across distinct DOM target containers:

```rust
use yav::server::turbo_stream::{TurboStream, TurboAction, TurboMessage};

// 1. Build a multi-action TurboStream batch:
let stream = TurboStream::new()
    .replace("chat-messages", "<div id='msg-42'>Message reçu !</div>")
    .append("notifications", "<div class='toast'>Nouveau message</div>")
    .remove("typing-indicator");

// 2. Format as TurboStream HTML fragment or SSE events:
let html_response = stream.to_html();
let sse_event = stream.to_sse_event();
```

---

## Event Sourcing & CQRS Framework (`yav::server::event_sourcing`)

Full-featured aggregate event sourcing with strict optimistic concurrency control, immutable audit history, and pluggable read-model projections:

```rust
use yav::server::event_sourcing::{EventStore, DomainEvent, Projection};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct OrderCreated {
    order_id: String,
    total_cents: u64,
}

impl DomainEvent for OrderCreated {
    fn event_type(&self) -> &'static str { "OrderCreated" }
}

let store = EventStore::new();

// 1. Append immutable domain events with optimistic version checks:
let version = store.append_event(
    "order-101",
    OrderCreated { order_id: "order-101".into(), total_cents: 4990 },
    0, // expected aggregate version
).expect("Failed to append event");

// 2. Read event stream to reconstitute state:
let events = store.get_events("order-101").unwrap();
assert_eq!(events.len(), 1);
```

---

## Resumable Chunked Uploads (Tus Protocol v1.0.0) (`yav::server::resumable_upload`)

Production-ready implementation of the Tus.io open protocol (v1.0.0) supporting network interruption recovery, partial binary chunk concatenation, and checksum validation:

```rust
use yav::server::resumable_upload::{ChunkedUploader, TusError};

let uploader = ChunkedUploader::new(100 * 1024 * 1024); // 100 MB max

// 1. Create upload session (Tus POST):
let session_id = uploader.create_session(5_000_000, Some("video.mp4".into()), None).unwrap();

// 2. Stream chunk (Tus PATCH with offset check):
let first_chunk = vec![0u8; 1_000_000];
let new_offset = uploader.append_chunk(&session_id, 0, &first_chunk).unwrap();
assert_eq!(new_offset, 1_000_000);

// 3. Inspect upload status (Tus HEAD):
let session = uploader.get_session(&session_id).unwrap();
assert_eq!(session.offset, 1_000_000);
```

---

## WebPush Protocol & VAPID Authorization (`yav::server::web_push`)

Standard RFC 8291 / RFC 8292 Web Push notifications with VAPID JWT token generation and endpoint payload dispatching:

```rust
use yav::server::web_push::{PushSubscription, WebPushNotification, VapidSigner};

let subscription = PushSubscription {
    endpoint: "https://fcm.googleapis.com/fcm/send/sub_123".into(),
    keys: yav::server::web_push::PushSubscriptionKeys {
        p256dh: "BM...".into(),
        auth: "kT...".into(),
    },
};

let notification = WebPushNotification::new("Nouvelle commande !", "La commande #42 a été validée.")
    .icon("/assets/logo.png")
    .badge("/assets/badge.png")
    .url("/dashboard/orders/42")
    .ttl(3600);

// Generate VAPID Authorization header:
let signer = VapidSigner::new("mailto:admin@example.com", "private_pem_or_key");
let auth_header = signer.generate_auth_header(&subscription.endpoint);
```

---

## Mock API Server & Realistic Data Faker (`yav::build::mock_server`)

Zero-dependency local HTTP mock server with route pattern matching, status/header simulation, latency injection, and deterministic fake data generator:

```rust
use yav::build::mock_server::{MockServer, MockEndpoint, MockFaker};

let mut server = MockServer::new();

// 1. Deterministic realistic fake data generator:
let mut faker = MockFaker::new(42);
let user_email = faker.email();
let price = faker.price(10, 200);

// 2. Define mock endpoint responses:
server.add_endpoint(
    MockEndpoint::get("/api/users")
        .response(serde_json::json!([
            {"id": 1, "email": user_email, "price": price}
        ]))
        .delay_ms(15)
);

let response = server.handle_request("GET", "/api/users", None).unwrap();
assert_eq!(response.status, 200);
```

---

## Static i18n Key Extractor & Translation Linter (`yav::build::i18n_lint`)

Static analysis tool scanning Rust templates and RSX trees for `t!("key")` invocations, validating catalog completeness and detecting orphan translation keys:

```rust
use yav::build::i18n_lint::{scan_source_code, lint_catalog};
use std::collections::HashMap;

let source = r#"
    let msg = t!("auth.login_success");
    let err = t!("errors.not_found");
"#;

// 1. Extract used translation keys from code:
let occurrences = scan_source_code("src/views/login.rs", source);

// 2. Lint locale catalog:
let mut catalog = HashMap::new();
catalog.insert("auth.login_success".to_string(), "Connexion réussie".to_string());

let report = lint_catalog(&occurrences, &catalog);
assert!(report.missing_keys.contains(&"errors.not_found".to_string()));
```

---

## Deployment Targets

Yav provides official scaffolders for popular deployment platforms via `yav deploy`:

```bash
# Deploy to Fly.io (generates fly.toml and multi-stage Dockerfile)
yav deploy fly

# Deploy to Shuttle.rs (generates Shuttle.toml)
yav deploy shuttle

# Deploy to Railway (generates railway.json)
yav deploy railway

# Deploy to Linux Systemd (generates /etc/systemd/system unit file)
yav deploy systemd

# Deploy to Render.com (generates render.yaml blueprint)
yav deploy render

# Deploy to Cloudflare Containers (generates wrangler.toml)
yav deploy cloudflare

# Deploy with Caddy Reverse Proxy & Auto-HTTPS (generates Caddyfile)
yav deploy caddy
```

### Docker Multi-Stage Container

Generate an optimized production Docker deployment:

```bash
yav docker
```

This generates a lean, 2-stage Dockerfile using Alpine or Debian Slim:
- **Build stage**: Compiles Rust binary and assets.
- **Runtime stage**: Minimal non-root container image (~25MB) running the native binary.

Run with Docker:
```bash
docker build -t my-yav-app .
docker run -p 3000:3000 my-yav-app
```

## Production Server Tuning

In production, configure the following environment variables:
- `PORT`: HTTP port to bind (e.g. `8080`).
- `HOST`: Bind address (e.g. `0.0.0.0` inside containers).
- `SESSION_SECRET`: 32+ character random secret for HMAC-SHA256 session signature verification.
- `RUST_LOG`: Structured logging filter (e.g. `yav=info,tower_http=info`).
