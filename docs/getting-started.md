# Getting Started with Yav

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.75+)
- No Node.js required

## Installation

```bash
cargo install --git https://github.com/isaacvay/yav.rs yav-cli
```

## Create a New Project

```bash
yav new my-app
cd my-app
```

This generates a ready-to-run project:

```
my-app/
├── Cargo.toml          # Rust dependencies
├── package.yavml       # Yav central manifest (package, server, tailwind, scripts)
├── src/
│   ├── main.rs         # Server entry point
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── index.rs    # Home page (/)
│   │   └── about.rs    # About page (/about)
│   ├── components/
│   │   └── mod.rs
│   └── styles/
│       └── app.css     # Tailwind input CSS
├── public/             # Static assets
└── README.md
```

## Start Development

```bash
yav dev
```

This will:
1. Compile your Rust code
2. Process Tailwind CSS
3. Start the server at `http://localhost:3000`
4. Watch for file changes
5. Automatically reload on changes

## Write Your First Component

Edit `src/routes/index.rs`:

```rust
use yav::prelude::*;

pub fn index_page() -> Node {
    rsx! {
        <main class="min-h-screen bg-white p-8">
            <div class="mx-auto max-w-3xl">
                <h1 class="text-4xl font-bold text-gray-900">
                    "Welcome to Yav"
                </h1>
                <p class="mt-4 text-lg text-gray-600">
                    "Build modern web apps with Rust."
                </p>
                <a href="/about" class="mt-6 inline-block rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700">
                    "Learn More"
                </a>
            </div>
        </main>
    }
}
```

## Build for Production

```bash
yav build
```

This compiles in release mode, minifies CSS, and prepares assets for deployment.

## Project Manifest (`package.yavml`)

`package.yavml` is the single source of truth for your application (comparable to `package.json` in React/Next.js or `pubspec.yaml` in Flutter):

```toml
[package]
name = "my-app"
version = "0.1.0"
description = "A fast Yav web app"
license = "MIT"

[server]
port = 3000
host = "127.0.0.1"

[tailwind]
enabled = true
version = "3.4.17"
input = "src/styles/app.css"
output = "public/styles.css"

[build]
out_dir = "dist"
public_dir = "public"
minify = true
fingerprint = true

[dependencies]
# Yav plugins and feature extensions

[scripts]
dev = "yav dev"
build = "yav build"
test = "cargo test"

[assets]
favicon = "public/favicon.ico"
```

`YavApp::new()` in `src/main.rs` automatically discovers and applies the configuration from `package.yavml` at startup, so there is no need to repeat host or port numbers in your Rust source code.


## RSX Syntax Quick Reference

| Syntax | Description | Example |
|--------|-------------|---------|
| `<tag>` | HTML element | `<div>...</div>` |
| `<Tag />` | Component (PascalCase) | `<Button />` |
| `class="..."` | String attribute | `class="text-lg"` |
| `attr={expr}` | Expression attribute | `id={my_id}` |
| `disabled` | Boolean attribute | `<input disabled />` |
| `"text"` | Text content | `"Hello, world!"` |
| `{expr}` | Rust expression | `{format!("Hi {}", name)}` |
| `data-x="y"` | Hyphenated attribute | `data-testid="main"` |

## Next Steps

- [Configuration Reference (package.yavml)](configuration.md)
- [Components Guide](components.md)
- [Routing](routing.md)
- [Styling with Tailwind](styling.md)
- [Icons](icons.md)
