# Development Server & Live Reload

The Yav development server (`yav dev`) provides a lightning-fast feedback loop with file watching, incremental Tailwind compilation, and state-preserving live reloading.

## Quick Start

Start the development server from the root of any Yav project:

```bash
yav dev
```

Options:
- `-p, --port <PORT>`: Specify local application port (defaults to `3000` or `[server].port` in `package.yavml`).

## Architecture & How It Works

```
[File Change Detected] (notify watcher)
          │
          ├── .css file ──> Standalone Tailwind CLI ──> SSE CssReload (instant CSS swap)
          │
          ├── public/ asset ──────────────────────────> SSE Reload (page refresh)
          │
          └── .rs / .yavml / Cargo.toml ──────────────> Cargo Recompile
                                                               │
                                         ┌─────────────────────┴────────────────────┐
                                         ▼                                          ▼
                                   [Build Success]                           [Build Failure]
                                         │                                          │
                             Kill & restart child app                    Broadcast Error event
                                         │                                          │
                               SSE Reload (DOM Morph)                     Display dark-mode overlay
```

### 1. Multi-Stage Change Classification

The development server automatically classifies changes:
- **CSS-only changes**: Triggers the standalone Tailwind CLI and dispatches a lightweight `CssReload` event to swap stylesheet links in the DOM without losing JavaScript state or scroll position.
- **Static assets**: Reloads the browser when files in `public/` are added or changed.
- **Rust code changes**: Automatically triggers `cargo run`, tracks compilation progress, kills previous processes cleanly (including Windows process tree termination via `taskkill /PID {pid} /T /F`), and triggers DOM morphing.

### 2. State-Preserving DOM Morphing

When a page reload event fires, the client script fetches the newly rendered HTML from the server and performs a structural **DOM morphing** pass (`morphElement`):
- Text and element attributes are diffed and updated in-place.
- Form inputs and textareas that are currently focused or filled retain their user values.
- Checkboxes and radio buttons retain their checked state.

### 3. Dark-Mode Compiler Error Overlay

If your Rust code fails to compile, Yav **never masquerades an error as a success** and never drops you on a broken white screen.
Instead:
- The exact rustc error (file name, line number, and compiler diagnostic) is captured.
- An interactive, accessible dark-mode error modal (`#yav-error-overlay`) is pushed to all connected browsers.
- Once you fix the code in your editor and save, the error overlay vanishes automatically and the page updates.
