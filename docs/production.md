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
3. **Asset Fingerprinting & Pre-Compression**: Copies files from `public/` into the output directory (`dist/`), computes Blake3/SHA-256 hashes, and generates pre-compressed `.br` (Brotli) and `.gz` (Gzip) variants.
4. **Release Compilation**: Runs `cargo build --release` with optimization settings.
5. **Manifest Generation**: Writes `dist/manifest.json` mapping logical paths to fingerprinted filenames for long-term browser caching.

## Deployment Targets

Yav provides official scaffolders for popular deployment platforms via `yav deploy`:

```bash
# Deploy to Fly.io (generates fly.toml and multi-stage Dockerfile)
yav deploy fly

# Deploy to Shuttle.rs (generates shuttle configuration)
yav deploy shuttle

# Deploy to Railway (generates railway.toml and Procfile)
yav deploy railway

# Deploy to Linux Systemd (generates /etc/systemd/system unit file)
yav deploy systemd
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
