//! Project scaffolding for `yav new <name>`.
//!
//! Generates a ready-to-run Yav web application template with routing,
//! Tailwind CSS styles, components, and configuration.

use std::path::Path;

/// Validates that a project name consists of valid identifier characters.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    if let Some(first) = name.chars().next() {
        if first.is_ascii_digit() {
            return Err("Project name cannot start with a digit".to_string());
        }
    }

    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return Err(format!(
                "Project name contains invalid character '{c}'. Only alphanumeric characters, dashes, and underscores are allowed."
            ));
        }
    }

    Ok(())
}

/// Scaffolds a new project directory with all boilerplate files.
pub fn scaffold_project(target_dir: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create directory structure
    std::fs::create_dir_all(target_dir.join("src/layouts"))?;
    std::fs::create_dir_all(target_dir.join("src/routes"))?;
    std::fs::create_dir_all(target_dir.join("src/components"))?;
    std::fs::create_dir_all(target_dir.join("src/styles"))?;
    std::fs::create_dir_all(target_dir.join("public"))?;

    // 2. Generate Cargo.toml
    let yav_dep = if target_dir.join("../crates/yav").exists() {
        r#"yav = { path = "../crates/yav" }"#.to_string()
    } else if target_dir.join("../yav.rs/crates/yav").exists() {
        r#"yav = { path = "../yav.rs/crates/yav" }"#.to_string()
    } else if let Ok(path) = std::env::var("YAV_CRATE_PATH") {
        format!(r#"yav = {{ path = "{}" }}"#, path.replace('\\', "/"))
    } else {
        format!(r#"yav = "{}""#, env!("CARGO_PKG_VERSION"))
    };

    // If scaffolded inside an existing Cargo workspace, declare an empty [workspace]
    // table to isolate the new project from parent workspace collisions.
    let is_inside_workspace = {
        let mut has_parent_ws = false;
        let mut curr = target_dir.parent();
        while let Some(p) = curr {
            let manifest = p.join("Cargo.toml");
            if manifest.exists() {
                if let Ok(content) = std::fs::read_to_string(&manifest) {
                    if content.contains("[workspace]") {
                        has_parent_ws = true;
                        break;
                    }
                }
            }
            curr = p.parent();
        }
        has_parent_ws
    };

    let workspace_section = if is_inside_workspace {
        "\n[workspace]\n"
    } else {
        ""
    };

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
{workspace_section}
[dependencies]
{yav_dep}
axum = "0.8"
tokio = {{ version = "1", features = ["full"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
"#
    );
    std::fs::write(target_dir.join("Cargo.toml"), cargo_toml)?;

    // 3. Generate package.yavml (central project manifest)
    let package_yavml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
description = "A modern web application built with Yav"
authors = []
license = "MIT"

[server]
port = 3000
host = "127.0.0.1"

[tailwind]
enabled = true
version = "3.4.17"
input = "src/styles/app.css"
output = "public/styles.css"

[motion]
enabled = true
reduced_motion = "user"

[build]
out_dir = "dist"
public_dir = "public"
minify = true
fingerprint = true

[dependencies]

[scripts]
dev = "yav dev"
build = "yav build"
test = "cargo test"

[assets]
favicon = "public/favicon.svg"
icon = "public/yav-document.png"
"#
    );
    std::fs::write(target_dir.join("package.yavml"), package_yavml)?;

    // 4. Generate src/main.rs (YavApp automatically discovers package.yavml)
    let main_rs = r#"mod components;
mod layouts;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // YavApp automatically loads [server] configuration from package.yavml
    // and auto_routes! discovers and registers all routes under src/routes
    let mut app = yav::server::YavApp::new().static_dir("public");
    if let Some(port) = std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
        app = app.port(port);
    }
    let app = yav::auto_routes!(app);

    println!("  ➜  Local:   http://{}:{}", app.host, app.port);
    app.serve().await.unwrap();
}
"#;
    std::fs::write(target_dir.join("src/main.rs"), main_rs)?;

    // 5. Generate src/layouts/mod.rs and src/layouts/app_layout.rs
    let layouts_mod = r#"pub mod app_layout;

pub use app_layout::*;
"#;
    std::fs::write(target_dir.join("src/layouts/mod.rs"), layouts_mod)?;

    let app_layout_rs = r##"//! Minimal Editorial Blueprint application shell.

use yav::prelude::*;

#[derive(Clone, Default)]
pub struct AppLayoutProps {
    pub children: Vec<Node>,
}

#[component]
pub fn AppLayout(props: AppLayoutProps) -> Node {
    rsx! {
        <div class="page-frame min-h-screen flex flex-col justify-between relative overflow-hidden">
            <Head>
                <meta name="theme-color" content="#0b0b0c" />
                <link rel="icon" type="image/svg+xml" href="/public/favicon.svg" />
                <link rel="alternate icon" type="image/x-icon" href="/favicon.ico" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
                <link href="https://fonts.googleapis.com/css2?family=Geist:wght@300..800&family=Geist+Mono:wght@400..700&family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet" />
                <link rel="stylesheet" href="/public/styles.css" />
            </Head>

            <div class="bp-grid" aria-hidden="true"></div>
            <div class="rails" aria-hidden="true"></div>

            <main class="flex-1 flex flex-col items-center justify-center px-6 py-16 relative z-10">
                {props.children}
            </main>

            <footer class="border-t border-rule py-5 px-6 relative z-10 bg-ink/80 backdrop-blur-sm">
                <div class="wrap flex flex-wrap items-center justify-center gap-x-8 gap-y-2 text-xs font-mono text-mute">
                    <a href="/" class="hover:text-paper transition-colors inline-flex items-center gap-1.5">
                        <span class="text-oxide">"01"</span>
                        <span>"Home"</span>
                    </a>
                    <a href="/about" class="hover:text-paper transition-colors inline-flex items-center gap-1.5">
                        <span class="text-oxide">"02"</span>
                        <span>"Architecture"</span>
                    </a>
                    <a href="/health" class="hover:text-paper transition-colors inline-flex items-center gap-1.5">
                        <span class="dot dot-ok"></span>
                        <span>"Health JSON"</span>
                    </a>
                    <a
                        href="https://github.com/isaacvay/yav.rs"
                        target="_blank"
                        rel="noreferrer"
                        class="hover:text-paper transition-colors inline-flex items-center gap-1"
                    >
                        <span>"Go to yav.rs"</span>
                        <span class="text-dim">"→"</span>
                    </a>
                </div>
            </footer>
        </div>
    }
}
"##;
    std::fs::write(target_dir.join("src/layouts/app_layout.rs"), app_layout_rs)?;

    // 6. Generate src/routes/mod.rs
    let routes_mod = r#"pub mod about;
pub mod health;
pub mod index;
pub mod not_found;
"#;
    std::fs::write(target_dir.join("src/routes/mod.rs"), routes_mod)?;

    // 7. Generate src/routes/index.rs (Minimal Next.js-style starter in Editorial Blueprint)
    let index_rs = r##"use yav::prelude::*;
use axum::extract::Request;
use crate::layouts::*;

pub async fn index(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="w-full max-w-lg space-y-8">
                // Brand Mark
                <div class="flex items-center justify-between">
                    <div class="brand">
                        <img src="/public/favicon.svg" class="w-7 h-7" alt="Yav logo" />
                        <span class="text-lg tracking-tight">"yav.rs"</span>
                        <span class="tag">"__YAV_FRAMEWORK_VERSION__"</span>
                    </div>
                    <span class="chip">
                        <span class="dot dot-ok"></span>
                        <span>"127.0.0.1:3000"</span>
                    </span>
                </div>

                // Steps Card (Editorial Blueprint Plate)
                <div class="plate">
                    <span class="tick tl"></span>
                    <span class="tick tr"></span>
                    <span class="tick bl"></span>
                    <span class="tick br"></span>

                    <ol class="p-6 space-y-3.5 font-mono text-xs sm:text-sm text-paper-2 leading-relaxed">
                        <li class="flex items-start gap-3">
                            <span class="text-oxide select-none">"1."</span>
                            <span>
                                "Get started by editing "
                                <code class="px-1.5 py-0.5 bg-sunk border border-rule text-paper">"src/routes/index.rs"</code>
                                "."
                            </span>
                        </li>
                        <li class="flex items-start gap-3">
                            <span class="text-oxide select-none">"2."</span>
                            <span>
                                "Save and see your changes instantly with "
                                <code class="px-1.5 py-0.5 bg-sunk border border-rule text-oxide">"yav dev"</code>
                                "."
                            </span>
                        </li>
                    </ol>
                </div>

                // Primary Actions
                <div class="flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
                    <a
                        href="https://github.com/isaacvay/yav.rs"
                        target="_blank"
                        rel="noreferrer"
                        class="btn btn-oxide justify-center"
                    >
                        <span>"Read Documentation"</span>
                        <span>"↗"</span>
                    </a>
                    <a href="/about" class="btn btn-line justify-center">
                        <span>"Inspect Architecture"</span>
                        <span>"→"</span>
                    </a>
                </div>
            </div>
        </AppLayout>
    };

    yav::server::HtmlPage::new(page)
        .title("Yav — Rust Web Framework")
        .favicon("/public/favicon.svg")
}
"##;
    // Keep the generated starter page on the framework's real version instead
    // of a hardcoded string that goes stale on every release.
    let index_rs = index_rs.replace(
        "__YAV_FRAMEWORK_VERSION__",
        &format!("v{}", env!("CARGO_PKG_VERSION")),
    );
    std::fs::write(target_dir.join("src/routes/index.rs"), index_rs)?;

    // 8. Generate src/routes/about.rs (Minimal Architecture overview)
    let about_rs = r##"use yav::prelude::*;
use axum::extract::Request;
use crate::layouts::*;

pub async fn about(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="w-full max-w-lg space-y-6">
                <div class="space-y-1.5">
                    <span class="label-oxide">"02 / Architecture"</span>
                    <h1 class="text-2xl font-semibold tracking-tight text-paper">
                        "Single-binary Rust SSR. "
                        <span class="serif text-oxide">"Zero VDOM."</span>
                    </h1>
                </div>

                <div class="plate">
                    <span class="tick tl"></span>
                    <span class="tick tr"></span>
                    <span class="tick bl"></span>
                    <span class="tick br"></span>

                    <div class="divide-y divide-rule font-mono text-xs bg-sunk">
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <span class="text-mute">"HTTP Router"</span>
                            <span class="text-paper">"Axum 0.8 & Tokio"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <span class="text-mute">"Templates"</span>
                            <span class="text-oxide">"Compile-time rsx! {}"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <span class="text-mute">"Styling"</span>
                            <span class="text-paper">"Standalone Tailwind CLI"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <span class="text-mute">"Client JS"</span>
                            <span class="text-ok">"0 KB Hydration"</span>
                        </div>
                    </div>
                </div>

                <div>
                    <a href="/" class="btn btn-line">
                        <span>"← Back to Home"</span>
                    </a>
                </div>
            </div>
        </AppLayout>
    };

    yav::server::HtmlPage::new(page)
        .title("Architecture — Yav")
        .favicon("/public/favicon.svg")
}
"##;
    std::fs::write(target_dir.join("src/routes/about.rs"), about_rs)?;

    // 8b. Generate src/routes/health.rs (production health check endpoint)
    let health_rs = r#"use yav::prelude::*;
use axum::response::Response;

/// Production health check endpoint returning HTTP 200 with JSON status payload.
/// Compatible with container orchestrators (Kubernetes, Docker), uptime monitors, and cloud hosts.
pub async fn health() -> Response {
    json_response(
        StatusCode::OK,
        &HealthStatus {
            status: "ok",
            service: "yav",
            version: env!("CARGO_PKG_VERSION"),
        },
    )
}
"#;
    std::fs::write(target_dir.join("src/routes/health.rs"), health_rs)?;

    // 8c. Generate src/routes/not_found.rs (Minimal Next.js-style 404 page in Editorial Blueprint)
    let not_found_rs = r##"use yav::prelude::*;
use axum::extract::Request;
use crate::layouts::*;

/// Custom 404 Not Found fallback page.
pub async fn not_found(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="w-full max-w-md text-center space-y-6">
                <div class="inline-flex items-center gap-4 font-mono">
                    <span class="text-2xl font-bold text-oxide pr-4 border-r border-rule">"404"</span>
                    <span class="text-sm text-paper-2">"This page could not be found."</span>
                </div>

                <div>
                    <a href="/" class="btn btn-line">
                        <span>"← Back to Home"</span>
                    </a>
                </div>
            </div>
        </AppLayout>
    };

    yav::server::HtmlPage::new(page)
        .status(axum::http::StatusCode::NOT_FOUND)
        .title("404 — Page Not Found | Yav")
        .favicon("/public/favicon.svg")
}
"##;
    std::fs::write(target_dir.join("src/routes/not_found.rs"), not_found_rs)?;

    // 9. Generate src/components/mod.rs
    let components_mod = r#"//! Reusable UI components.
"#;
    std::fs::write(target_dir.join("src/components/mod.rs"), components_mod)?;

    // 10. Generate src/styles/app.css (Editorial Blueprint Design System)
    let app_css = r#"@import url('https://fonts.googleapis.com/css2?family=Geist:wght@300..800&family=Geist+Mono:wght@400..700&family=Instrument+Serif:ital@0;1&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  color-scheme: dark;
  --ink: #0b0b0c;
  --ink-2: #111112;
  --ink-3: #171718;
  --ink-sunk: #080809;
  --paper: #ece9e3;
  --paper-2: #c9c5bd;
  --mute: #8d8982;
  --dim: #5d5a55;
  --faint: #3a3936;
  --rule: rgba(236, 233, 227, 0.08);
  --rule-2: rgba(236, 233, 227, 0.14);
  --rule-3: rgba(236, 233, 227, 0.24);
  --oxide: #e0592f;
  --oxide-hi: #f07448;
  --oxide-soft: rgba(224, 89, 47, 0.1);
  --ok: #9cc69b;
  --bad: #d77a6a;
  --sans: 'Geist', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --mono: 'Geist Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
  --serif: 'Instrument Serif', Georgia, serif;
  --max: 1280px;
  --pad: clamp(1.25rem, 3.5vw, 3rem);
}

html {
  background-color: var(--ink);
  color: var(--paper);
  font-family: var(--sans);
  scroll-behavior: smooth;
}

body {
  background-color: var(--ink);
  color: var(--paper);
  font-family: var(--sans);
  overflow-x: hidden;
  selection-background-color: var(--oxide);
}

::selection {
  background: var(--oxide);
  color: #fff;
}

.wrap {
  max-width: var(--max);
  margin-left: auto;
  margin-right: auto;
  padding-left: var(--pad);
  padding-right: var(--pad);
}

.rails {
  pointer-events: none;
  position: fixed;
  inset: 0;
  z-index: 5;
  max-width: var(--max);
  margin: 0 auto;
  border-left: 1px solid var(--rule);
  border-right: 1px solid var(--rule);
}

.site-header {
  position: sticky;
  top: 0;
  z-index: 40;
  background: rgba(11, 11, 12, 0.84);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--rule);
}

.brand {
  display: inline-flex;
  align-items: center;
  gap: 0.625rem;
  font-family: var(--mono);
  font-weight: 600;
  font-size: 0.9375rem;
  color: var(--paper);
}

.brand .tag {
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--mute);
  padding: 0.1rem 0.4rem;
  border: 1px solid var(--rule-2);
  background: var(--ink-2);
}

.nav-link {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0 0.75rem;
  height: 100%;
  font-size: 0.8125rem;
  color: var(--mute);
  border-bottom: 2px solid transparent;
  transition: color 0.15s ease, border-color 0.15s ease;
}

.nav-link .num {
  font-family: var(--mono);
  font-size: 0.6875rem;
  color: var(--dim);
}

.nav-link:hover,
.nav-link[aria-current="page"] {
  color: var(--paper);
}

.nav-link[aria-current="page"] {
  border-bottom-color: var(--oxide);
}

.nav-link[aria-current="page"] .num {
  color: var(--oxide);
}

.display {
  font-family: var(--sans);
  font-weight: 600;
  font-size: clamp(2.5rem, 5.4vw, 4.5rem);
  line-height: 0.98;
  letter-spacing: -0.035em;
  color: var(--paper);
}

.serif {
  font-family: var(--serif);
  font-style: italic;
  font-weight: 400;
}

.lede {
  font-size: clamp(1rem, 1.2vw, 1.125rem);
  line-height: 1.65;
  color: var(--paper-2);
}

.label {
  font-family: var(--mono);
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--mute);
}

.label-oxide {
  font-family: var(--mono);
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--oxide);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1.125rem;
  font-family: var(--mono);
  font-size: 0.75rem;
  font-weight: 500;
  border: 1px solid transparent;
  transition: all 0.15s ease;
  cursor: pointer;
}

.btn-oxide {
  background: var(--oxide);
  color: #fff;
  border-color: var(--oxide);
}

.btn-oxide:hover {
  background: var(--oxide-hi);
}

.btn-line {
  background: var(--ink-2);
  color: var(--paper);
  border-color: var(--rule-2);
}

.btn-line:hover {
  border-color: var(--oxide);
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.25rem 0.625rem;
  font-family: var(--mono);
  font-size: 0.6875rem;
  color: var(--paper-2);
  background: var(--ink-2);
  border: 1px solid var(--rule-2);
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 9999px;
}

.dot-ok {
  background: var(--ok);
  box-shadow: 0 0 8px var(--ok);
}

.dot-oxide {
  background: var(--oxide);
}

.bp-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(to right, rgba(236, 233, 227, 0.035) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(236, 233, 227, 0.035) 1px, transparent 1px);
  background-size: 48px 48px;
  pointer-events: none;
}

.hero-glow {
  position: absolute;
  top: -120px;
  right: 10%;
  width: 520px;
  height: 520px;
  background: radial-gradient(circle, rgba(224, 89, 47, 0.11) 0%, transparent 70%);
  pointer-events: none;
}

.plate {
  position: relative;
  background: var(--ink-2);
  border: 1px solid var(--rule-2);
}

.plate-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.65rem 1rem;
  border-bottom: 1px solid var(--rule);
  background: var(--ink-3);
  font-family: var(--mono);
  font-size: 0.6875rem;
  color: var(--mute);
}

.tick {
  position: absolute;
  width: 6px;
  height: 6px;
  border-color: var(--oxide);
  pointer-events: none;
}

.tick.tl { top: -1px; left: -1px; border-top: 1.5px solid var(--oxide); border-left: 1.5px solid var(--oxide); }
.tick.tr { top: -1px; right: -1px; border-top: 1.5px solid var(--oxide); border-right: 1.5px solid var(--oxide); }
.tick.bl { bottom: -1px; left: -1px; border-bottom: 1.5px solid var(--oxide); border-left: 1.5px solid var(--oxide); }
.tick.br { bottom: -1px; right: -1px; border-bottom: 1.5px solid var(--oxide); border-right: 1.5px solid var(--oxide); }

.sec-head {
  display: flex;
  align-items: center;
  gap: 0.875rem;
  margin-bottom: 2rem;
  font-family: var(--mono);
}

.sec-head .n {
  font-size: 0.75rem;
  color: var(--oxide);
  font-weight: 600;
}

.sec-head .t {
  font-size: 0.875rem;
  color: var(--paper);
  font-weight: 600;
}

.sec-head .sec-rule {
  flex: 1;
  height: 1px;
  background: var(--rule-2);
}

.sec-head .r {
  font-size: 0.6875rem;
  color: var(--dim);
}

.cells {
  display: grid;
  gap: 1px;
  background: var(--rule-2);
  border: 1px solid var(--rule-2);
}

.cell {
  background: var(--ink);
  padding: 1.5rem;
  position: relative;
}

.term-window {
  background: var(--ink-sunk);
  border: 1px solid var(--rule-2);
}

.term-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.6rem 1rem;
  border-bottom: 1px solid var(--rule);
  background: var(--ink-2);
}

.wordmark {
  font-family: var(--sans);
  font-weight: 800;
  font-size: clamp(4rem, 14vw, 11rem);
  line-height: 0.85;
  letter-spacing: -0.05em;
  color: transparent;
  -webkit-text-stroke: 1px rgba(236, 233, 227, 0.08);
}
"#;
    std::fs::write(target_dir.join("src/styles/app.css"), app_css)?;

    // 11. Generate tailwind.config.js with Editorial Blueprint tokens
    let tailwind_config_js = r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: [
    "./src/**/*.{rs,html,yav,rsx,yavml}",
    "./public/**/*.{html,js}",
  ],
  theme: {
    extend: {
      colors: {
        ink: '#0b0b0c',
        'ink-2': '#111112',
        'ink-3': '#171718',
        sunk: '#080809',
        paper: '#ece9e3',
        'paper-2': '#c9c5bd',
        mute: '#8d8982',
        dim: '#5d5a55',
        faint: '#3a3936',
        oxide: '#e0592f',
        'oxide-hi': '#f07448',
        ok: '#9cc69b',
        bad: '#d77a6a',
        rule: 'rgba(236, 233, 227, 0.08)',
      },
      fontFamily: {
        sans: ['Geist', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
        mono: ['Geist Mono', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
        serif: ['Instrument Serif', 'Georgia', 'serif'],
      },
    },
  },
  plugins: [],
}
"#;
    std::fs::write(target_dir.join("tailwind.config.js"), tailwind_config_js)?;

    // 10. Generate public/robots.txt
    let robots_txt = r#"User-agent: *
Allow: /
"#;
    std::fs::write(target_dir.join("public/robots.txt"), robots_txt)?;

    // 10b. Generate initial public/styles.css fallback with Editorial Blueprint design system
    std::fs::write(target_dir.join("public/styles.css"), app_css)?;

    // 11. Generate public/favicon.ico, public/favicon.svg (gradient), and public/favicon-flat.svg
    #[rustfmt::skip]
    let favicon_ico: &[u8] = &[
        0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
        0x20, 0x00, 0x30, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
        0x28, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0xeb, 0x44, 0x25, 0xff, 0x00, 0x00, 0x00, 0x00,
    ];
    std::fs::write(target_dir.join("public/favicon.ico"), favicon_ico)?;

    let favicon_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Main fiery gradient: luminous yellow to deep orange/coral -->
    <linearGradient id="yav-gold-fire" x1="85%" y1="0%" x2="15%" y2="100%">
      <stop offset="0%" stop-color="#FFDD44" />
      <stop offset="25%" stop-color="#FFA812" />
      <stop offset="65%" stop-color="#FF6B00" />
      <stop offset="90%" stop-color="#FF3800" />
      <stop offset="100%" stop-color="#E01E00" />
    </linearGradient>

    <!-- Top-left wing warm ambient gradient -->
    <linearGradient id="yav-left-wing" x1="10%" y1="0%" x2="90%" y2="100%">
      <stop offset="0%" stop-color="#FFDD55" />
      <stop offset="40%" stop-color="#FFAA15" />
      <stop offset="100%" stop-color="#FF5D00" />
    </linearGradient>

    <!-- Lower wing gradient -->
    <linearGradient id="yav-lower-wing" x1="90%" y1="10%" x2="10%" y2="90%">
      <stop offset="0%" stop-color="#FF9E00" />
      <stop offset="50%" stop-color="#FF5500" />
      <stop offset="100%" stop-color="#DC1414" />
    </linearGradient>

    <!-- Subtle specularity overlay -->
    <linearGradient id="yav-sheen" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.35" />
      <stop offset="35%" stop-color="#FFFFFF" stop-opacity="0.0" />
      <stop offset="100%" stop-color="#000000" stop-opacity="0.25" />
    </linearGradient>

    <!-- Drop shadow for depth on dark or light interfaces -->
    <filter id="yav-depth" x="-10%" y="-10%" width="125%" height="125%">
      <feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#FF4500" flood-opacity="0.3" />
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.25" />
    </filter>
  </defs>

  <g id="yav-monogram-gradient" filter="url(#yav-depth)">
    <!-- Left Wing -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z"
          fill="url(#yav-left-wing)" />
    <!-- Left Wing Sheen -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z"
          fill="url(#yav-sheen)" />

    <!-- Center Main Diagonal Wing -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z"
          fill="url(#yav-gold-fire)" />
    <!-- Center Sheen -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z"
          fill="url(#yav-sheen)" />

    <!-- Lower Right Parallel Wing -->
    <path d="M 454.4 152.6
             C 430.0 151.5 394.0 156.0 374.0 165.5
             L 132.9 461.8
             L 183.5 462.9
             C 194.5 459.5 208.0 452.0 216.7 447.1
             Z"
          fill="url(#yav-lower-wing)" />
  </g>
</svg>
"##;
    std::fs::write(target_dir.join("public/favicon.svg"), favicon_svg)?;

    let favicon_flat_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <!-- Yav Monogram - Flat Icon -->
  <g id="yav-monogram-flat" fill="#F5A623">
    <!-- Left Wing -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z" />
    
    <!-- Center Main Diagonal Wing -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z" />
    
    <!-- Lower Right Parallel Wing -->
    <path d="M 454.4 152.6
             C 430.0 151.5 394.0 156.0 374.0 165.5
             L 132.9 461.8
             L 183.5 462.9
             C 194.5 459.5 208.0 452.0 216.7 447.1
             Z" />
  </g>
</svg>
"##;
    std::fs::write(target_dir.join("public/favicon-flat.svg"), favicon_flat_svg)?;

    // 11c. Generate public/yav-document.svg and public/yav-document.png
    let yav_document_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="doc-card-body" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#464956" />
      <stop offset="60%" stop-color="#383A44" />
      <stop offset="100%" stop-color="#282930" />
    </linearGradient>
    <linearGradient id="ribbon-gold" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FFDE43" />
      <stop offset="100%" stop-color="#F59E0B" />
    </linearGradient>
    <linearGradient id="fold-front" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFF085" />
      <stop offset="50%" stop-color="#FFCD1E" />
      <stop offset="100%" stop-color="#E69500" />
    </linearGradient>
    <linearGradient id="fold-shadow" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#14151B" stop-opacity="0.75" />
      <stop offset="100%" stop-color="#000000" stop-opacity="0.0" />
    </linearGradient>
    <linearGradient id="emblem-gold" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFDD44" />
      <stop offset="50%" stop-color="#F5A623" />
      <stop offset="100%" stop-color="#D97706" />
    </linearGradient>
    <filter id="doc-shadow" x="-15%" y="-15%" width="130%" height="135%">
      <feDropShadow dx="0" dy="18" stdDeviation="22" flood-color="#000000" flood-opacity="0.55" />
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.3" />
    </filter>
    <filter id="emblem-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#000000" flood-opacity="0.4" />
    </filter>
  </defs>

  <g id="yav-document" filter="url(#doc-shadow)">
    <path d="M 140 72 L 320 72 L 396 148 L 396 416 C 396 429.3 385.3 440 372 440 L 140 440 C 126.7 440 116 429.3 116 416 L 116 96 C 116 82.7 126.7 72 140 72 Z" fill="url(#doc-card-body)" />
    <path d="M 140 73 L 320 73" stroke="#FFFFFF" stroke-opacity="0.15" stroke-width="1.5" />
    <path d="M 320 72 L 320 148 L 396 148 Z" fill="url(#fold-shadow)" />
    <path d="M 320 72 L 320 136 C 320 142.6 325.4 148 332 148 L 396 148 Z" fill="url(#fold-front)" />
    <path d="M 148 72 L 192 72 L 192 148 L 170 134 L 148 148 Z" fill="url(#ribbon-gold)" />
    <g id="center-y" filter="url(#emblem-shadow)">
      <path d="M 194 210 L 230 210 L 256 264 L 282 210 L 318 210 L 274 286 L 274 340 L 238 340 L 238 286 Z" fill="url(#emblem-gold)" />
      <path d="M 194 210 L 230 210 L 256 264 L 244 264 Z" fill="#FFFFFF" fill-opacity="0.12" />
    </g>
  </g>
</svg>
"##;
    std::fs::write(target_dir.join("public/yav-document.svg"), yav_document_svg)?;

    let yav_document_png = include_bytes!("../assets/png/yav-document-256.png");
    std::fs::write(target_dir.join("public/yav-document.png"), yav_document_png)?;

    // 11. Generate README.md
    let readme = format!(
        r#"# {name}

A web application built with the [Yav.rs](https://github.com/isaacvay/yav.rs) web framework.

## Getting Started

### Development
Start the development server with live reload:
```bash
yav dev
```

### Production Build
Build optimized assets and binary:
```bash
yav build
```

### Run Tests
```bash
yav test
```
"#
    );
    std::fs::write(target_dir.join("README.md"), readme)?;

    // 12. Generate .gitignore
    let gitignore = r#"/target
/.yav
dist/
**/*.rs.bk
*.log
.DS_Store
"#;
    std::fs::write(target_dir.join(".gitignore"), gitignore)?;

    // 13. Generate .vscode/ configuration for instant syntax highlighting and icons
    std::fs::create_dir_all(target_dir.join(".vscode"))?;
    let vscode_settings = r#"{
  "files.associations": {
    "*.yav": "yav",
    "*.rsx": "yav",
    "*.yavml": "yavml"
  },
  "emmet.includeLanguages": {
    "yav": "html",
    "rsx": "html"
  },
  "material-icon-theme.files.associations": {
    "*.yav": "react",
    "*.rsx": "react_ts",
    "*.yavml": "settings"
  },
  "vsicons.associations.files": [
    {
      "icon": "react",
      "extensions": ["yav", "rsx"],
      "format": "svg"
    },
    {
      "icon": "toml",
      "extensions": ["yavml"],
      "format": "svg"
    }
  ]
}
"#;
    std::fs::write(target_dir.join(".vscode/settings.json"), vscode_settings)?;

    let vscode_extensions = r#"{
  "recommendations": [
    "yav-framework.yav-vscode",
    "rust-lang.rust-analyzer",
    "bradlc.vscode-tailwindcss"
  ]
}
"#;
    std::fs::write(
        target_dir.join(".vscode/extensions.json"),
        vscode_extensions,
    )?;

    Ok(())
}

/// Compile-check a freshly scaffolded project in its own directory.
///
/// Separated from [`run`] so the behaviour can be tested without invoking the
/// interactive editor/Tailwind prompts. `cargo_target_dir`, when set, isolates
/// the build artefacts from the parent build (avoids contending on the shared
/// `target/` lock). Compiler diagnostics are never suppressed.
async fn verify_project_compiles(
    target_dir: &Path,
    cargo_target_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\x1b[34mRunning cargo check on the generated project...\x1b[0m\n");
    let mut command = tokio::process::Command::new("cargo");
    command.arg("check").current_dir(target_dir);
    if let Some(dir) = cargo_target_dir {
        command.env("CARGO_TARGET_DIR", dir);
    }
    let status = command.status().await?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "generated project failed `cargo check` (exit code {}). Fix the errors above \
             or run `yav check` inside '{}'.",
            status.code().unwrap_or(-1),
            target_dir.display()
        )
        .into())
    }
}

/// CLI entry point for the `yav new` command.
///
/// `verify` runs `cargo check` inside the generated project before reporting
/// success. It is opt-in because a fresh scaffold resolves its `yav`
/// dependency from the registry, which may require network access.
pub async fn run(name: &str, verify: bool) -> Result<(), Box<dyn std::error::Error>> {
    validate_project_name(name).map_err(|msg| format!("Invalid project name '{name}': {msg}"))?;

    let target_dir = Path::new(name);
    if target_dir.exists() {
        if target_dir.is_dir() {
            let mut entries = std::fs::read_dir(target_dir)?;
            if entries.next().is_some() {
                return Err(format!(
                    "Target directory '{}' already exists and is not empty",
                    target_dir.display()
                )
                .into());
            }
        } else {
            return Err(format!(
                "Target path '{}' already exists as a file",
                target_dir.display()
            )
            .into());
        }
    }

    println!("\x1b[34mCreating new Yav project:\x1b[0m \x1b[1m{name}\x1b[0m");
    scaffold_project(target_dir, name)?;

    if verify {
        verify_project_compiles(target_dir, None).await?;
        println!("\n\x1b[1;32m[OK] Created and verified project '{name}'\x1b[0m");
    } else {
        // Do not claim the project compiles: it has not been built yet.
        println!("\n\x1b[1;32m[OK] Created project '{name}' (not compiled)\x1b[0m");
    }

    // Check and offer/install Yav editor extension
    crate::ext::prompt_and_install_if_needed();

    // Check and offer/install Tailwind CSS standalone compiler
    let _ = crate::tailwind_setup::prompt_and_install_in_new("3.4.17").await;

    println!("  Next steps:");
    println!("    cd {name}");
    if !verify {
        println!("    yav check   # verify the generated project compiles");
    }
    println!("    yav dev\n");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_project_name() {
        assert!(validate_project_name("my-app").is_ok());
        assert!(validate_project_name("blog_backend").is_ok());
        assert!(validate_project_name("site123").is_ok());

        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("123app").is_err());
        assert!(validate_project_name("my app").is_err());
        assert!(validate_project_name("foo/bar").is_err());
    }

    /// Build a dependency-free crate so `cargo check` needs no registry access.
    fn write_tiny_crate(dir: &Path, main_rs: &str) {
        std::fs::create_dir_all(dir.join("src")).expect("create src dir");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"yav-verify-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write Cargo.toml");
        std::fs::write(dir.join("src/main.rs"), main_rs).expect("write main.rs");
    }

    #[tokio::test]
    async fn test_verify_project_compiles_reports_success_and_failure() {
        let root = std::env::temp_dir().join(format!("yav_verify_probe_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        let good = root.join("good");
        let bad = root.join("bad");
        write_tiny_crate(&good, "fn main() { println!(\"ok\"); }\n");
        write_tiny_crate(&bad, "fn main() { this is not rust }\n");

        let ok_target = root.join("target_ok");
        let bad_target = root.join("target_bad");

        assert!(
            verify_project_compiles(&good, Some(&ok_target))
                .await
                .is_ok(),
            "a valid generated crate must pass cargo check"
        );
        let err = verify_project_compiles(&bad, Some(&bad_target))
            .await
            .expect_err("an invalid generated crate must fail cargo check");
        assert!(
            err.to_string().contains("cargo check"),
            "the error must name the failing command, got: {err}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_scaffold_project() {
        let temp_dir = std::env::temp_dir().join("yav_test_scaffold");
        let _ = std::fs::remove_dir_all(&temp_dir);

        scaffold_project(&temp_dir, "test-site").expect("Scaffolding should succeed");

        assert!(temp_dir.join("Cargo.toml").is_file());
        assert!(temp_dir.join("package.yavml").is_file());
        assert!(temp_dir.join("src/main.rs").is_file());
        assert!(temp_dir.join("src/layouts/mod.rs").is_file());
        assert!(temp_dir.join("src/layouts/app_layout.rs").is_file());
        assert!(temp_dir.join("src/routes/mod.rs").is_file());
        assert!(temp_dir.join("src/routes/index.rs").is_file());
        assert!(temp_dir.join("src/routes/about.rs").is_file());
        assert!(temp_dir.join("src/routes/health.rs").is_file());
        assert!(temp_dir.join("src/routes/not_found.rs").is_file());
        assert!(temp_dir.join("src/components/mod.rs").is_file());
        assert!(temp_dir.join("src/styles/app.css").is_file());
        assert!(temp_dir.join("public/robots.txt").is_file());
        assert!(temp_dir.join("public/favicon.ico").is_file());
        assert!(temp_dir.join("public/favicon.svg").is_file());
        assert!(temp_dir.join("public/favicon-flat.svg").is_file());
        assert!(temp_dir.join("public/styles.css").is_file());
        assert!(temp_dir.join("README.md").is_file());
        assert!(temp_dir.join(".gitignore").is_file());

        let main_content = std::fs::read_to_string(temp_dir.join("src/main.rs")).unwrap();
        assert!(main_content.contains("app.host"));
        assert!(main_content.contains("app.port"));
        assert!(main_content.contains("std::env::var(\"PORT\")"));
        assert!(main_content.contains(".port(port)"));

        let cargo_content = std::fs::read_to_string(temp_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_content.contains("name = \"test-site\""));
        assert!(cargo_content.contains("[profile.release]"));
        assert!(cargo_content.contains("lto = true"));

        let css_content = std::fs::read_to_string(temp_dir.join("src/styles/app.css")).unwrap();
        assert!(css_content.contains("--oxide: #e0592f"));
        assert!(css_content.contains("Instrument Serif"));
        assert!(css_content.contains(".plate"));
        assert!(css_content.contains(".wordmark"));

        let layout_content =
            std::fs::read_to_string(temp_dir.join("src/layouts/app_layout.rs")).unwrap();
        assert!(layout_content.contains("<Head>"));
        assert!(layout_content.contains("bp-grid"));
        assert!(layout_content.contains("rails"));

        let index_content = std::fs::read_to_string(temp_dir.join("src/routes/index.rs")).unwrap();
        assert!(index_content.contains("Get started by editing"));
        assert!(index_content.contains("src/routes/index.rs"));
        assert!(index_content.contains("yav dev"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
