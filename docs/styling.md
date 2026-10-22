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
