# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-11-17

Audit remediation release. Full findings and severities:
[`AUDIT-2026-11-13.md`](AUDIT-2026-11-13.md).

### Security

- **WASM Island CSP Nonce**: `WasmIsland` now supports CSP nonces via
  `.with_nonce(...)`, generating `<script nonce="...">` tags to comply with
  strict Content-Security-Policy rules without requiring `unsafe-inline`.
- **Federation Proxy Hardening**: `FederationGateway::proxy_request` client
  builder now configures `.no_proxy()` to prevent local proxy environments from
  intercepting internal microservice calls. `fetch_and_stitch_slot` detects
  upstream HTTP error codes (`status >= 400`) and properly triggers the slot's
  fallback HTML.
- **PPR Memory Bounds**: Added hard capacity limits and automatic eviction to
  `SHELL_CACHE` (1024 shells) and `ComponentCache` (`with_capacity(1024)`),
  preventing unbounded memory growth under high concurrency.
- **PPR XSS**: PPR swap scripts now serialise slot HTML through
  `streaming::escape_js_string_literal` and `render_complete` injects the
  request CSP nonce, matching the streaming path. Regression tests cover
  `</script>` breakout in both paths.
- **Web Push SSRF**: `send_notification` now rejects unsafe endpoints before
  signing (https-only, no userinfo/fragment/control chars, no private,
  loopback, link-local or multicast hosts, DNS results checked), disables
  redirects and applies a 10 s timeout. VAPID claims are built with
  `serde_json::json!` instead of `format!`.
- **RFC 8291**: two-stage HKDF derivation now binds the 16-byte salt, uses the
  correct empty context for CEK/nonce expansion and the RFC 8188 `0x02`
  last-record delimiter. Previously the salt was written to the header but
  never used, making the payload undecryptable by real push services.
- **CSP**: removed the scheme-wide `https:` script source and hardened the
  nonce-less fallback to `script-src 'self'` only.
- **WebAuthn**: `finish_authentication` (no signature verification) is no
  longer public; only `finish_authentication_with_signature` is exposed.
- **Federation**: `forward_auth` defaults to `false`, upstream hosts require an
  explicit allowlist, and stitched fragments are sanitized by default.
- **Escaping**: `srcdoc` is double-escaped, `srcset` is validated URL-by-URL,
  `style` is filtered, and JSON-LD data blocks encode `<`, `>`, `&` and JS
  line separators. `org_chart` now uses `escape_url` for `img src`.
- **Flash cookies**: legacy payloads without an expiry are rejected; flash
  secret resolution mirrors `YavApp` (one ephemeral CSPRNG secret, no panic).
- **CSRF**: production no longer silently falls back to the hardcoded
  development secret when `SESSION_SECRET` is missing.
- **OpenAPI docs**: the Scalar/Swagger pages are now genuinely self-contained
  static HTML (no CDN bundles, no JavaScript).
- **GraphQL playground**: `graphiql_html(endpoint)` no longer interpolates the
  endpoint directly into `fetch('…')`. It is now serialised through
  `streaming::escape_js_string_literal` (JSON string literal with `<`, `>`,
  `&`, `/` and U+2028/U+2029 escaped), preventing `</script>` breakout and JS
  injection. Regression tests cover both a normal and a hostile endpoint; the
  module docs now say it is a hand-written page, not GraphiQL 3.x.

### Changed

- **Concurrent federation slots**: `FederationGateway::fetch_and_stitch_slots`
  fetches all upstream fragments in parallel with `join_all` and stitches them
  in declaration order. `fetch_fragment` was extracted from
  `fetch_and_stitch_slot` so both paths share identical fallback and
  sanitization semantics. Verified by a barrier-based test against a local TCP
  server: a sequential implementation would time out.
- **Opt-in automatic HTTP tracing**: `YavApp::otel(true)` layers
  `otel_tracing_middleware` outermost, producing one `SERVER` span per request
  in `global_tracer()` and echoing W3C `traceparent`. Off by default, so no
  existing application gains headers. Covered by real-TCP integration tests
  for both the enabled and default states. DB, AI and outbound calls still
  require explicit spans.
- **Dev server shutdown handle**: the cloneable `ReloadShutdown` handle
  (`ReloadServer::shutdown_handle()`) stops the SSE listener via a `watch`
  channel; `yav dev` invokes it on Ctrl+C and the port is released.
  Covered by a test that rebinds the port after shutdown.
- **`yav new --check`**: verifies the generated project with `cargo check`
  before reporting success and exits non-zero on failure. Without the flag the
  success message now says "(not compiled)" instead of implying the project
  builds. Opt-in because scaffolding resolves `yav` from the registry.
- **PPR documentation corrected**: the `ppr.rs` module doc still claimed slots
  resolve sequentially; it now describes the concurrent `join_all` /
  `FuturesUnordered` behaviour that has shipped since 0.1.6. Two barrier-based
  tests prove concurrency on both the composed and streaming paths.
- **CLI `yav test` Passthrough Arguments**: `yav test` now accepts trailing arguments and filters (`#[arg(trailing_var_arg = true, allow_hyphen_values = true)]`), forwarding them verbatim to `cargo test` (e.g. `yav test my_filter`, `yav test -p yav-core`, `yav test -- --nocapture`). Previously, `Commands::Test` took zero arguments and rejected any options or filters with `unexpected argument`.
- **PPR Concurrent & Out-of-Order Slot Resolution**: `render_complete` now
  resolves dynamic slots concurrently in parallel via
  `futures_util::future::join_all`. `into_streaming_response` resolves and
  streams dynamic slots concurrently and out-of-order via `FuturesUnordered` +
  bounded `mpsc::channel`, swapping dynamic slots on the client as each one
  completes without blocking on slower sibling slots.
- **AI Real-time Token Streaming & Bounded RAG**: `stream_text_channel` and
  `stream_text_stream` in `yav-server::ai` now stream tokens in real-time
  directly from asynchronous LLM channels or streams (OpenAI, Anthropic, Ollama)
  without pre-buffering all chunks into memory. `VectorIndex` now supports
  `with_capacity` bounding and uses $O(N + K \log K)$ partial selection search.
- **Truthfulness**: `EmbeddedDatabaseAdapter` / `LocalLlmAdapter` are now
  `MockDatabaseAdapter` / `MockLlmAdapter` (deprecated aliases kept); they
  return explicit errors instead of fake `Ok(1)` / fabricated completions.
  The metrics dashboard reads real counters from `global_tracer()`; values
  that are not instrumented are `0`, never invented. `MockServer::delay_ms`
  is actually applied. VAT now covers all 27 EU states with exact integer
  arithmetic. `ImagePipeline` and `AppState` now exist as documented.
- **Build**: `reqwest` uses `default-features = false` (no OpenSSL/native-tls
  beside rustls); unused `hyper`/`tracing-subscriber` deps, the duplicate
  `serde` feature and the no-op `full` feature were removed;
  `futures-util` is centralised; `panic = "abort"` removed from release.
- **CI**: `--locked` / `--all-targets` everywhere, all-features tests, doc
  tests, `cargo doc --lib` (Linux runner only, see Fixed), a check that icon
  licenses are packaged, an MSRV metadata check, `cargo-audit --locked`,
  plus `rustfmt.toml`, `clippy.toml`, `deny.toml` and Dependabot config.
- **Documentation**: README rewritten as a plain, developer-facing document
  with no emoji and no marketing superlatives, but with a real pitch and five
  Mermaid diagrams (maturity tiers, five-step mental model, request lifecycle,
  RSX compile/render path, crate map). Added a short "why this exists"
  section, a side-by-side comparison with bare Axum / typical JS full-stack,
  an explicit target audience and non-audience, an annotated Quick Start, and
  a concrete RSX-to-escaped-HTML example. It states feature maturity
  explicitly and keeps the crates.io metadata caveat; all Rust snippets were
  compile-checked against `examples/dashboard`. `SECURITY.md`,
  `CONTRIBUTING.md`, `CHANGELOG.md` and `docs/security.md` no longer claim
  CSRF is opt-in or that `SESSION_SECRET` aborts startup; API examples use
  real signatures; false GraphiQL/AVIF/multi-region claims corrected.
  `docs/limitations.md` now names the `Node::Raw` variant instead of the
  non-existent `Node::raw` helper. Restored the sense of scale the earlier
  draft lost: the README now opens with a real screenshot of
  `examples/dashboard` and, under "What else is in the box", states the shipped
  footprint — 82 UI components across 56 modules, 8 themes, a ~1,900-icon
  classic set (~13,600 behind `full-icons`), and 57 server modules — followed
  by a grouped catalogue that names all 82 `yav-ui` components by category. All
  counts were verified against the source; no emoji or marketing superlatives
  were reintroduced.

### Fixed

- Both Clippy `-D warnings` failures (`render.rs` doc-comment blank line,
  `overlay.rs` redundant reference) and three new lints introduced by the
  security work.
- `docs/cli.md` duplicate section number, `docs/production.md` obsolete
  "parallel slot hydration", `docs/architecture.md` crate attribution,
  VS Code extension URL/version, and hardcoded Windows paths in asset scripts.
- **Doctests**: the 10 previously ignored examples in `yav` are now
  `rust,no_run` and compiled by CI (15 compiling doctests in that crate). Two
  that did not compile were repaired instead of hidden: `TiltPlate` used a
  non-existent `glare` prop (real field: `spotlight`), and the
  `export_static` snippet was an uncontextualised fragment (now a full
  `#[tokio::main]` example with real handlers). The 11 `yav-macros` examples
  remain explicitly `ignore`d with the structural reason documented in the
  crate docs and `docs/limitations.md`: a proc-macro crate cannot resolve the
  `yav` facade or the consuming project's template/route files from its own
  doctest environment. Real compiled macro coverage lives in `yav`'s docs.
- **rustdoc**: fixed the unresolved `CsrfConfig::disabled` link introduced by
  the CSRF documentation update. `cargo doc` now runs with `--lib` and only on
  the Linux CI runner: the `yav` bin/lib name collision plus `Card`/`card`
  style UI pairs produce case-only filename collisions on Windows/macOS.
  The limitation is documented in `docs/limitations.md`.
- `test-app` release profile no longer uses `panic = "abort"` (aligned with
  the workspace profile), `assets/requirements.txt` now exists and lists the
  Python packages actually imported by `process_assets.py`, and
  `docs/limitations.md` no longer describes the metrics dashboard as
  hardcoded or the doctests as ignored.
- The `examples/dashboard` test now reflects CSRF being enabled by default:
  an unprotected `POST /settings` is asserted to return `403`, while the same
  handler returns `200` when `CsrfConfig::disabled()` is applied explicitly.
  `examples/dashboard/Cargo.lock` and `test-app/Cargo.lock` were regenerated
  offline so both demo projects build with `--locked`.
- Stale claims removed from `docs/architecture.md` and `docs/production.md`
  (GraphiQL 3.x, AVIF output, parallel federation slots) and from the 0.1.5
  changelog entry (CSRF described as "opt-in").

## [0.1.5] - 2026-11-08

Security-hardening, "truthfulness" and feature-completeness release. The
guiding rule for this version was rule 12 of the specification: **a command or
feature must never report success when it did not actually do the work.**

### Added

- Opt-in roadmap integrations in `yav_server::roadmap`: real SQLite/PostgreSQL
  adapters and executable migrations (`sqlite`, `postgres`), Moka tagged cache
  (`tagged-cache`), durable SQL-backed job queue (`durable-jobs`), SMTP transport
  (`smtp`), object-storage adapter (`object-storage`), OpenAI-compatible HTTP
  LLM client (`http-llm`), Garde validation (`validation`), JSON Schema/TypeScript
  helpers (`schema-codegen`), OAuth2 token exchange with PKCE (`oauth`),
  Argon2id hashing (`argon2`), OTel SDK exporter (`otel-sdk`), plus build-side
  accessibility linting, advanced Markdown rendering and Fluent i18n.
- Every shipped integration is now reachable from the `yav` facade: the
  `sqlite`, `postgres`, `durable-jobs`, `tagged-cache`, `validation`,
  `schema-codegen`, `smtp`, `argon2`, `oauth`, `otel-sdk`, `object-storage`,
  `http-llm`, `a11y`, `markdown`, `i18n-advanced` and `icon-minify` features
  are forwarded to their owning crates, with `all-integrations` as a CI bundle.
  `yav::server::roadmap` is re-exported. Unused optional dependencies
  (`scraper`/`syntect`/`pulldown-cmark` in `yav-server`, `cargo_metadata` in
  `yav-build`, `openidconnect`, `tracing-opentelemetry`) were removed so no
  feature links a crate it does not use.
- [`docs/opt-in-integrations.md`](docs/opt-in-integrations.md): feature names,
  real entry points, required configuration and the exact boundaries of each
  opt-in integration.
- First-party contract tests: HTTP snapshots with `axum-test`/`insta`, CLI
  process tests with `assert_cmd`, property-based escaping test with `proptest`,
  and a `trybuild` pass fixture.
- [`docs/competitive-roadmap.md`](docs/competitive-roadmap.md): code-based
  inventory of current capabilities, prioritized feature gaps, concrete Rust
  dependency candidates with checked versions, and opt-in Cargo feature layout.
- Real ES256 / P-256 WebAuthn assertion verification, including a COSE_Key
  CBOR decoder (including negative integer labels) in `yav-server::passkeys`.
- Real ES256 ECDSA VAPID signing and **RFC 8291 `aes128gcm` payload encryption**
  in `yav-server::web_push`, plus actual HTTP delivery of push messages via
  `reqwest` (respecting `410 Gone` / `404` subscription expiry).
- Real HTTP reverse proxying in `yav-server::federation`
  (`proxy_request`, `fetch_and_stitch_slot`) with per-request timeouts and
  fallback HTML.
- Real OTLP/HTTP JSON export to a collector in `yav-server::otel`
  (`export_otlp_http`), in addition to the existing OTLP JSON payload builder
  and Prometheus exposition.
- `Web Push`, federation and OTLP documentation now states exactly what is
  implemented; GraphQL and AI/RAG modules are explicitly labelled as
  lightweight / experimental in their module docs.
- Icon licensing: `crates/yav-icons/LICENSE-LUCIDE` (ISC),
  `LICENSE-MYNAUI` (MIT), `LICENSE-PHOSPHOR` (MIT) and `NOTICE.md` are now
  shipped with the published crate.
- `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md` and
  [`docs/limitations.md`](docs/limitations.md).
- `.env.example` documenting the environment variables used by the framework,
  and `.gitignore` entries for large local artifacts (`promo-video/`,
  `assets/logos/`, raw upstream icon downloads).
- **JWT claim validation**: `JwtValidation` plus `JwtManager::verify_with_config`
  and `verify_at` now enforce `nbf`, `iss`, `aud`, optional future-`iat`
  rejection and configurable leeway (`JwtClaims` gained `nbf`/`iss`/`aud`).
- **CSRF middleware**: enabled-by-default `CsrfConfig` / `csrf_middleware` /
  `YavApp::csrf_protection` validate a session-bound token on unsafe methods,
  from the `x-csrf-token` header or the `_csrf` URL-encoded form field, with
  constant-time comparison, safe-method pass-through and path exemptions.
- `MarkdownEditor::preview_html` now sanitizes caller HTML with a tag/attribute/
  URL-scheme allowlist; `preview_html_trusted()` is the explicit raw escape hatch.
- Negative security tests for JWT claims, CSRF rejection, static-export path
  traversal (percent-encoded, backslash, drive-letter, reserved device names)
  and Markdown preview injection.
- **Real CLI integration tests** in `crates/yav-cli/tests/cli_integration.rs`:
  they spawn the built `yav` binary and assert exit codes and generated files
  (`--version`, help, unknown subcommand, `doctor` failure, `db seed` without
  `DATABASE_URL`, `--ts --openapi` codegen, `icon info`, route traversal).
  Uses only `std::process::Command` so the suite works offline.
- **Macro expansion-level negative tests** in `yav-macros` (`parser`,
  `component`, `form_derive`): mismatched/unclosed RSX tags, `self` receivers,
  destructured props, tuple structs and enums are asserted to produce
  `compile_error!`-equivalent errors, without a `trybuild` dependency.
- **Real end-to-end HTTP integration tests** in
  `crates/yav-server/tests/http_integration.rs`: a live Axum server on an
  ephemeral loopback port plus `reqwest` requests, verifying security headers
  and resolved CSP nonces, ETag / `If-None-Match` → 304, CSRF rejection of
  missing/wrong tokens, CSRF acceptance with the form body preserved, and the
  built-in health endpoints.

### Changed

- **Icon feature gates**: `yav` now compiles only the Classic family by
  default (~1,900 variants) instead of all ~13,600. Added `full-icons` plus
  the existing `minimal-icons` / `system-icons` switches; the dashboard and
  `test-app` examples opt into `full-icons` so their three-family demos keep
  working. `ICON_COUNT` now reflects the compiled feature set.
- `yav-cli` explicitly enables all three `yav-icons` families, so `yav icon
  search|list|info` works when the CLI is built/published on its own rather
  than silently shipping an empty catalog.
- **CLI commands now fail honestly**: `yav export` propagates SSR failures
  instead of emitting a placeholder page; `yav db` errors when `DATABASE_URL`
  or the DB client is missing and `yav db seed` actually executes the seed
  SQL; `yav doctor` exits non-zero when a critical tool is missing;
  `yav update` reports network/checksum failures instead of printing
  "up to date".
- `yav generate` respects `--ts`, can emit TypeScript and OpenAPI in one run,
  and generates **OpenAPI 3.1** documents. `yav codegen --ts --openapi
  --output X` now writes X (TypeScript) plus `X.openapi.json` instead of
  letting OpenAPI silently overwrite the TypeScript file.
- Hardcoded `v0.1.4` / `0.1.4` strings removed from crate doc headers, the
  generated starter page (now injected from `CARGO_PKG_VERSION`), OTLP scope
  version, and documentation examples.
- **MSRV corrected to Rust 1.88** (was a fictional `1.75`) and enforced by a
  new CI job. Verified with `cargo +1.88.0 check --workspace --locked`;
  `Cargo.lock` v4 and the locked `image` / `icu` / `encoding_rs` releases make
  1.75, 1.78 and 1.85 impossible without pinning older dependencies.
  `docs/getting-started.md` and `CONTRIBUTING.md` now state the real floor.
- `yav run` shell-escapes user arguments (prevents command injection).
- `yav dev` uses checked port arithmetic, watches `Cargo.toml`, `Cargo.lock`
  and Tailwind config, and propagates Tailwind download failures.
- Tailwind binary downloads now verify a SHA-256 checksum; a missing
  checksum is an error, not a warning, and versions are strictly validated.
- Generated deployment units (systemd / Docker) no longer contain a
  hardcoded `SESSION_SECRET`.
- `yav-core::wasm::with_params` validates its parameter as JSON and escapes
  `</script` sequences before embedding it in inline JS.
- Production error responses no longer leak internal messages or disk paths.
- Documentation: all 22 `file:///` links replaced with relative links,
  version references unified on 0.1.5, incorrect "Blake3 ETag",
  "QR Version 2 / 25x25" and "parallel federation slots" claims corrected.

### Fixed

- CBOR negative-integer decoding (`-1 - arg`, not `arg - 1`) for COSE keys.
- QR code generation is now backed by the `qrcode` crate (full ISO/IEC 18004
  with error correction) and image optimisation by the `image` crate.
- Code playground no longer fabricates a "simulated successful execution"
  result when no run endpoint is configured.
- Session cookies, CSP nonces, open redirects and header injection handling
  hardened across the server middleware stack.
- `export_static` no longer "sanitizes" paths with `replace("..", "")`; it
  decodes percent-escapes, normalises Windows separators and **rejects**
  unsafe route paths outright.
- PBKDF2 now reuses a single HMAC instance (`finalize_reset()`) instead of
  rebuilding the key schedule on every iteration.

### Known limitations

See [`docs/limitations.md`](docs/limitations.md). The most significant ones:
no per-icon tree-shaking (all enabled families are linked), the GraphQL
executor is a lightweight parser without fragments/directives, the AI/RAG
module has no bundled LLM client, and 21 doctests were marked `ignore`
(10 of them are compiled as `rust,no_run` since 0.1.6; the 11 `yav-macros`
sketches remain illustrative — see `docs/limitations.md`).

## [0.1.4] - Previous

- 8 built-in themes, zero-JS interaction attributes, view transitions,
  streaming SSR and the initial `yav-ui` component set.

## [0.1.0] - Initial

- First public workspace: `yav`, `yav-core`, `yav-macros`, `yav-router`,
  `yav-server`, `yav-ui`, `yav-icons`, `yav-cli`, `yav-build`.
