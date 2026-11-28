# Opt-in integrations

Status: shipped in the workspace, off by default.  
Date: 2026-11-17.

The default `yav` build still links only the small dependency set needed for
server-rendered HTML. The integrations added from the
[competitive roadmap](competitive-roadmap.md) are compiled only when their
Cargo feature is enabled. None of them activate themselves at runtime, and
none of them pretend a service is available when it is not configured.

## Enabling them

Features can be enabled on the facade crate. `all-integrations` is a convenience
bundle for validation and CI, not a recommended production default.

```toml
[dependencies]
yav = { version = "0.1.8", features = ["sqlite", "validation", "smtp"] }
```

```bash
# Compile and run every gated test
cargo test --workspace --all-features --locked
```

The underlying modules are always reachable through
`yav::server::roadmap::<module>` (or `yav_server::roadmap::<module>` when
depending on `yav-server` directly). A submodule exists only if its feature is
enabled; the `roadmap` module itself is always present and may be empty.

## Server features

| Feature | Module | Main entry points | What it actually does |
|---|---|---|---|
| `sqlite` | `roadmap::persistence` | `SqliteDatabaseAdapter::connect` | Real pooled `sqlx` SQLite connection; `execute`, `query_rows` and `health_check` run SQL. |
| `postgres` | `roadmap::persistence` | `PostgresDatabaseAdapter::connect` | Same adapter contract against PostgreSQL. |
| — | `roadmap::persistence` | `Migrator::apply_all_with`, `Migrator::rollback_last_with_adapter` | Executes discovered SQL migrations through any `DatabaseAdapter`, verifies checksums and journals applied versions. Available without a Cargo feature. |
| `tagged-cache` | `roadmap::persistence` | `MokaCacheAdapter::new` | Bounded `moka` cache with per-entry TTL and tag-based invalidation. Process-local. |
| `durable-jobs` | `roadmap::jobs` | `SqliteJobQueue::{ensure_schema, enqueue, try_claim, ack, retry}` | SQL-backed `yav_jobs` table with attempts, `run_at` and claim ownership. Errors are propagated, never swallowed. |
| `validation` | `roadmap::validation` | `validate_with_garde`, `ValidatedForm` | Converts a `garde::Report` into the existing `FormErrors` type. |
| `schema-codegen` | `roadmap::schema` | `json_schema`, `ts_type`, `ts_declarations` | Emits JSON Schema and TypeScript declarations for a Rust type. Helpers only; no automatic type discovery. |
| `smtp` | `roadmap::email` | `SmtpMailAdapter::new`, `from_env` | Real `lettre` SMTP delivery over implicit TLS (465) or STARTTLS. Reads `YAV_SMTP_RELAY`, `YAV_SMTP_PORT`, `YAV_SMTP_FROM`, `YAV_SMTP_USERNAME`, `YAV_SMTP_PASSWORD`. |
| `argon2` | `roadmap::security` | `Argon2Hasher::{hash_password, verify_password}` | Argon2id PHC strings. |
| `oauth` | `roadmap::oauth` | `OAuthTokenClient::{new, authorization_url, exchange_code, userinfo}` | Authorization URL with CSRF state and PKCE S256, real token exchange, and a Bearer-authenticated userinfo call. |
| `otel-sdk` | `roadmap::otel` | `otlp_exporter`, `tracer_provider`, `install_propagator` | OTLP/HTTP span exporter, batch tracer provider with `service.name`, and the W3C Trace Context propagator. |
| `object-storage` | `roadmap::storage` | `ObjectStoreAdapter::new` | Wraps a caller-supplied `object_store::ObjectStore` for `put`/`get`/`delete`. |
| `http-llm` | `roadmap::llm` | `HttpLlmAdapter::new`, `from_env` | Calls an OpenAI-compatible `/v1/chat/completions` or `/v1/embeddings` endpoint through the existing `reqwest` dependency. Requires `YAV_LLM_API_KEY` (`YAV_LLM_BASE_URL` optional). |

## Build, core, icons and CLI features

| Feature | Crate | Main entry points | What it actually does |
|---|---|---|---|
| `a11y` | `yav-build` | `lint_html`, `lint_report_html` | Static HTML checks for `img-alt`, `button-name`, `input-label`, `link-name`, `html-lang`. |
| `markdown` | `yav-build` | `render_markdown`, `highlight_code` | GFM rendering through `pulldown-cmark` with raw HTML disabled, plus `syntect` highlighting. |
| `i18n-advanced` | `yav-core` | `FluentCatalog::{new, format}`, `choose_plural` | Fluent message catalogs with typed arguments; minimal plural rules for en/fr/de/es. |
| `icon-minify` | `yav-icons` | `allowed_icon_names`, `is_allowed` | Deterministic build-time allowlist hook exposed by `build.rs`. |
| `dx-inspect` | `yav-cli` | `dx::workspace_summary` | Prints a `cargo metadata` workspace summary at CLI startup. |

## Boundaries that still apply

- **`object-storage` does not construct a backend for you.** `yav-server`
  declares `object_store` with `default-features = false`, so cloud and `fs`
  features must be enabled in the application crate, which then passes an
  `Arc<dyn ObjectStore>` to `ObjectStoreAdapter::new`. `from_url` returns an
  explicit error instead of inventing a URL. `presign_put` fails explicitly on
  backends that cannot presign.
- **`otel-sdk` is not full auto-instrumentation.** It installs the propagator
  and builds a provider. The application must still register the provider
  globally, attach a `tracing_opentelemetry` layer to its subscriber, and bridge
  the existing `traceparent` middleware into SDK span contexts. The zero-SDK
  helpers in `yav::server::otel` remain unchanged.
- **`tagged-cache` is process-local.** It is not a distributed cache and does
  not coordinate invalidation across nodes.
- **`oauth` stops at the access token.** Full OIDC `id_token` signature and
  claims validation is not wired on top of the OAuth client.
- **`icon-minify` performs compile-time allowlist pruning** when
  `YAV_ICON_ALLOWLIST` is non-empty, omitting unlisted icons from the compiled
  `IconData` slices.
- **`schema-codegen` + `yav generate`** emit `schemars` / `ts-rs` schemas in
  application code and scan `#[derive(Serialize/Deserialize/ApiSchema/TS/JsonSchema)]`
  structs into `types.d.ts` and `openapi.json` via `yav generate`.
- **`a11y` is a static rule set**, not an automated WCAG audit or a keyboard
  test runner. `i18n-advanced` covers four locales' plural rules, not all CLDR
  languages.
- **`http-llm` needs network access and a key.** With no key it returns an
  error; there is no mock model.

For the reasoning and dependency versions behind these additions, see the
[competitive roadmap](competitive-roadmap.md).
