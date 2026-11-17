# Competitive feature and dependency roadmap

Status: analysis note, not a commitment to ship every item.  
Date: 2026-11-16.  
Scope: the nine workspace crates, the CLI, the docs, and the currently locked
dependency graph. Versions were checked with `cargo search` and the crates.io
API on the date above; re-check them before adding a dependency to a release.

This note answers two questions:

1. What would make Yav credible next to modern full-stack frameworks?
2. Which concrete Rust crates would support each addition, and at what cost?

## Implementation status (2026-11-16)

Implemented behind opt-in features: `sqlx` SQLite/PostgreSQL adapters and
executable migrations, Moka tagged cache, SQL-backed durable jobs, SMTP mail,
object storage, HTTP LLM client, Garde validation, JSON Schema/TypeScript
helpers, OAuth2 with PKCE and token exchange, Argon2id hashing, OTel SDK
exporter, a11y linting, advanced Markdown rendering, Fluent i18n, first-party
test harness, and Cargo metadata workspace inspection.

Partially implemented: per-icon tree-shaking is currently a deterministic
allowlist hook, not catalog rewriting; full OIDC `id_token` validation is not
wired on top of the OAuth client; `yav codegen` does not yet discover arbitrary
schema types automatically; `yav dev` still restarts the Rust process on
source changes.

Dependency deviations from the initial recommendation: no `fred`, `apalis`,
`ammonia`, `rig`, `qdrant-client`, `lancedb`, `openidconnect` or
`tracing-opentelemetry` was kept in the final graph. Redis caching is left to
the existing adapter trait, durable jobs are implemented directly on SQL, LLM
access uses the existing `reqwest` dependency, OAuth stops at the access token,
and the OTel SDK bridge exposes a provider and propagator rather than a
subscriber layer the framework cannot own. This keeps the default dependency
surface and MSRV stable.

All shipped features are forwarded through the `yav` facade crate, so an
application only needs `yav = { version = "0.1.5", features = ["sqlite"] }`.
The feature list, entry points and remaining boundaries are documented in
[`docs/opt-in-integrations.md`](opt-in-integrations.md).

Deliberately not copied: RSC/hydration, a spec-complete GraphQL engine, an
edge runtime, and a generated admin.

## 1. Baseline

This section is the **pre-implementation snapshot** taken before the opt-in
features were wired. The workspace still has **9 crates**, but `Cargo.lock` now
contains **497 packages** (after removing unused optional dependencies) and
`yav-server` declares **35 direct dependencies**, almost all of them optional.
The default feature set still resolves to the
small surface shown below; the numbers in this table are kept as the baseline
used to decide what to add. The current feature list is in
[`docs/opt-in-integrations.md`](opt-in-integrations.md).

At the time of the analysis the workspace had **267 locked packages** and this
direct dependency surface:

| Crate | Direct dependencies | Important existing libraries |
|---|---:|---|
| `yav-core` | 1 | `serde_json` |
| `yav-macros` | 3 | `syn`, `quote`, `proc-macro2` |
| `yav-router` | 3 | `serde`, `tracing` |
| `yav-server` | 22 | `axum`, `tokio`, `tower-http`, `reqwest` + `rustls-tls`, `image`, `p256`, `aes-gcm` |
| `yav-cli` | 15 | `clap`, `notify`, `notify-debouncer-mini`, `toml`; `cargo metadata` was not used yet at snapshot time (now used by `dx-inspect`) |
| `yav-build` | 8 | `flate2`, `reqwest`, `sha2`, `toml` |
| `yav-ui` | 3 | `yav-core`, `yav-icons`, `serde_json` |
| `yav-icons` | 1 | `yav-core` |
| `yav` | 7 | the other workspace crates only |

Already present transitively in `Cargo.lock`: `url`, `idna`,
`icu_normalizer`, `icu_properties`, `rustls`, `hyper`, `http`, `mime`,
`percent-encoding`, `bytes`, `smallvec`, `once_cell`, `regex`-free matching via
`aho-corasick`/`memchr`, and `image`/`png`.

Already absent from `Cargo.lock`: `sqlx`, `sea-orm`, `redis`, `fred`, `moka`,
`opentelemetry`, `argon2`, `webauthn-rs`, `oauth2`, `lettre`, `object_store`,
`insta`, `trybuild`, `html5ever`, `ammonia`, `fluent-bundle`, `icu` (the meta
crate), `chrono`, `uuid`.

Rule used below: prefer an optional feature in an existing crate before adding
a new direct dependency; prefer pure Rust; keep defaults small and make heavy
integrations opt-in.

## 2. What is already competitive

These capabilities do **not** need a new dependency; they need documentation,
tests, or wider adoption:

- File-system routing, layouts, intercepting routes and typed URL generation
  in `crates/yav-router/src/fs_router.rs`.
- SSR, streaming HTML and concurrent PPR in `crates/yav-server/src/streaming.rs`
  and `ppr.rs`.
- Contextual escaping for text, attributes, URLs, `style`, `srcdoc`, `srcset`
  and JSON-LD in `crates/yav-core/src/escape.rs` and `render.rs`.
- CSRF enabled by default, nonce-based CSP, security headers, ETags and signed
  sessions in `crates/yav-server/src/middleware.rs` and `session.rs`.
- 56 UI modules / 82 public constructors, 8 themes and 13,622 icons behind
  explicit Cargo features.
- 21 CLI commands, real CLI integration tests and real loopback HTTP tests.
- Parallel federation slots and opt-in automatic HTTP spans via
  `YavApp::otel(true)`.

The gap is not the rendering model. The gap is everything a team expects
around it: persistence, background work, operational tooling, type generation,
and quality gates.

## 3. Dependency-backed roadmap

### P0 — do first

| Feature to add | Recommended dependency | Integration point | Why now |
|---|---|---|---|
| Real database + executable migrations | `sqlx = "0.9.0"` with `runtime-tokio-rustls`, `sqlite`, `postgres`, `macros`, `migrate`; alternatively `sea-orm = "2.0.4"` if an ORM is wanted | `crates/yav-server/src/adapters.rs`, `db.rs`, `crates/yav-cli/src/db_cmd.rs` | Biggest credible gap versus Rails, Django, Laravel or Prisma. `db.rs` already knows migrations; it lacks an executor. |
| Typed server actions and shared validation | `garde = "0.23.0"` or `validator = "0.21.0"`, `serde_html_form = "0.4.1"`, `axum-extra = "0.12.6"` | `crates/yav-macros/src/server_macro.rs`, `crates/yav-server/src/forms.rs` | `#[server]` currently emits no typed client stub and accepts 0 or 1 argument. This is Yav's equivalent of SvelteKit remote functions. |
| JSON Schema / OpenAPI / TypeScript generation | `schemars = "1.2.3"` or `utoipa = "6.0.0"`, plus `ts-rs = "12.0.1"` if TS types should come from Rust structs | `openapi.rs`, `crates/yav-cli/src/codegen.rs` | Current OpenAPI is manually registered and `yav codegen --ts` does not parse arbitrary structs. This closes a visible CLI caveat. |
| Cache/revalidation by tags | `moka = "0.12.16"` for local TTL/TinyLFU, `fred = "10.1.0"` for Redis/Valkey, `redb = "4.6.0"` for durable local state | `cache.rs`, `ppr.rs`, `distributed_cache.rs` | Needed for Next-style `revalidateTag` semantics. `moka` is a real upgrade over `RwLock<HashMap>`; `fred` is pure Rust. |
| First-party test harness | `insta = "1.49.0"`, `assert_cmd = "2.2.2"`, `trybuild = "1.0.125"`, `axum-test = "21.1.0"`, `proptest = "1.12.0"` | dev-dependencies only; `tests/` inside each crate | Turns existing hand-written HTTP/CLI tests into a repeatable framework contract. `axum-test 22` is an RC, use `21.1.0`. |
| HTML sanitizing and static a11y checks | `ammonia = "4.2.5"`, `html5ever = "0.40.1"`, `scraper = "0.29.0"` | `yav-build` lint pass, `yav-ui` tests, `Node::Raw` boundary | The current sanitizer is intentionally not a full HTML5 parser. A browser-grade parser enables real ARIA/focus/alt checks in CI. |

### P1 — after the P0 foundation

| Feature to add | Recommended dependency | Integration point | Why now |
|---|---|---|---|
| Durable jobs and cron | `apalis = "1.2.0"` with a SQLite/Redis backend, plus `tokio-cron-scheduler = "0.15.1"` or `cron = "0.17.0"` | `crates/yav-server/src/jobs.rs` | Current jobs are in-process. Rails 8 and Laravel treat queues as table stakes. |
| Real SMTP email | `lettre = "0.11.23"` | `crates/yav-server/src/email.rs`, `adapters.rs` | `Mailer` currently writes `.eml` files. Keep RSX templates; replace only the transport. |
| OAuth/OIDC completion | `oauth2 = "5.0.0"`, `openidconnect = "4.1.0"` | `crates/yav-server/src/auth.rs` | The PKCE module only builds URLs; it does not exchange tokens or fetch userinfo. |
| Stronger password and passkey assurance | `argon2 = "0.6.0"` or `password-auth = "2.0.0"`, `webauthn-rs = "0.6.2"` | `auth.rs`, `passkeys.rs` | Hand-rolled PBKDF2 is honest but not competitive. `webauthn-rs` covers attestation and metadata. |
| Faster DX / real HMR | `cargo_metadata = "0.23.1"`, `tracing-tree = "0.4.1"`; reuse existing `notify` | `crates/yav-cli/src/dev.rs`, `reload.rs`, `crates/yav-build` | Rust edits still restart the process. DOM morphing already exists in the reload script; it can be reused. No Node bundler. |
| i18n with plurals and formats | `fluent-bundle = "0.16.0"`, `unic-langid = "0.9.6"`, `icu = "2.4.0"` or `jiff = "1.0.0"` | `crates/yav-core/src/i18n.rs`, `crates/yav-build/src/i18n_lint.rs`, `yav-ui` | The current dictionary is simple interpolation. ICU components already exist transitively through `url`; the meta crate does not. |

### P2 — differentiate or defer

| Feature to add | Recommended dependency | Integration point | Why now |
|---|---|---|---|
| Standard observability stack | `opentelemetry = "0.33.1"`, `opentelemetry_sdk = "0.33.1"`, `opentelemetry-otlp = "0.33.1"`, `tracing-opentelemetry = "0.34.0"`, `opentelemetry-prometheus = "0.33.1"`; lighter alternative: `metrics = "0.24.6"` + `metrics-exporter-prometheus = "0.18.3"` | `crates/yav-server/src/otel.rs`, `metrics_dashboard.rs` | Replace the hand-rolled tracer with the ecosystem standard. Validate version alignment in the resolver before committing. |
| Object storage backends | `object_store = "0.14.2"` or `opendal = "0.59.5"` | `storage.rs`, `adapters.rs` | Both S3/GCS/Azure/local behind one trait. Avoid `aws-sdk-*` directly unless AWS-specific depth is required. |
| Markdown and code rendering | `pulldown-cmark = "0.13.6"` or stable `comrak = "0.59.0"`, `syntect = "5.3.0"` | `yav-build`, `markdown_editor.rs`, `code_block.rs` | Content sites are a core Next/Astro use case. `comrak 1.0.0-rc.2` is not an RC to ship by default. |
| Optional AI providers and vector search | `rig = "0.46.0"`, `qdrant-client = "1.20.0"`, `lancedb = "0.41.0"`, `uuid = "1.29.2"` | `ai.rs`, `adapters.rs` | Keep strictly optional; no default model, no bundled embedding runtime. This upgrades existing `VectorIndex` and `ToolRegistry` without changing the framework's DNA. |
| Per-icon tree-shaking | No new runtime crate; a `build.rs` using existing `syn`/`quote`/`proc-macro2` and optional `cargo_metadata` | `crates/yav-icons` | Reduces binary size, one of Yav's strongest differentiators. Pure engineering, no ecosystem dependency. |

## 4. Versions to avoid or to treat carefully

| Crate | Latest shown | Stable alternative | Decision |
|---|---|---|---|
| `redis` | `2.0.0-rc.3` | `1.8.2` | Prefer `fred 10.1.0` for a pure Rust client. |
| `dashmap` | `7.0.0-rc2` | `6.2.1` | `moka` is a better semantic fit for caches. |
| `axum-test` | `22.0.0-rc.1` | `21.1.0` | Use the stable line in dev-dependencies. |
| `comrak` | `1.0.0-rc.2` | `0.59.0` | Avoid RC in the default build. |
| `lightningcss` | `1.0.0-alpha.72` | no stable release reported | Do not adopt yet; keep current minifiers. |
| `sled` | `1.0.0-alpha.124` | `0.34.7` | Prefer `redb 4.6.0`. |
| `diesel` / `diesel-async` | `2.4.0` / `0.10.0` | stable | Powerful, but sync-first and SQLite often needs native libs; `sqlx` fits the pure Rust story better. |
| `aws-sdk-*` | varies | n/a | Heavy; use `object_store` or `opendal` behind a feature instead. |
| Node/Vite/Turbopack tooling | n/a | n/a | Out of scope: contradicts "skip the Node toolchain". |

## 5. Suggested Cargo feature layout

Do not make all of this default. A reasonable split:

```toml
[workspace.dependencies]
sqlx = { version = "0.9", default-features = false, features = ["runtime-tokio-rustls", "macros", "migrate"], optional = true }
moka = { version = "0.12", optional = true }
fred = { version = "10.1", optional = true }
garde = { version = "0.23", optional = true }
schemars = { version = "1.2", optional = true }
lettre = { version = "0.11", default-features = false, optional = true }
opentelemetry = { version = "0.33", optional = true }
```

```toml
# in yav / yav-server
[features]
default = ["classic-icons"]          # unchanged
sqlite = ["dep:sqlx", "sqlx/sqlite"]
postgres = ["dep:sqlx", "sqlx/postgres"]
redis-cache = ["dep:fred"]
validation = ["dep:garde"]
openapi-derive = ["dep:schemars"]
smtp = ["dep:lettre"]
otel-sdk = ["dep:opentelemetry"]
```

Rules for reviewers:

- `yav new` must still compile with no external service configured.
- A new dependency must be justified by a test that fails without it.
- Heavy integrations are opt-in features, not default features.
- Preserve `rust-version = "1.88"` or update it explicitly in the same commit.
- Preserve `reqwest` with `default-features = false, features = ["rustls-tls"]`;
  do not let a new crate silently add `native-tls`/OpenSSL.

## 6. Phasing

1. **0.1.6 — quality and credibility**  
   Add dev-only test dependencies, HTML5 sanitizing/a11y lint, validation on
   `#[server]`, and fix OpenAPI/TS generation. No new default runtime dependency.

2. **0.2.0 — operational core**  
   Add `sqlite` first, then `postgres`; wire `db::Migrator` to `sqlx::migrate`;
   introduce tagged cache/revalidation behind `moka`. Keep `fred` optional.

3. **0.3.0 — full-stack parity**  
   Durable jobs via `apalis`, SMTP via `lettre`, OAuth/OIDC completion,
   optional `argon2`/`webauthn-rs` hardening.

4. **Later — differentiated scale**  
   Standard OTel stack, object storage backends, richer i18n, optional AI
   providers, per-icon tree-shaking, published benchmarks.

## 7. Success criteria

For each dependency added, record before/after numbers for:

- `cargo check --workspace --locked` duration;
- `cargo test --workspace --locked` duration;
- default `yav new` binary size;
- number of direct dependencies in `yav-server`;
- MSRV and OpenSSL/`native-tls` presence in `cargo tree`.

The goal is not "more dependencies". The goal is to close a documented
limitation with a real implementation while keeping the no-Node, single-binary,
pure-RTLS position intact.

## Sources

- Local code: `Cargo.toml`, all `crates/*/Cargo.toml`, `Cargo.lock`,
  `docs/limitations.md`, `README.md`.
- Version check: `cargo search <crate> --limit 1`, 2026-11-16.
- Pre-release/stable distinction: crates.io API `max_version` and
  `max_stable_version`, 2026-11-16.
- Framework benchmark: Next.js Cache Components, SvelteKit remote functions,
  Astro server islands, React Router 7 framework mode, Nuxt 4 server-agnostic
  server, Rails 8 Solid Queue/Cache, Laravel 12 queues, Leptos and Dioxus
  server functions.
