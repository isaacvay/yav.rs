# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| `0.1.5` (current workspace version) | Yes |
| `0.1.x` older releases | No — upgrade to 0.1.5 |
| Unreleased `main` | Best effort; not recommended for production |

Yav is pre-1.0. Security fixes land on `main` and are released as the next
`0.1.x` patch; there are no long-term support branches yet.

> **Release metadata caveat:** the `0.1.5` artifact on crates.io declares
> `rust-version = 1.75`, while the workspace and its locked dependency graph
> require **Rust 1.88**. `cargo install yav-cli` on 1.75 is accepted by Cargo
> and then fails to compile. This is corrected in the next release.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Send a private report to **isaacyavs@gmail.com** with the subject
`[SECURITY] yav.rs: <short summary>`. Include:

- the affected crate and version / commit,
- a minimal reproducer (request, payload, or code snippet),
- the impact you believe it has,
- whether you have already disclosed it anywhere.

Expected handling: acknowledgement within 5 business days, an assessment and
fix plan within 15 business days, and a coordinated disclosure once a fix is
available. If you do not hear back within 5 business days, please re-send —
do not escalate to a public issue without giving us that window.

Please do **not** run destructive tests against third-party deployments, and
do not include real credentials or personal data in the report.

## Security-relevant design notes

These are the controls currently implemented; they are also the areas where
regressions matter most.

- **Sessions**: AES-256-GCM encrypted cookies authenticated with HMAC.
  If `SESSION_SECRET` / `YAV_SECRET` is unset, the process generates **one**
  ephemeral CSPRNG secret, logs a warning and starts. Sessions are then
  invalidated on every restart and are **not** shared between instances — set
  the variable for any multi-instance deployment. Generated systemd/Docker
  units require it explicitly. Flash cookies follow the same behaviour (no
  startup panic), and legacy flash payloads without an expiry are rejected.
- **Passwords**: PBKDF2-HMAC-SHA256 hashing (one reused HMAC instance per
  derivation). Login is a synchronous CPU-bound operation; put a rate limiter
  in front of it.
- **JWT**: `JwtManager::verify_with_config` / `verify_at` enforce `alg`, `exp`,
  `nbf`, optional `iss`/`aud`, optional future-`iat` rejection and configurable
  clock leeway. The legacy `verify()` keeps its permissive behaviour for
  backwards compatibility — **use the config-aware API for new code**.
- **CSRF**: `CsrfConfig` + `csrf_middleware` enforce a session-bound token on
  POST/PUT/PATCH/DELETE, via the `x-csrf-token` header or the `_csrf` form
  field, with constant-time comparison and configurable path exemptions.
  It is **enabled by default** (`CsrfConfig::default().enabled == true`).
  Disable it explicitly with `CsrfConfig::disabled()` for bearer-token-only
  JSON APIs, or customise it via `YavApp::csrf_protection(...)`.
- **CSP**: per-response nonces injected into `<script>` tags. Keep inline
  handlers out of components; use `yav:*` declarative attributes.
- **WebAuthn / passkeys**: real ES256 (P-256) signature verification with a
  COSE_Key CBOR decoder. Empty or tampered assertions are rejected.
- **Web Push**: real ES256 ECDSA VAPID JWTs and RFC 8291 `aes128gcm`
  payload encryption. Keep the VAPID private key out of source control.
- **Escaping**: `Node::text` and `Element::attr` escape by default.
  `Node::raw` is only safe for static trusted markup — see
  [`docs/limitations.md`](docs/limitations.md).
- **CLI**: user arguments are shell-escaped; downloaded Tailwind binaries are
  SHA-256 verified before use.
- **Static files**: path traversal is blocked (including percent-encoded
  variants), hidden files/directories (`.env`, `.git`, `.htaccess`) are
  rejected, and `OPTIONS` no longer emits a wildcard CORS header.
- **Static export**: output paths are percent-decoded, normalised for `/` and
  `\`, and reject `..`, drive letters, control characters and Windows
  reserved device names instead of silently rewriting them.
- **Markdown preview**: `MarkdownEditor::preview_html` runs an allowlist
  sanitizer (tags, attributes, URL schemes, event handlers). Raw caller HTML
  is only emitted through the explicit `preview_html_trusted()` escape hatch.

## Known gaps

Known limitations that affect the security posture are documented in
[`docs/limitations.md`](docs/limitations.md). In particular: CSRF is enabled
by default but session revocation still requires configuring a
`SessionRevocationList`; passkey attestation-chain validation is not
implemented; `pdf_signer` is an HMAC integrity tag rather than a PAdES/X.509
signature; rate limiting is optional and must be enabled explicitly. Web Push
endpoints are now validated by an SSRF guard (https-only, no userinfo, no
private/loopback/link-local hosts, no redirects), but applications should
still keep their own allowlist of trusted push services.

The default CSP uses a per-response nonce plus `'strict-dynamic'`; it no
longer permits arbitrary HTTPS scripts, and the nonce-less fallback policy
allows `script-src 'self'` only. Applications needing a CDN must add the
exact origin through a custom policy.
