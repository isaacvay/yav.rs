# Styling

## CSS Support

Yav supports standard CSS alongside Tailwind CSS. CSS remains standard CSS —
no proprietary syntax.

## Tailwind CSS Integration

Yav integrates the official Tailwind CSS standalone CLI, requiring no Node.js.

### Setup

Tailwind is enabled by default in new projects. The configuration in `package.yavml`:

```toml
[tailwind]
enabled = true
version = "3.4.17"
input = "src/styles/app.css"
output = "public/styles.css"
```

*(Note: legacy `package.yav` and `yav.toml` files remain supported for backwards compatibility).*

### How It Works

1. `yav dev` or `yav build` downloads the Tailwind CLI (once, cached globally)
2. Yav processes your `src/styles/app.css` against your `.rs`, `.yavml`, `.yav`, `.rsx`, and `.html` files
3. Output CSS is written to `public/styles.css` and served at `/public/styles.css` (and fingerprinted in production via `dist/asset-manifest.json`)

### Input CSS

Your `src/styles/app.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

/* Your custom CSS here */
.custom-button {
    transition: all 0.2s ease;
}
```

### Using Tailwind in Components

```rust
rsx! {
    <div class="min-h-screen bg-gray-100 dark:bg-gray-900">
        <h1 class="text-4xl font-bold text-gray-900 dark:text-white">
            "Hello"
        </h1>
        <button class="rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700">
            "Click me"
        </button>
    </div>
}
```

### Custom CSS

You can use plain CSS alongside or instead of Tailwind:

```rust
rsx! {
    <button class="custom-button">"Save changes"</button>
}
```

### Dark Mode & Anti-FOUC

Yav provides built-in dark mode support designed to work seamlessly with Tailwind's `class` strategy (e.g. `dark:bg-gray-900`).

To prevent the dreaded Flash of Unstyled Content (FOUC) when loading stored preferences, inject `anti_fouc_script()` in your document head or root layout:

```rust
use yav::prelude::*;

rsx! {
    <head>
        {anti_fouc_script()}
        <link rel="stylesheet" href="/public/styles.css" />
    </head>
}
```

Add the zero-dependency `<ThemeToggle />` component anywhere in your header or navbar:

```rust
rsx! {
    <header class="flex items-center justify-between p-4">
        <h1 class="text-xl font-bold">"My App"</h1>
        <ThemeToggle class="p-2 border rounded-md" />
    </header>
}
```

---

## Multi-Theme System (`yav-ui::theme`)

In addition to simple light/dark toggling, Yav v0.1.4 features a comprehensive design system with **8 pre-built themes**:

| Theme Slug | Name | Accent Colors | Atmosphere |
|------------|------|---------------|------------|
| `editorial-blueprint` | **Editorial Blueprint** | Rust Oxide `#e0592f`, Warm Paper `#ece9e3` | Yav flagship editorial technical paper |
| `obsidian-minimal` | **Obsidian Minimal** | Electric Indigo `#6366f1`, Deep Black `#09090b` | Sleek modern developer dashboard |
| `nordic-paper` | **Nordic Paper** | Terracotta `#c85a32`, Alabaster `#f7f5f0` | Scandinavian warm minimalist publication |
| `cyber-terminal` | **Cyber Terminal** | Matrix Green `#00ff66`, Phosphor `#ffb000` | Retro-futuristic CRT hacker console |
| `solarized-lab` | **Solarized Lab** | Cyan `#2aa198`, Teal `#002b36` | Scientific notebook & precision research |
| `glass-aurora` | **Glass Aurora** | Frosted Violet `#8b5cf6`, Midnight `#070913` | Translucent blurred glassmorphism |
| `light` | **Classic Light** | Slate `#0f172a`, Blue `#2563eb` | Standard clean daytime interface |
| `dark` | **Slate Dark** | Zinc `#f4f4f5`, Sky `#0284c7` | Standard comfortable night interface |

### Using Themes in Your Application

1. **Inject Theme CSS**: In your root layout or document `<head>`, inject all theme CSS variables:
```rust
use yav::ui::Theme;

rsx! {
    <head>
        {Theme::inject_all_themes_css()}
        <link rel="stylesheet" href="/public/styles.css" />
    </head>
}
```

2. **Add the Theme Selector**:
```rust
use yav::ui::ThemeSelector;

rsx! {
    <ThemeSelector default_theme="editorial-blueprint" class="theme-dropdown" />
}
```
* **Persistence**: Automatically persists user selection in `localStorage` and `yav_theme` cookies.
* **Zero-FOUC**: Injects an inline anti-FOUC script that sets `document.documentElement.dataset.theme` before first paint.

### Theme CSS Variables
Each theme configures semantic CSS custom properties:
- `--bg` — Canvas background color
- `--fg` — Primary typography color
- `--muted` — Secondary text and descriptions
- `--card` — Surface and card background
- `--border` — Border and hairline dividers
- `--accent` — Theme primary accent
- `--oxide` — Signature rust-orange accent
- `--font-mono` / `--font-sans` — Curated typographic pairings

---

## Class Directives in RSX (`class:<name>={bool}`)

Yav v0.1.4 supports ergonomic class directives to conditionally toggle CSS classes:

```rust
let is_open = true;
let is_loading = false;

rsx! {
    <button
        class="base-button px-4 py-2"
        class:is-open={is_open}
        class:opacity-50={is_loading}
        class:border-oxide=true
    >
        "Bouton"
    </button>
}
```
Directives are evaluated at runtime with zero string allocation overhead when false.

---

## Inline `<style>` and Pseudo-Classes in RSX

In Yav v0.1.4, `<style>` tags in `rsx!` support dynamic Rust variables and correctly preserve CSS pseudo-classes (like `:hover`, `:focus`, `:root[data-theme="..."]`) without inserting unwanted whitespace:

```rust
let brand_color = "#e0592f";

rsx! {
    <div>
        <style>
            ".btn:hover { background-color: " {brand_color} "; transform: translateY(-2px); }"
            ":root[data-theme=\"custom\"] { --oxide: #ff7700; }"
        </style>
        <button class="btn">"Hover me"</button>
    </div>
}
```

### Instant CSS Hot-Swap (HMR)

During `yav dev`, when any CSS or Tailwind class changes in your source files:
- The Tailwind CLI recompiles in milliseconds.
- The `yav` dev server sends a targeted `data: css:<path>` event over SSE.
- The browser reload runtime swaps `<link rel="stylesheet">` nodes in-place immediately, preserving your current UI scroll position and state without a full page refresh!

### Production Optimization

`yav build` runs Tailwind with `--minify`, removing unused classes and
minimizing the output CSS.

## Asset Pipeline

### Static Files

Place files in the `public/` directory. They're served at the root path:

```
public/
├── favicon.ico      → /favicon.ico
├── robots.txt       → /robots.txt
└── images/
    └── logo.png     → /images/logo.png
```

### CSS in Document Shell

CSS files are linked in the document head automatically:

```html
<link rel="stylesheet" href="/public/styles.css" />
```
