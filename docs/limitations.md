# Known limitations

This document lists what Yav **does not** do yet. It is deliberately blunt:
the README describes capabilities, this file describes boundaries. If a
feature is not listed here and you find it is only partially implemented,
please open an issue — and see [`CONTRIBUTING.md`](../CONTRIBUTING.md) for the
rule that forbids reporting fake success.

Current workspace version: **0.1.7**.

## 1. Modules that are libraries, not mounted services

`YavApp` mounts only a small set of built-in routes by default:
`/health`, `/healthz`, `/readyz`, `/metrics`, `/_yav/image`, `/_yav/og`,
`/_yav/vitals`. All other `yav-server` modules (`graphql`, `web_push`,
`otel`, `federation`, `ai`, `rebac`, `security_audit`, `metrics_dashboard`,
…) are **library modules**: you must wire them into your own router and
supply your own persistence. There is no `/graphql`, `/mcp`,
`/metrics/dashboard` or `/_yav/csp-report` endpoint unless you register one.

`metrics_dashboard_handler` reads real values from
`yav_server::otel::global_tracer()` (span count, error count, uptime, total
duration). Values that are not instrumented — percentiles, cache hit ratio,
RSS memory — are shown as `0`, never fabricated. With `YavApp::otel(true)` it
reflects the automatic per-request HTTP spans; without that flag, and for any
span the application did not record explicitly, it shows zeros. Callers that
own their own counters can still use `render_metrics_dashboard_html` directly.

## 2. "Distributed" primitives are single-process

By default the following modules still use in-memory `RwLock<HashMap>` / `Vec`
storage and offer **no cross-process or cross-node guarantee**, despite their
names: `distributed_cache`, `distributed_lock`, `replica_pool`,
`event_sourcing`, `outbox`, `storage`, `email`, `jobs`, and the SQL string
builder in `db`. The default `ServiceAdapters` remain development-only.

Opt-in features now provide real implementations: `sqlite` and `postgres` add
`sqlx` adapters implementing `DatabaseAdapter`; `Migrator::apply_all_with`
executes migration SQL through a real adapter and journals the result;
`tagged-cache` adds a Moka-backed `CacheAdapter`; `durable-jobs` adds a
SQL-backed queue; `smtp` adds a real `lettre` transport; `object-storage`
wraps an `object_store` backend; and `http-llm` talks to an OpenAI-compatible
HTTP API. These features are off by default, so `yav new` still compiles
without external services. They are forwarded through the `yav` facade and
catalogued in [`docs/opt-in-integrations.md`](opt-in-integrations.md). Without
them, do not use the in-process modules for clustering, exactly-once delivery,
or durability.

## 3. GraphQL is a lightweight executor

`yav-server::graphql` implements simple top-level field selection, basic
introspection and mutation resolution with user-supplied resolvers. It is
**not** a spec-complete GraphQL engine: no fragments, no nested selection
sets, no variables, no directives, no input validation, no schema SDL
parser, and the embedded playground is a hand-written HTML page rather than
GraphiQL 3.x. For a real schema use `async-graphql` or `juniper` and mount
it yourself.

## 4. AI / RAG has no bundled model client

`yav-server::ai` provides `Conversation`, a `ToolRegistry` with JSON Schema
generation, real-time SSE streaming helpers (`stream_text_channel`,
`stream_text_stream`, and the legacy `stream_text_sse`),
`stream_ui_fragment`, an in-memory `VectorIndex` with bounded capacity
(`with_capacity`) and partial selection search ($O(N + K \log K)$), and
`reciprocal_rank_fusion` over pre-ranked lists. There is **no LLM client,
no embedding model, and no persistence**. Supply your own embeddings and
your own provider calls; the in-memory index is intended for development
and small datasets, not large corpora. The optional `http-llm` feature adds
`roadmap::llm::HttpLlmAdapter`, an OpenAI-compatible HTTP client that requires
`YAV_LLM_API_KEY` and fails loudly when it is absent. It does not bundle a
model, an embedding runtime, or vector persistence.

## 5. Observability: automatic HTTP spans only, everything else manual

`YavApp::otel(true)` installs `otel_tracing_middleware` as the outermost
layer. Every request then gets exactly one `SERVER` span in `global_tracer()`
(method, path, status, duration) and a W3C `traceparent` echoed on the
response; a `traceparent` supplied by the caller is continued as a child.
It is **off by default** so existing applications keep their header set.

Everything beyond that HTTP boundary is still manual: database calls, AI
helpers and outbound HTTP requests need spans you create explicitly, and
exporting remains an explicit `export_otlp_http` / `export_prometheus` call.
The optional `otel-sdk` feature adds `roadmap::otel` helpers that build a real
OTLP/HTTP span exporter, tracer provider and W3C propagator from the official
`opentelemetry` crates. It still does not install a tracing subscriber for you
and does not auto-instrument third-party crates. gRPC/OTLP binary transport is
not enabled.

## 6. Federation: parallel fetch, ordered stitching, no head merging

`FederationGateway::fetch_and_stitch_slots(&[FederationSlot])` fetches every
upstream fragment concurrently with `futures_util::future::join_all`, then
stitches them into the shell **in declaration order**. A slow slot therefore
no longer delays the others. `fetch_and_stitch_slot` remains the single-slot
entry point; both APIs share the same per-route fallback and sanitization
behaviour, and errors are returned in declaration order.

Stitching is still pure string replacement: `stitch_slot` does not parse or
merge `<head>` assets automatically (use `parse_upstream_document` and merge
them yourself). `proxy_request` performs real HTTP reverse proxying via
`reqwest` with a per-request timeout and `.no_proxy()`, and refuses any host
not explicitly allowlisted.

## 7. PPR state is process-local and streaming needs chunked HTTP

Dynamic slots are **no longer resolved serially**: `render_complete` uses
`futures_util::future::join_all`, and `into_streaming_response` streams results
concurrently out-of-order via `FuturesUnordered` and bounded channels. Both
paths are covered by barrier-based tests in `ppr.rs`.

What remains bounded: real-time slot streaming requires HTTP/1.1 chunked
transfer or HTTP/2, and the static shell cache (`SHELL_CACHE`) plus component
memoization (`ComponentCache`) are **in-memory, single-process** caches. They
enforce bounded capacities with automatic eviction to prevent memory
exhaustion under high traffic, but are not shared across nodes or persisted
across restarts.

## 8. Icon catalog: family-level gating, no per-icon tree-shaking

`yav-icons` can compile ~13,600 generated `IconData` literals (~10.6 MB of
Rust source) into static slices. `yav` now enables **only `classic-icons` by
default** (~1,900 variants); `minimal-icons`, `system-icons` and the
`full-icons` bundle are opt-in features. This avoids linking the whole catalog
into every application, but it is still a **family-level** gate: there is no
per-icon dead-code elimination, so every enabled family is linked in full.
Workaround: depend on `yav-icons` directly with
`default-features = false, features = ["classic"]`. The `icon-minify` feature
adds a deterministic allowlist hook (`YAV_ICON_ALLOWLIST`) exposed through
`yav_icons::minify`; it does **not** yet rewrite the generated catalog or
reduce the linked binary. Published build-time and binary-size measurements
are still unavailable. Note that `cargo test --workspace` builds `yav-icons` as a root
package with its own default features (all three families), so workspace
measurements do not reflect a downstream Classic-only build.

The raw upstream SVG/JSON trees are excluded from the published crate. The
required third-party license texts are shipped at the crate root
(`LICENSE-LUCIDE`, `LICENSE-MYNAUI`, `LICENSE-PHOSPHOR`, `NOTICE.md`).

## 9. Auth and session caveats

- The optional `argon2` feature provides `roadmap::security::Argon2Hasher`
  with standard PHC strings. It is not enabled by default; the built-in
  PBKDF2 hasher remains available for compatibility.
- The optional `oauth` feature provides `roadmap::oauth::OAuthTokenClient`:
  authorization URL with CSRF state and PKCE S256, real authorization-code
  exchange and bearer userinfo. It does not validate `id_token` signatures;
  use the `openidconnect` crate directly for full OIDC flows.
- `JwtManager::verify` (legacy) still checks `alg` and `exp` only. The
  config-aware `verify_with_config` / `verify_at` APIs additionally validate
  `nbf`, `iss`, `aud`, optional future-`iat` rejection and clock leeway, but
  **you must call them explicitly** — the default entry point is unchanged for
  backwards compatibility.
- CSRF enforcement now exists (`CsrfConfig`, `csrf_middleware`,
  `YavApp::csrf_protection`) and is **enabled by default** since v0.1.5.
  Disabling it requires `CsrfConfig::disabled()`. Every state-changing form
  must carry `_csrf` or every AJAX request must carry `x-csrf-token`;
  third-party webhook endpoints must be added to `exempt_paths`.
- Session revocation requires explicitly configuring a
  `SessionRevocationList`; it is not active by default.
- Passkeys/WebAuthn verify ES256 assertions and COSE_Key CBOR, but
  attestation-chain validation, token binding and full authenticator metadata
  are not implemented. For high-assurance deployments prefer `webauthn-rs`.
- `pdf_signer` produces an **HMAC-SHA256 integrity tag**, not a PAdES / X.509
  PDF signature with `/Sig` and `/ByteRange`. Its module docs say so.
- `pdf_viewer` embeds the document in an `<iframe src="…#page=N">`; it is not
  a pure-Rust vector PDF renderer.

## 10. Codegen and CLI caveats

- `yav generate --openapi` emits **OpenAPI 3.1** from the registered route
  table. Routes are still typed conservatively; it does not infer request
  bodies or per-status response schemas from handler signatures.
- `yav generate --ts` emits TypeScript interfaces for routes plus a fixed
  set of framework types. It does **not** parse arbitrary Rust `struct`
  definitions. `#[derive(ApiSchema)]` exposes `ts_interface()` in user code,
  but the CLI does not collect those automatically yet.
- The optional `schema-codegen` feature adds `roadmap::schema::json_schema`,
  `ts_type` and `ts_declarations` helpers backed by `schemars` and `ts-rs`.
  They must be called from application code; `yav codegen` does not yet
  discover and aggregate those types automatically.
- TypeScript and OpenAPI can be generated together with `--ts --openapi`.
  With an explicit `--output X`, the TypeScript file is written to `X` and the
  spec to a derived `X.openapi.json` sibling; without `--output` the defaults
  `types.d.ts` and `openapi.json` are used.
- `yav new --check` runs `cargo check` inside the generated project and fails
  with a non-zero exit status if it does not compile. Without `--check`, the
  project is created but explicitly reported as "(not compiled)". The flag is
  opt-in because a fresh scaffold resolves `yav` from the registry and may
  need network access.
- `yav export` fails loudly when SSR fails, but static export still implies
  that every route is prerenderable without per-request state.
- The dev server watches `Cargo.toml`, `Cargo.lock`, Tailwind config and Rust
  sources. The cloneable `ReloadShutdown` handle obtained via
  `ReloadServer::shutdown_handle()` now stops the SSE listener cooperatively;
  `yav dev` calls it on Ctrl+C, and the port is released. This stops the reload
  listener only — it does not manage the application process lifetime.
- `yav doctor` reports missing critical tools with a non-zero exit code, but
  it does not repair the environment.
- `yav update` has **no prebuilt release binaries**: it always runs
  `cargo install` into a temporary root, which is a full source build and can
  take several minutes. The replacement itself is atomic (`self_replace`), so
  it works on Windows, but the new binary only takes effect after restarting
  the shell. `--force` reinstalls the CLI; it never forces a dependency
  downgrade. Tailwind's standalone compiler is cached by pinned version and
  is not upgraded to newer Tailwind releases by `yav update`.
- The self-update fix is only effective in the binary that contains it. A CLI
  already installed from `0.1.5`/`0.1.6` on Windows cannot replace itself and
  needs one manual `cargo install yav-cli --force` (or a `--git` install from
  `main`) before `yav update` becomes self-sufficient.

## 11. Test and packaging gaps

- Of the 21 previously `ignore`d doctests, the **10 in `yav`** are now
  `rust,no_run`: they are compiled by `cargo test --workspace --doc` but not
  executed (they construct servers, bind ports or need runtime state). Two
  that did not compile were repaired rather than hidden: `TiltPlate` used a
  non-existent `glare` prop (real field: `spotlight`), and the
  `export_static` snippet was an uncontextualised fragment (now a full
  `#[tokio::main]` example with real handlers).
- The **11 in `yav-macros` remain explicitly `ignore`d**, with the reason
  documented in that crate's module docs: a proc-macro crate's doctests
  cannot resolve the umbrella `yav` facade without a circular dev-dependency,
  and its file/route macros read real `templates/*.rsx` / `src/routes/`
  files from the consuming project at compile time. Real compiled coverage of
  `rsx!`, `#[component]`, `#[server]` and the route macros exists in the
  `yav` crate docs (15 compiling doctests). Around 40 other doctests compile
  and run normally.
- `cargo doc --workspace --no-deps` is run with `--lib` and **only on Linux**
  in CI. `yav-cli` has a `yav` binary and `yav` has a `yav` library, which
  collide on Cargo's known `target/doc/yav/index.html` bug; additionally
  `yav-ui` exports `Card`/`card`-style pairs whose rustdoc output filenames
  differ only by case and collide on case-insensitive filesystems (Windows and
  default APFS on macOS). Building docs locally on those platforms can fail
  with "output filename collision" / `Access is denied`; use WSL/Linux or
  `cargo doc -p <crate> --no-deps --lib` for individual crates.
- A real CLI integration suite now exists at
  `crates/yav-cli/tests/cli_integration.rs` (spawns the built binary, asserts
  exit codes and generated files). It uses `std::process::Command` rather than
  `assert_cmd` to avoid adding an offline-unavailable dependency. The `yav test`
  CLI command accepts and forwards arbitrary filters, target flags, and test
  binary arguments directly to `cargo test` (`yav test <filter> -p <pkg> -- <flags>`).
- A real end-to-end HTTP suite now exists at
  `crates/yav-server/tests/http_integration.rs`: it binds a loopback TCP port,
  serves an Axum router and issues network requests with `reqwest`, asserting
  security headers/CSP nonce, conditional ETag requests, CSRF rejection and
  acceptance (including form-body preservation) and the built-in health
  endpoints. Integration tests must live inside a crate — this workspace has a
  virtual root manifest, so the empty root `tests/` directory is never
  compiled.
- First-party contract tests now also cover SSR responses and context-aware
  escaping (`axum-test`, `insta`, `proptest`) and CLI process behaviour
  (`assert_cmd`). `trybuild` currently provides a **pass** fixture that is
  opt-in via `YAV_RUN_TRYBUILD=1` because a nested Cargo invocation can
  deadlock under some Windows/Cargo combinations; a compile-fail suite with
  expected diagnostics is still absent.
- `rsx!`, `#[component]` and `#[derive(FromFormData)]` negative cases are
  covered by expansion-level unit tests in `yav-macros` (the `syn::Error` that
  becomes `compile_error!`). There is still **no `trybuild` compile-fail
  fixture suite**; negative cases are asserted at expansion level rather than
  by compiling a failing crate.
- MSRV is **Rust 1.88**, verified locally with
  `cargo +1.88.0 check --workspace --locked` and enforced by a dedicated CI
  job. The previously declared 1.75 was never achievable: `Cargo.lock` is v4
  (Cargo ≥ 1.78) and locked dependencies (`image`, `icu`, `encoding_rs`,
  `idna_adapter`) require up to rustc 1.88. Lowering it further would mean
  pinning older dependency releases.
- `examples/dashboard` is excluded from the workspace and must be checked
  separately (`cargo check --manifest-path examples/dashboard/Cargo.toml`).
- `promo-video/`, `assets/logos/` and the raw icon source trees are local
  artifacts, gitignored, and not part of the published framework.
- `test-app/` is a second demo project excluded from the workspace.

## 12. Escaping rules for component authors

The `Node::Raw` variant bypasses escaping. It is safe only for static, trusted
markup (such as the generated icon catalog). Components that accept
caller-provided HTML must sanitize it themselves before building `Node::Raw`;
there is no
global HTML sanitizer in `yav-core`. Any inline JavaScript must serialize
data with `serde_json::to_string` and escape `</script` sequences. All
built-in UI components now use delegated `data-*` attributes handled by
`YAV_UI_ACTIONS_SCRIPT` instead of inline `onclick` handlers, the
signals runtime no longer uses `new Function()` / `eval()`, and `WasmIsland`
supports request-level CSP nonces via `.with_nonce(...)`, making the
framework fully compatible with strict CSP policies.

`MarkdownEditor::preview_html` is sanitized by an allowlist; only
`preview_html_trusted()` emits caller HTML verbatim. The sanitizer covers
script/style tags, event handlers, dangerous URL schemes and unterminated
tags, but it is **not** a full HTML5 parser — do not use it as a general
purpose sanitizer for arbitrary third-party documents.

The optional `yav-build/a11y` feature adds a static HTML linter backed by
`html5ever`/`scraper`. It checks `img` alt text, accessible names for buttons
and links, input labelling and `html[lang]`. It is a CI aid, not a WCAG audit.

Security-relevant gaps and the disclosure process are in [`SECURITY.md`](../SECURITY.md).
