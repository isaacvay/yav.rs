# Yav CLI Documentation

The `yav` CLI is the developer toolbelt for creating, developing, scaffolding, and deploying Yav web applications.

---

## Commands Reference

### 1. `yav new <name>`
Scaffolds a new Yav web project with Tailwind CSS, file-based routes, and recommended configuration.

```bash
yav new mon-application
cd mon-application
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
- Applies asset fingerprinting (`main.[hash].css`).
- Generates `dist/manifest.json`.
- Compiles the optimized Rust release binary.

```bash
yav build
```

---

### 5. `yav check`
Verifies project configuration (`package.yavml`, legacy `package.yav`, or `yav.toml`), ensures Tailwind assets exist, and performs a fast `cargo check`.

```bash
yav check
```

---

### 6. `yav generate` (alias: `yav g`)
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

### 7. `yav icon`
Explores, searches, and inspects the built-in 80+ Lucide SVG icons catalog:

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

### 8. `yav docker`
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

### 9. `yav sitemap`
Scans your `src/routes/` directory to generate standard `public/sitemap.xml` and `public/robots.txt` files:

```bash
# Generate static search engine discovery files
yav sitemap --base-url https://example.com
```

### 10. `yav db`
Manages database initialization and timestamped SQL migration files:

```bash
# Initialize migrations/ directory and .env configuration
yav db init

# Create a new timestamped migration
yav db new add_user_profiles

# List existing migrations
yav db status
```

---

### 11. `yav codegen`
Generates client-side TypeScript interfaces from Rust models/DTOs or exports OpenAPI 3.1 specifications:

```bash
# Generate TypeScript definitions for frontend clients (src/types.d.ts by default)
yav codegen --ts
yav codegen --ts --output frontend/src/types.ts

# Generate OpenAPI 3.1 specification (openapi.json by default)
yav codegen --openapi
yav codegen --openapi --output docs/api.json
```
* **Type-Safety across the Wire**: Ensures client-side forms and API fetchers stay in sync with Rust structs and enum types.
* **OpenAPI 3.1 Ecosystem**: Export specifications directly to Postman, Insomnia, or client SDK generators.

---

### 12. `yav analyze`
Audits your Yav application performance, bundle sizes, static assets, and readiness scorecard:

```bash
yav analyze
```
* **Route & Component Inventory**: Counts pages, dynamic parameters, and custom components.
* **Asset & CSS Inspection**: Analyzes stylesheet size and flags unoptimized large images (> 250 KB PNG/JPG vs WebP/AVIF).
* **Readiness Scorecard**: Verifies ETag caching, Motion runtime status, sitemap/robots, and PWA setup.

---

### 13. `yav pwa`
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

### 14. `yav test`
Runs the workspace test suite across all modules.

```bash
yav test
```
