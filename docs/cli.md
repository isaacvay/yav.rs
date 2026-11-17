# Yav CLI Documentation

The `yav` CLI is the developer toolbelt for creating, developing, scaffolding, and deploying Yav web applications.

---

## Commands Reference

### 1. `yav new <name> [--check]`
Scaffolds a new Yav web project with Tailwind CSS, file-based routes, and recommended configuration.

Without `--check`, the command reports the project as created but **not compiled**. Pass `--check` to run `cargo check` inside the generated directory before reporting success; it exits non-zero if the project does not build. Verification is opt-in because a fresh scaffold resolves its `yav` dependency from the registry and may need network access.

```bash
yav new mon-application
cd mon-application

# or verify the generated project immediately:
yav new mon-application --check
```

---

### 2. `yav dev [--port 3000]`
Starts the development server with:
- **Instant Hot Live Reload** via SSE and WebSocket.
- **Incremental Tailwind CSS compilation**.
- **Browser Error Overlay** (glassmorphism dark-mode modal displaying compiler error traces directly on the page, with automatic recovery upon saving the fix).

```bash
yav dev
# Or custom port:
yav dev --port 8080
```

---

### 3. `yav add <component>`
Adds production-ready, styled UI components directly to your `src/components/` directory (similar to shadcn/ui). The generated code is 100% yours to customize and modify.

```bash
yav add auth-card        # Login / Register card with validation inputs
yav add stat-grid        # 4-card responsive KPI metric grid
yav add data-table       # Searchable, paginated table with status badges
yav add pricing-table    # 3-tier pricing plans with features checklist
yav add navbar           # Responsive sticky navbar with logo and links
yav add drawer           # Sliding drawer / sheet overlay component
yav add command-menu     # Spotlight / Command palette modal with shortcuts
yav add stat-card        # KPI metric card with trend indicator and badge
yav add carousel         # Responsive media carousel with swipe and dots
yav add theme-selector   # Multi-theme dropdown & instant theme switcher (8 themes)
yav add motion           # Spring-physics motion container & gestures
yav add cursor-spotlight # Radial glow mouse-follower plate with CSS coordinates
yav add charts           # Server-rendered SVG Line/Bar/Donut/Sparkline charts panel
yav add data-grid        # Searchable, sortable, selectable enterprise DataGrid
yav add date-picker      # Accessible calendar DatePicker field
yav add input-otp        # Multi-digit 2FA/TOTP InputOtp verification block
yav add kanban-board     # Multi-column drag-and-drop Kanban board with LexoRank
yav add flow-canvas      # Interactive SVG node-graph and workflow builder canvas
yav add virtual-list     # 100,000+ row virtualized scroll container
yav add rich-editor      # Notion/TipTap-style rich text editor with toolbar
yav add ai-chat          # Streaming SSE AI chat interface
```

Once added, the component is automatically declared in `src/components/mod.rs` and can be imported immediately:

```rust
use crate::components::AuthCard;

rsx! {
    <AuthCard 
        title="Connexion" 
        subtitle="Accédez à votre espace membre" 
        submit_label="Se connecter" 
    />
}
```

---

### 4. `yav build`
Builds your project for production:
- Compiles minified Tailwind CSS with full purging.
- Applies asset fingerprinting (`styles.[hash].css`) and pre-compresses `.gz` assets in a single in-memory pass.
- Generates `dist/asset-manifest.json`.
- Compiles the optimized Rust release binary.

```bash
yav build
```

---

### 5. `yav update` (aliases: `yav up`, `yav upgrade`)
Updates the `yav` CLI binary itself and upgrades Yav dependencies in the current project (`package.yavml`, `Cargo.toml`, `Cargo.lock`, and the cached standalone Tailwind CSS binary):

```bash
# Update both the yav CLI and the current project (if inside a Yav project)
yav update

# Only check for available updates without modifying files or installing
yav update --check

# Only update the yav CLI binary itself
yav update --self

# Only update the current project's manifests and Cargo.lock
yav update --project

# Update yav-cli directly from the official GitHub repository (main branch)
yav update --git

# Force reinstallation / update even if already up to date
yav update --force
```

---

### 6. `yav check`
Verifies project configuration (`package.yavml`, legacy `package.yav`, or `yav.toml`), ensures Tailwind assets exist, and performs a fast `cargo check`.

```bash
yav check
```

---

### 7. `yav generate` (alias: `yav g`)
Scaffolds boilerplate for routes, functional components, or layouts with idiomatic conventions:

```bash
# Generate a new route handler in src/routes/
yav g route /dashboard
yav g route /blog/[slug]

# Generate a new component in src/components/
yav g component StatCard
yav g component user-profile

# Generate a layout wrapper in src/layouts/
yav g layout AdminLayout
```

---

### 8. `yav icon`
Explores, searches, and inspects the built-in 13,000+ SVG icons catalog (Classic/Lucide, Minimal/MynaUI, and System/Phosphor):

```bash
# Search icons by keyword
yav icon search user
yav icon search arrow

# List all icons or filter by category
yav icon list
yav icon list --category navigation

# Inspect code snippet and details for an icon
yav icon info home
```

---

### 9. `yav docker`
Generates an ultra-optimized, multi-stage production Docker deployment:

```bash
# Generate Dockerfile, docker-compose.yml, and .dockerignore
yav docker

# Overwrite existing docker files if needed
yav docker --force
```
* **Multi-Stage Build**: Caches cargo dependencies and compiles a standalone, statically linked binary.
* **Minimal Footprint**: Deploys on an Alpine/Distroless container with non-root security (`appuser`) and automated `/healthz` health checks, resulting in a production image under 30MB!

---

### 10. `yav sitemap`
Scans your `src/routes/` directory to generate standard `public/sitemap.xml` and `public/robots.txt` files:

```bash
# Generate static search engine discovery files
yav sitemap --base-url https://example.com
```

### 11. `yav db`
Manages database initialization, schema versioning, and migration execution for SQLite and PostgreSQL:

```bash
# Initialize migrations/ directory and .env configuration
yav db init

# Create a new timestamped migration (e.g. migrations/1730000000_add_user_profiles.sql)
yav db new add_user_profiles

# List existing migrations, applied state, and checksum verification
yav db status

# Apply pending Up SQL migrations against SQLite or PostgreSQL database
yav db migrate

# Roll back the most recently applied Down SQL migration
yav db rollback

# Execute database seed data (migrations/seed.sql)
yav db seed
```

* **Automated Runner Execution**: Reads `DATABASE_URL` directly from `.env` or the environment. If targeting SQLite (`sqlite://app.db` or a file path), it automatically executes SQL statements against the SQLite database; if targeting PostgreSQL (`postgres://...`), it connects via `psql` to execute transactional migrations.
* **Tamper-Evident SHA-256 Checksums**: Every applied migration records its cryptographic hash in the `_yav_migrations` journal table. If past migration files are altered or corrupted, `yav db status` flags the discrepancy immediately.
* **Up / Down Delimiters**: Each generated `.sql` file contains `-- Up` and `-- Down` sections for clean, reversible schema changes.

---

### 12. `yav codegen`
Generates client-side TypeScript interfaces from Rust models/DTOs or exports OpenAPI 3.1 specifications:

```bash
# Generate TypeScript definitions for frontend clients (types.d.ts by default)
yav codegen --ts
yav codegen --ts --output frontend/src/types.ts

# Generate OpenAPI 3.1 specification (openapi.json by default)
yav codegen --openapi
yav codegen --openapi --output docs/api.json
```
* **Full Route & Server Action Discovery**: Automatically discovers all file-based routes (`src/routes/`) as well as all RPC Server Actions decorated with `#[server]` across `src/`, generating typed endpoint unions (`YavServerEndpoints`) in TypeScript and `POST` operations with schemas in `openapi.json`.
* **Type-Safety across the Wire**: Ensures client-side forms and API fetchers stay in sync with Rust structs and enum types.
* **OpenAPI 3.1 Ecosystem**: Export specifications directly to Postman, Insomnia, or client SDK generators.

---

### 13. `yav analyze`
Audits your Yav application performance, bundle sizes, static assets, and readiness scorecard:

```bash
yav analyze
```
* **Route & Component Inventory**: Counts pages, dynamic parameters, and custom components.
* **Asset & CSS Inspection**: Analyzes stylesheet size and flags unoptimized large images (> 250 KB PNG/JPG vs WebP/AVIF).
* **Readiness Scorecard**: Verifies ETag caching, Motion runtime status, sitemap/robots, and PWA setup.

---

### 14. `yav pwa`
Scaffolds a complete Progressive Web App (PWA) setup with installable app manifests, offline caching, and fallback pages:

```bash
# Generate manifest.json, sw.js (Service Worker), and offline.html
yav pwa

# Overwrite existing PWA assets
yav pwa --force
```
* **Offline-Ready Service Worker**: Automatically caches static assets (cache-first) and provides an offline fallback page for document navigations (network-first).
* **Web App Manifest**: Configures app name, icons (192px and 512px SVG), splash background color, and standalone display.

---

### 15. `yav test`
Runs the test suite across the workspace or a specific package, with full argument passthrough to `cargo test`:

```bash
# Run all tests in the workspace
yav test

# Run tests matching a specific name filter
yav test my_test_name

# Target a specific crate / package
yav test -p yav-core
yav test -p yav-server

# Pass flags directly to the test binary (e.g. show stdout output, run ignored tests)
yav test -- --nocapture
yav test -- --ignored

# Combine filters and test binary flags
yav test -p yav-core my_test -- --nocapture
```
* **Seamless Argument Forwarding**: All trailing arguments and options are forwarded directly to `cargo test`, supporting package selectors (`-p`), test name filters, lib/bin/test targets (`--lib`, `--test <name>`), and runtime flags (`-- --nocapture`).
* **Visual Status Banner**: Prints formatted summary headers and clear success/failure indicators.

---

### 16. `yav run <script>`
Executes a custom script defined in the `[scripts]` section of `package.yavml` (similar to `npm run <script>`):

```bash
yav run dev
yav run lint
```

---

### 17. `yav deploy <target>`
Scaffolds production deployment configurations for cloud platforms and Linux servers:

```bash
yav deploy fly      # Generates fly.toml for Fly.io
yav deploy shuttle  # Generates Shuttle.toml for Shuttle.rs
yav deploy railway  # Generates railway.json for Railway.app
yav deploy systemd  # Generates hardened <app>.service unit for Linux Systemd
```

---

### 18. `yav ext`
Installs or checks the status of the official Yav syntax highlighting and snippets extension across VS Code, Cursor, VS Code Insiders, and VSCodium:

```bash
yav ext install
yav ext status
```

---

### 19. `yav doctor`
Diagnoses your local environment, Rust toolchains, project health, and runtime dependencies:
- **Rust Toolchain**: Verifies `rustc` and `cargo` versions in `PATH`.
- **Manifest Health**: Validates `package.yavml` structure, package name, version, and server bindings.
- **Standalone Tailwind CSS**: Checks whether the cached standalone binary is present or ready for download.
- **Routes Directory**: Audits `src/routes/` for root `index.rs` or `page.rs`, layouts, and missing route endpoints.
- **Port Availability**: Checks if the configured HTTP port (e.g. `3000`) is free or currently bound by another process.
- **Git Repository**: Validates git tracking status and clean working tree.

```bash
yav doctor
```

---

### 20. `yav routes`
Inspects all file-based routes discovered under `src/routes/` and displays a formatted table in the terminal:
- **HTTP Methods**: Automatically detects `GET`, `POST`, `PUT`, `DELETE` handlers defined in each route file.
- **URL Pattern**: Displays the final URL route pattern, including dynamic segments (`/users/:id`, `/docs/*slug`).
- **Source File**: Links directly to the underlying file path (`src/routes/users/_id_.rs`).
- **Active Layouts & Middlewares**: Lists the hierarchical chain of `_layout.rs` and `_middleware.rs` files wrapping each route.

```bash
yav routes
```

---

### 21. `yav export`
Performs **Static Site Generation (SSG)**, crawling and rendering all static routes into standalone `.html` files in `dist/` (or a custom directory):
- **Pre-renders and minifies all static pages**: Generates directory-structured `index.html` files (e.g. `dist/about/index.html` for `/about`) with automated whitespace and comment minification via `yav_build::minify_html`.
- **Asset Bundling & Minification**: Copies and minifies all CSS and HTML static assets from `public/` into `dist/`.
- **Canonical URL Inlining**: Embeds the specified `--base-url` into `<link rel="canonical">` and Open Graph tags.
- **Sitemap & Robots Generation**: Automatically creates `sitemap.xml` and `robots.txt` in the export root.
- **Zero-Server Hosting**: Output is immediately ready for deployment on GitHub Pages, Cloudflare Pages, Netlify, Vercel, or AWS S3.

```bash
# Export to dist/ with default base URL
yav export

# Custom output directory and production base URL
yav export --output build --base-url https://mon-site.fr
```
