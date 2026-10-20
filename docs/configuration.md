# Configuration Reference (`package.yavml`)

The `package.yavml` manifest is the central project descriptor for Yav applications, serving a role equivalent to `package.json` in React/Node.js or `pubspec.yaml` in Dart/Flutter.

It allows you to declare project identity, HTTP server settings, Tailwind CSS compiler options, build targets, dependencies, custom scripts, and static assets in one clear location.

---

## Full Example

```toml
[package]
name = "my-awesome-app"
version = "0.1.0"
description = "A blazing-fast web application built with Yav"
authors = ["Jane Doe <jane@example.com>"]
license = "MIT"
repository = "https://github.com/my-org/my-awesome-app"
homepage = "https://my-awesome-app.com"
keywords = ["yav", "web", "rust"]

[server]
port = 3000
host = "127.0.0.1"
cors = false
https = false
tls_cert = ""
tls_key = ""

[tailwind]
enabled = true
version = "3.4.17"
input = "src/styles/app.css"
output = "public/styles.css"

[motion]
enabled = true
reduced_motion = "user"

[pwa]
enabled = true
name = "My Awesome App"
short_name = "AwesomeApp"
theme_color = "#0f172a"
background_color = "#0f172a"
display = "standalone"
start_url = "/"

[build]
out_dir = "dist"
public_dir = "public"
minify = true
fingerprint = true

[dependencies]
# Yav plugins and feature packages
# Simple version requirement:
# yav-auth = "0.1"
# Detailed dependency:
# yav-analytics = { git = "https://github.com/yav-rs/analytics.git", branch = "main", features = ["charts"] }

[scripts]
dev = "yav dev"
build = "yav build"
test = "cargo test"

[assets]
favicon = "public/favicon.ico"
fonts = []
static_dirs = []
```

---

## Sections

### `[package]`
Project metadata and identity.

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | String | `"my-app"` | Application name (must be a valid identifier) |
| `version` | String | `"0.1.0"` | SemVer version string |
| `description` | String | `""` | Short description of the project |
| `authors` | Array<String> | `[]` | List of author names and emails |
| `license` | String | `"MIT"` | SPDX license identifier |
| `repository` | String | `""` | Source code repository URL |
| `homepage` | String | `""` | Project website or documentation URL |
| `keywords` | Array<String> | `[]` | Search tags |

### `[server]`
Runtime HTTP server settings. Used automatically by `YavApp::new()`.

| Field | Type | Default | Description |
|---|---|---|---|
| `port` | Integer | `3000` | Port to bind the HTTP server to |
| `host` | String | `"127.0.0.1"` | Host address to bind (`"0.0.0.0"` for all interfaces) |
| `cors` | Boolean | `false` | Enable standard Cross-Origin Resource Sharing |
| `https` | Boolean | `false` | Enable TLS/HTTPS |
| `tls_cert` | String | `""` | Path to TLS certificate PEM file |
| `tls_key` | String | `""` | Path to TLS private key PEM file |

### `[tailwind]`
Tailwind CSS standalone CLI integration (no Node.js required).

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | Boolean | `true` | Enable automated Tailwind compilation |
| `version` | String | `"3.4.17"` | Official Tailwind standalone CLI binary version |
| `input` | String | `"src/styles/app.css"` | Source CSS file with `@tailwind` directives |
| `output` | String | `"public/styles.css"` | Generated and purged CSS output file |

### `[motion]`
Motion & Animation runtime configuration (`yav-motion`). Enabled by default.

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | Boolean | `true` | Enable the hardware-accelerated WAAPI motion runtime |
| `reduced_motion` | String | `"user"` | Accessibility policy: `"user"` (honors OS preference), `"always"`, or `"never"` |

### `[pwa]`
Progressive Web App (PWA) configuration for installable mobile and desktop applications.

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | Boolean | `false` | Enable PWA features and manifest serving |
| `name` | String | `""` | Full application name (defaults to package name) |
| `short_name` | String | `""` | Short name for home screen app tiles |
| `theme_color` | String | `"#0f172a"` | Browser address bar and status bar color |
| `background_color`| String | `"#0f172a"` | Splash screen background color |
| `display` | String | `"standalone"` | App display mode (`"standalone"`, `"fullscreen"`, `"minimal-ui"`, `"browser"`) |
| `start_url` | String | `"/"` | Entry point when launched from home screen |

### `[build]`
Production build and asset optimization settings.

| Field | Type | Default | Description |
|---|---|---|---|
| `out_dir` | String | `"dist"` | Directory for deployable build artifacts |
| `public_dir` | String | `"public"` | Static assets directory |
| `minify` | Boolean | `true` | Minify CSS and HTML in production builds |
| `fingerprint` | Boolean | `true` | Append content hashes (`styles.a1b2c3d4.css`) for cache busting |

### `[dependencies]`
Declare Yav ecosystem plugins and extensions:

```toml
[dependencies]
yav-auth = "0.1"

[dependencies.yav-database]
version = "0.2"
features = ["postgres"]

[dependencies.my-custom-plugin]
path = "../plugins/my-custom-plugin"
```

### `[scripts]`
Custom tasks and command shortcuts:

```toml
[scripts]
dev = "yav dev"
build = "yav build"
test = "cargo test"
lint = "cargo clippy"
deploy = "fly deploy"
```

### `[assets]`
Static asset settings:

```toml
[assets]
favicon = "public/favicon.ico"
fonts = ["https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700"]
static_dirs = ["public/images", "public/fonts"]
```

---

## Automatic Discovery & Backwards Compatibility

When starting `YavApp::new()`, the framework runs `PackageYav::discover()`:
1. Looks for `package.yavml` in the current working directory.
2. If absent, falls back to legacy `package.yav`.
3. If absent, falls back to legacy `yav.toml` and converts it automatically.
4. If none is present, safe defaults are loaded (`127.0.0.1:3000`).

You can also load the manifest programmatically:

```rust
use yav::build::PackageYav;

let manifest = PackageYav::discover().expect("Failed to load package.yavml");
println!("App name: {}", manifest.package.name);
println!("Listening on port: {}", manifest.server.port);
```
