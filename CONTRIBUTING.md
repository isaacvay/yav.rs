# Contributing to Yav

Thanks for contributing. Yav is a Rust workspace; everything below runs from
the repository root (`D:/deep/yav.rs` in local development, but use your own
clone path).

## Workspace layout

| Crate | Responsibility |
| --- | --- |
| `yav-core` | RSX `Node`/`Element`, HTML rendering, escaping, WASM islands |
| `yav-macros` | `rsx!`, `#[component]`, derive macros, `#[server]` |
| `yav-router` | Radix route trie, file-based routing, reverse routing |
| `yav-server` | Axum app builder, middleware, auth, sessions, built-in endpoints |
| `yav-ui` | Component library (one module per component family) |
| `yav-icons` | Generated icon catalog (Lucide / MynaUI / Phosphor) |
| `yav-cli` | `yav` binary: `new`, `dev`, `build`, `export`, `db`, `doctor`, … |
| `yav-build` | Tailwind bootstrap, manifest parsing, asset pipeline |
| `yav` | Umbrella crate re-exporting the public API |

`examples/dashboard` is deliberately **excluded** from the workspace (see
`Cargo.toml`) and must be checked separately.

## Before opening a pull request

```sh
cargo fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --doc
cargo check -p yav                       # Classic-only icon build
cargo check -p yav --features full-icons # All three icon families
cargo check --manifest-path examples/dashboard/Cargo.toml
```

`crates/yav-cli/tests/cli_integration.rs` spawns the real binary; it is covered
by `cargo test --workspace`. Do not replace process exit-code assertions with
unit tests of the command functions.

CI runs the same commands on Linux, macOS and Windows plus `cargo audit` and
a dedicated **MSRV job** pinned to the `rust-version` in the workspace
manifest. The current verified MSRV is **Rust 1.88** (`cargo +1.88.0 check
--workspace --locked`); don't use language or library features newer than
that without discussing it first, and don't bump dependency versions without
re-running the MSRV check — several transitive crates already require 1.88.

## Coding rules

- **Never report a fake success.** If a download, a database command, an SSR
  render or an export fails, return an error with a non-zero exit code and an
  actionable message. No `unwrap_or(false)` to hide a failure, no
  `let _ = fs::write(...)` on a required file.
- **Escape at the boundary.** Use `Node::text` / `Element::attr` for anything
  derived from user input. `Node::raw` is only acceptable for static, trusted
  markup, and must be accompanied by a comment saying so. Any inline
  JavaScript interpolation must go through `serde_json::to_string` and escape
  `</script`.
- **Verify cryptography, do not simulate it.** Signature checks, token
  validation and checksum verification must be real and must have a negative
  test (tampered signature, empty signature, wrong checksum). Use
  `JwtManager::verify_with_config` (not legacy `verify`) for new auth code.
- **CSRF is enabled by default and mandatory for new state-changing routes.**
  If you add a POST/PUT/PATCH/DELETE handler, make sure it works with the
  default `CsrfConfig` — carry `_csrf` in forms or
  `x-csrf-token` in AJAX calls, and document any webhook exemption.
- **Never rewrite untrusted paths.** Decoding/normalisation must be followed by
  explicit rejection of `..`, drive letters, control characters and reserved
  device names; `replace("..", "")` is not a security control.
- **Add a test for every bug fix.** Include at least one negative/security
  test for anything touching auth, escaping, headers or crypto.
- Shell out with escaped arguments only; never concatenate untrusted input
  into a shell command.
- No new `expect("Lock poisoned")` in production paths.

## Adding or regenerating icons

The catalog in `crates/yav-icons/src/catalog/*.rs` is **generated**. Do not
hand-edit it. The upstream SVG trees are local-only and gitignored; see
`crates/yav-icons/scripts/`. After regenerating:

1. Keep every generated slice sorted by `name` (lookups use `binary_search_by_key`).
2. Update `ICON_COUNT` in `src/catalog/mod.rs`.
3. Re-check the attribution table in `crates/yav-icons/NOTICE.md` and make
   sure the three license files remain at the crate root so they are packaged.
4. Run `cargo package --list -p yav-icons` and confirm the license files are
   included and the raw SVG trees are not.
5. Do not assume all three families are compiled: `yav`'s default is
   `classic-icons` only. Any new CLI/example code that needs Minimal or System
   icons must opt into `minimal-icons`, `system-icons` or `full-icons` in its
   own `Cargo.toml`. Test both `cargo check -p yav` and
   `cargo check -p yav --features full-icons`.

## Documentation

- Put user-facing docs in `docs/`, keep links **relative** (no `file:///` URLs).
- The framework version lives in `[workspace.package] version` in the root
  `Cargo.toml`; do not hardcode a different version in docs.
- When you add a partially implemented feature, add a line to
  [`docs/limitations.md`](docs/limitations.md). Prefer an honest limitation
  over an aspirational README claim.

## Reporting vulnerabilities

Do **not** open a public issue for security bugs. See
[`SECURITY.md`](SECURITY.md) for the disclosure process.

## Commit style

Short imperative subject lines (`fix: escape WASM island params`). Group
related changes; keep generated catalog changes in their own commit.
