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
    } else {
        format!(r#"yav = "{}""#, env!("CARGO_PKG_VERSION"))
    };

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
{yav_dep}
axum = "0.8"
tokio = {{ version = "1", features = ["full"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
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
    let app = yav::server::YavApp::new().static_dir("public");
    let app = yav::auto_routes!(app);

    println!("🚀 Yav server running at http://{}:{}", app.host, app.port);
    app.serve().await.unwrap();
}
"#;
    std::fs::write(target_dir.join("src/main.rs"), main_rs)?;

    // 5. Generate src/layouts/mod.rs and src/layouts/app_layout.rs
    let layouts_mod = r#"pub mod app_layout;

pub use app_layout::*;
"#;
    std::fs::write(target_dir.join("src/layouts/mod.rs"), layouts_mod)?;

    let app_layout_rs = r##"//! Default application layout with navigation and footer.

use yav::prelude::*;
use yav::icons::{render_icon, IconSize};

#[derive(Clone, Default)]
pub struct AppLayoutProps {
    pub children: Vec<Node>,
}

#[component]
pub fn AppLayout(props: AppLayoutProps) -> Node {
    let anti_fouc = anti_fouc_script();

    rsx! {
        <div class="min-h-screen bg-[#fafafa] text-zinc-900 antialiased selection:bg-amber-500 selection:text-white dark:bg-[#0c0c0e] dark:text-zinc-100 flex flex-col justify-between font-sans">
            {anti_fouc}

            <header class="sticky top-0 z-50 w-full border-b border-zinc-200/80 bg-white/90 backdrop-blur-md dark:border-zinc-800/80 dark:bg-[#0c0c0e]/90 transition-colors">
                <div class="mx-auto flex h-14 max-w-4xl items-center justify-between px-6">
                    <div class="flex items-center gap-6">
                        <a href="/" class="flex items-center gap-2.5 hover:opacity-90 transition-opacity">
                            <img src="/public/favicon.svg" class="h-6 w-6" alt="Yav logo" />
                            <span class="font-bold tracking-tight text-zinc-900 dark:text-white text-base">"yav"</span>
                            <span class="rounded bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-400 px-1.5 py-0.5 text-[11px] font-mono font-medium">"v0.1.0"</span>
                        </a>

                        <nav class="hidden sm:flex items-center gap-5 text-sm text-zinc-600 dark:text-zinc-400 font-medium">
                            <a href="/" class="hover:text-zinc-900 dark:hover:text-white transition-colors flex items-center gap-1.5">
                                {render_icon("home", IconSize::Sm, Some("w-3.5 h-3.5 opacity-70"), None)}
                                <span>"Overview"</span>
                            </a>
                            <a href="/about" class="hover:text-zinc-900 dark:hover:text-white transition-colors flex items-center gap-1.5">
                                {render_icon("layers", IconSize::Sm, Some("w-3.5 h-3.5 opacity-70"), None)}
                                <span>"Architecture"</span>
                            </a>
                        </nav>
                    </div>

                    <div class="flex items-center gap-3">
                        <button
                            onclick="navigator.clipboard.writeText('http://127.0.0.1:3000'); const el=this.querySelector('.copy-txt'); el.innerText='copied!'; setTimeout(()=>el.innerText='127.0.0.1:3000', 1200);"
                            class="hidden sm:inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-zinc-100 hover:bg-zinc-200/80 dark:bg-zinc-800/80 dark:hover:bg-zinc-700/80 text-[11px] font-mono text-zinc-600 dark:text-zinc-300 transition-colors cursor-pointer border border-zinc-200 dark:border-zinc-700/60"
                            title="Click to copy local URL"
                        >
                            <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                            <span class="copy-txt">"127.0.0.1:3000"</span>
                            {render_icon("copy", IconSize::Sm, Some("w-3 h-3 opacity-60 ml-0.5"), None)}
                        </button>

                        <a
                            href="https://github.com/isaacvay/yav.rs"
                            target="_blank"
                            rel="noreferrer"
                            class="inline-flex items-center gap-1.5 text-xs text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-white transition-colors font-mono"
                        >
                            {render_icon("minimal:github", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                            <span>"GitHub"</span>
                            {render_icon("external-link", IconSize::Sm, Some("w-2.5 h-2.5 opacity-60"), None)}
                        </a>
                        <div class="border-l border-zinc-200 dark:border-zinc-800 pl-3">
                            <ThemeToggle />
                        </div>
                    </div>
                </div>
            </header>

            <main class="flex-grow max-w-4xl mx-auto w-full px-6 py-10 sm:py-14">
                {props.children}
            </main>

            <footer class="border-t border-zinc-200 dark:border-zinc-800/80 py-8 text-xs text-zinc-500 dark:text-zinc-400">
                <div class="mx-auto flex max-w-4xl flex-col sm:flex-row items-center justify-between gap-4 px-6">
                    <div class="flex items-center gap-2">
                        <img src="/public/favicon-flat.svg" class="h-3.5 w-3.5 opacity-70" alt="Yav" />
                        <span class="font-medium text-zinc-700 dark:text-zinc-300">"yav.rs"</span>
                        <span>"•"</span>
                        <span>"High-Velocity Rust Web Framework"</span>
                    </div>
                    <div class="flex items-center gap-5 font-mono">
                        <a href="https://github.com/isaacvay/yav.rs" target="_blank" rel="noreferrer" class="hover:underline flex items-center gap-1">
                            {render_icon("minimal:github", IconSize::Sm, Some("w-3 h-3 opacity-70"), None)}
                            <span>"GitHub"</span>
                        </a>
                        <a href="/about" class="hover:underline flex items-center gap-1">
                            {render_icon("layers", IconSize::Sm, Some("w-3 h-3 opacity-70"), None)}
                            <span>"Architecture"</span>
                        </a>
                        <span>"MIT License"</span>
                    </div>
                </div>
            </footer>
        </div>
    }
}
"##;
    std::fs::write(target_dir.join("src/layouts/app_layout.rs"), app_layout_rs)?;

    // 6. Generate src/routes/mod.rs
    let routes_mod = r#"pub mod about;
pub mod index;
"#;
    std::fs::write(target_dir.join("src/routes/mod.rs"), routes_mod)?;

    // 7. Generate src/routes/index.rs
    let index_rs = r##"use yav::prelude::*;
use yav::icons::{render_icon, IconSize};
use axum::extract::Request;
use crate::layouts::*;

pub async fn index(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="space-y-12">
                // 1. Hero Section
                <div class="space-y-4">
                    <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800/60 text-xs font-mono text-emerald-700 dark:text-emerald-400">
                        {render_icon("terminal", IconSize::Sm, Some("w-3.5 h-3.5 text-emerald-500"), None)}
                        <span>"dev server listening on 127.0.0.1:3000"</span>
                    </div>

                    <h1 class="text-3xl sm:text-4xl font-bold tracking-tight text-zinc-900 dark:text-zinc-50">
                        "Your Yav application is ready."
                    </h1>

                    <p class="text-base text-zinc-600 dark:text-zinc-400 max-w-2xl leading-relaxed">
                        "Full-stack Rust with Axum 0.8 and RSX templates. Edit "
                        <code class="px-1.5 py-0.5 rounded bg-zinc-200/70 dark:bg-zinc-800 font-mono text-xs text-zinc-900 dark:text-zinc-100 border border-zinc-300/80 dark:border-zinc-700">
                            "src/routes/index.rs"
                        </code>
                        " to start building your user interface."
                    </p>
                </div>

                // 2. Interactive Workbench (Dual Tab: Live Preview + RSX Source)
                <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-sm overflow-hidden">
                    <div class="px-5 py-3 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50/80 dark:bg-zinc-900/80 flex items-center justify-between flex-wrap gap-2">
                        <div class="flex items-center gap-1.5 p-1 rounded-lg bg-zinc-200/60 dark:bg-zinc-800 text-xs font-medium">
                            <button
                                id="tab-btn-preview"
                                onclick="document.getElementById('tab-preview').classList.remove('hidden'); document.getElementById('tab-code').classList.add('hidden'); this.className='inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-white dark:bg-zinc-900 text-zinc-900 dark:text-white shadow-xs font-medium cursor-pointer transition-all'; document.getElementById('tab-btn-code').className='inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white cursor-pointer transition-all';"
                                class="inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-white dark:bg-zinc-900 text-zinc-900 dark:text-white shadow-xs font-medium cursor-pointer transition-all"
                            >
                                {render_icon("play", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                                <span>"Live Preview"</span>
                            </button>
                            <button
                                id="tab-btn-code"
                                onclick="document.getElementById('tab-code').classList.remove('hidden'); document.getElementById('tab-preview').classList.add('hidden'); this.className='inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-white dark:bg-zinc-900 text-zinc-900 dark:text-white shadow-xs font-medium cursor-pointer transition-all'; document.getElementById('tab-btn-preview').className='inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white cursor-pointer transition-all';"
                                class="inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white cursor-pointer transition-all"
                            >
                                {render_icon("code", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                                <span>"RSX Source"</span>
                            </button>
                        </div>

                        <div class="flex items-center gap-2">
                            <span class="text-xs font-mono text-zinc-400">"src/routes/index.rs"</span>
                        </div>
                    </div>

                    // Tab 1: Live Interactive Preview
                    <div id="tab-preview" class="p-6 space-y-6">
                        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-lg bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80">
                            <div>
                                <div class="flex items-center gap-1.5">
                                    {render_icon("cpu", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                    <span class="text-xs font-mono uppercase tracking-wider text-zinc-400 dark:text-zinc-500 font-medium">
                                        "Client Island"
                                    </span>
                                </div>
                                <h3 class="text-sm font-semibold text-zinc-900 dark:text-zinc-100 mt-0.5">
                                    "Interactive State Counter"
                                </h3>
                                <p class="text-xs text-zinc-500 dark:text-zinc-400 mt-0.5">
                                    "Test reactive micro-interactions without hydration penalty"
                                </p>
                            </div>

                            <div class="flex items-center gap-2">
                                <button
                                    onclick="const el = document.getElementById('workbench-count'); el.innerText = Math.max(0, parseInt(el.innerText) - 1);"
                                    class="w-8 h-8 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-sm font-semibold text-zinc-800 dark:text-zinc-200 flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                    aria-label="decrement"
                                >
                                    "-"
                                </button>

                                <div class="px-4 py-1.5 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 font-mono text-sm font-bold text-zinc-900 dark:text-white min-w-[70px] text-center shadow-xs">
                                    <span id="workbench-count">"0"</span>
                                </div>

                                <button
                                    onclick="const el = document.getElementById('workbench-count'); el.innerText = parseInt(el.innerText) + 1;"
                                    class="w-8 h-8 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-sm font-semibold text-zinc-800 dark:text-zinc-200 flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                    aria-label="increment"
                                >
                                    "+"
                                </button>

                                <button
                                    onclick="document.getElementById('workbench-count').innerText = '0';"
                                    class="px-2.5 py-1.5 rounded-lg text-xs font-mono text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-white transition-colors cursor-pointer"
                                >
                                    "Reset"
                                </button>
                            </div>
                        </div>

                        // Live Greeting Input
                        <div class="p-4 rounded-lg bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 space-y-3">
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-1.5">
                                    {render_icon("sparkles", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                    <span class="text-xs font-mono uppercase tracking-wider text-zinc-400 dark:text-zinc-500 font-medium">
                                        "Reactive Binding"
                                    </span>
                                </div>
                                <span class="text-[11px] font-mono text-zinc-400">"Instant DOM update"</span>
                            </div>

                            <div class="flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
                                <input
                                    type="text"
                                    id="greeting-input"
                                    placeholder="Type your name..."
                                    oninput="document.getElementById('greeting-target').innerText = this.value || 'Rust Developer';"
                                    class="flex-1 px-3 py-2 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-xs text-zinc-900 dark:text-white placeholder:text-zinc-400 focus:outline-none focus:ring-2 focus:ring-amber-500/50"
                                />
                                <div class="px-4 py-2 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800/60 text-xs font-medium text-amber-800 dark:text-amber-300">
                                    <span>"Hello, "</span>
                                    <span id="greeting-target" class="font-bold">"Rust Developer"</span>
                                    <span>"!"</span>
                                </div>
                            </div>
                        </div>
                    </div>

                    // Tab 2: RSX Source Code
                    <div id="tab-code" class="hidden">
                        <div class="p-5 font-mono text-xs overflow-x-auto bg-[#121215] text-zinc-300 leading-relaxed border-t border-zinc-800/40">
                            <span class="text-zinc-500">"// src/routes/index.rs"</span><br />
                            <span class="text-amber-400">"use"</span>" yav::prelude::*;"<br />
                            <span class="text-amber-400">"use"</span>" axum::extract::Request;"<br /><br />
                            <span class="text-amber-400">"pub async fn"</span>" index(_req: Request) -> yav::server::HtmlPage {"<br />
                            "    let page = "<span class="text-sky-400">"rsx!"</span>" {"<br />
                            "        <AppLayout>"<br />
                            "            <h1 class=\"text-3xl font-bold\">\"Your Yav application is ready.\"</h1>"<br />
                            "            <p>\"Full-stack Rust with Axum 0.8 and RSX templates.\"</p>"<br />
                            "        </AppLayout>"<br />
                            "    };"<br /><br />
                            "    yav::server::HtmlPage::new(page)"<br />
                            "        .title(\"Yav — Rust Web Framework\")"<br />
                            "        .favicon(\"/public/favicon.svg\")"<br />
                            "}"
                        </div>
                    </div>
                </div>

                // 3. Project Anatomy (Interactive File Explorer)
                <div class="space-y-4">
                    <div class="flex items-center justify-between">
                        <h2 class="text-xs font-mono uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-semibold">
                            "Project Anatomy"
                        </h2>
                        <span class="text-xs font-mono text-zinc-400">"5 core files"</span>
                    </div>

                    <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/30 divide-y divide-zinc-100 dark:divide-zinc-800/60 overflow-hidden font-mono text-xs shadow-xs">
                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                {render_icon("file-code", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                <span class="text-amber-600 dark:text-amber-400 font-semibold">"src/routes/index.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Home route handler returning HtmlPage"</span>
                            </div>
                            <span class="px-2 py-0.5 rounded bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 font-sans text-[11px] font-semibold">"GET / (Current)"</span>
                        </div>

                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                {render_icon("file-code", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                <span class="text-amber-600 dark:text-amber-400 font-semibold">"src/routes/about.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <a href="/about" class="text-zinc-700 dark:text-zinc-300 font-sans text-xs hover:underline flex items-center gap-1">
                                    "Architecture specifications and pipeline breakdown"
                                </a>
                            </div>
                            <a href="/about" class="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-zinc-100 hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-zinc-800 dark:text-zinc-200 text-xs font-sans font-medium transition-colors">
                                <span>"GET /about"</span>
                                {render_icon("arrow-right", IconSize::Sm, Some("w-3 h-3"), None)}
                            </a>
                        </div>

                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                {render_icon("system:layout", IconSize::Sm, Some("w-4 h-4 text-sky-500"), None)}
                                <span class="text-zinc-800 dark:text-zinc-300 font-semibold">"src/layouts/app_layout.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Shared application frame, navigation, and theme switch"</span>
                            </div>
                            <span class="text-zinc-400 font-sans text-xs">"Layout"</span>
                        </div>

                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                {render_icon("play", IconSize::Sm, Some("w-4 h-4 text-emerald-500"), None)}
                                <span class="text-zinc-800 dark:text-zinc-300 font-semibold">"src/main.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Axum router registration, static asset serving, and server listen"</span>
                            </div>
                            <span class="text-zinc-400 font-sans text-xs">"Entrypoint"</span>
                        </div>

                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                <img src="/public/yav-document.png" class="w-4 h-4 object-contain shrink-0" alt="package.yavml icon" />
                                <span class="text-zinc-800 dark:text-zinc-300 font-semibold">"package.yavml"</span>
                                <span class="text-zinc-400">"→"</span>
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Project manifest for port, host, and Tailwind compiler"</span>
                            </div>
                            <span class="text-zinc-400 font-sans text-xs">"Manifest"</span>
                        </div>
                    </div>
                </div>

                // 4. CLI Workflow Commands
                <div class="space-y-4">
                    <h2 class="text-xs font-mono uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-semibold">
                        "Development Workflow"
                    </h2>

                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                        <button
                            onclick="navigator.clipboard.writeText('yav dev'); const el = this.querySelector('.cmd-copy'); el.innerText='copied!'; setTimeout(()=>el.innerText='click to copy', 1500);"
                            class="text-left rounded-xl border border-zinc-200 dark:border-zinc-800 p-4 bg-white dark:bg-zinc-900/40 hover:border-zinc-300 dark:hover:border-zinc-700 transition-all cursor-pointer group shadow-xs"
                        >
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    {render_icon("terminal", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                    <span class="font-mono text-xs font-bold text-zinc-900 dark:text-zinc-100">
                                        "$ yav dev"
                                    </span>
                                </div>
                                <span class="cmd-copy text-[10px] font-mono text-zinc-400 group-hover:text-amber-500 transition-colors flex items-center gap-1">
                                    {render_icon("copy", IconSize::Sm, Some("w-3 h-3"), None)}
                                    <span>"copy"</span>
                                </span>
                            </div>
                            <p class="text-xs text-zinc-500 dark:text-zinc-400 leading-relaxed font-sans mt-2">
                                "Starts dev server with automatic CSS rebuilds and live reload."
                            </p>
                        </button>

                        <button
                            onclick="navigator.clipboard.writeText('yav build --release'); const el = this.querySelector('.cmd-copy'); el.innerText='copied!'; setTimeout(()=>el.innerText='click to copy', 1500);"
                            class="text-left rounded-xl border border-zinc-200 dark:border-zinc-800 p-4 bg-white dark:bg-zinc-900/40 hover:border-zinc-300 dark:hover:border-zinc-700 transition-all cursor-pointer group shadow-xs"
                        >
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    {render_icon("package", IconSize::Sm, Some("w-4 h-4 text-sky-500"), None)}
                                    <span class="font-mono text-xs font-bold text-zinc-900 dark:text-zinc-100">
                                        "$ yav build --release"
                                    </span>
                                </div>
                                <span class="cmd-copy text-[10px] font-mono text-zinc-400 group-hover:text-amber-500 transition-colors flex items-center gap-1">
                                    {render_icon("copy", IconSize::Sm, Some("w-3 h-3"), None)}
                                    <span>"copy"</span>
                                </span>
                            </div>
                            <p class="text-xs text-zinc-500 dark:text-zinc-400 leading-relaxed font-sans mt-2">
                                "Compiles a single, optimized standalone native binary."
                            </p>
                        </button>

                        <button
                            onclick="navigator.clipboard.writeText('cargo test'); const el = this.querySelector('.cmd-copy'); el.innerText='copied!'; setTimeout(()=>el.innerText='click to copy', 1500);"
                            class="text-left rounded-xl border border-zinc-200 dark:border-zinc-800 p-4 bg-white dark:bg-zinc-900/40 hover:border-zinc-300 dark:hover:border-zinc-700 transition-all cursor-pointer group shadow-xs"
                        >
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    {render_icon("check-circle", IconSize::Sm, Some("w-4 h-4 text-emerald-500"), None)}
                                    <span class="font-mono text-xs font-bold text-zinc-900 dark:text-zinc-100">
                                        "$ cargo test"
                                    </span>
                                </div>
                                <span class="cmd-copy text-[10px] font-mono text-zinc-400 group-hover:text-amber-500 transition-colors flex items-center gap-1">
                                    {render_icon("copy", IconSize::Sm, Some("w-3 h-3"), None)}
                                    <span>"copy"</span>
                                </span>
                            </div>
                            <p class="text-xs text-zinc-500 dark:text-zinc-400 leading-relaxed font-sans mt-2">
                                "Runs your integration tests and RSX compile-time verification."
                            </p>
                        </button>
                    </div>
                </div>
            </div>
        </AppLayout>
    };

    let head_extra = r#"<link rel="icon" type="image/svg+xml" href="/public/favicon.svg">
<link rel="alternate icon" type="image/x-icon" href="/favicon.ico">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<script src="https://cdn.tailwindcss.com"></script>
<script>
  tailwind.config = {
    darkMode: 'class',
    theme: {
      extend: {
        fontFamily: {
          sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
          mono: ['JetBrains Mono', 'monospace'],
        },
      },
    },
  }
</script>
<link rel="stylesheet" href="/public/styles.css" />"#;

    yav::server::HtmlPage::new(page)
        .title("Yav — Rust Web Framework")
        .head_extra(head_extra)
        .favicon("/public/favicon.svg")
}
"##;
    std::fs::write(target_dir.join("src/routes/index.rs"), index_rs)?;

    // 8. Generate src/routes/about.rs
    let about_rs = r##"use yav::prelude::*;
use yav::icons::{render_icon, IconSize};
use axum::extract::Request;
use crate::layouts::*;

pub async fn about(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="space-y-12">
                <div class="space-y-3">
                    <a
                        href="/"
                        class="inline-flex items-center gap-1.5 text-xs font-mono text-zinc-500 hover:text-zinc-900 dark:hover:text-white transition-colors"
                    >
                        {render_icon("arrow-left", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                        <span>"Back to Overview"</span>
                    </a>

                    <h1 class="text-3xl font-bold tracking-tight text-zinc-900 dark:text-zinc-50">
                        "Architecture & Runtime"
                    </h1>
                    <p class="text-base text-zinc-600 dark:text-zinc-400 max-w-2xl leading-relaxed">
                        "Yav combines Axum 0.8 HTTP routing with compile-time RSX macro templates and the standalone Tailwind compiler into a single native binary."
                    </p>
                </div>

                // Request Flow
                <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/40 p-6 space-y-4 shadow-xs">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2">
                            {render_icon("cpu", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                            <h2 class="text-xs font-mono uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-semibold">
                                "Request Pipeline"
                            </h2>
                        </div>
                        <span class="text-xs font-mono text-zinc-400">"Sub-millisecond processing"</span>
                    </div>

                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs font-mono">
                        <div class="p-4 rounded-lg bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex flex-col justify-between">
                            <div class="flex items-center justify-between">
                                <span class="text-[10px] text-zinc-400 uppercase tracking-wider font-semibold">"Stage 01"</span>
                                {render_icon("server", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                            </div>
                            <div class="my-2.5">
                                <span class="font-semibold text-zinc-900 dark:text-zinc-100 text-sm">"Axum Router"</span>
                                <p class="text-[11px] text-zinc-500 font-sans mt-1 leading-relaxed">
                                    "Matchit radix tree resolves route paths in nanoseconds."
                                </p>
                            </div>
                            <span class="text-[10px] text-zinc-400">"HTTP / TCP Socket"</span>
                        </div>

                        <div class="p-4 rounded-lg bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex flex-col justify-between">
                            <div class="flex items-center justify-between">
                                <span class="text-[10px] text-amber-500 uppercase tracking-wider font-semibold">"Stage 02"</span>
                                {render_icon("code", IconSize::Sm, Some("w-3.5 h-3.5 text-amber-500"), None)}
                            </div>
                            <div class="my-2.5">
                                <span class="font-semibold text-amber-600 dark:text-amber-400 text-sm">"RSX Streaming Buffer"</span>
                                <p class="text-[11px] text-zinc-500 font-sans mt-1 leading-relaxed">
                                    "Zero-allocation streaming HTML generation verified at compile time."
                                </p>
                            </div>
                            <span class="text-[10px] text-zinc-400">"rustc verified"</span>
                        </div>

                        <div class="p-4 rounded-lg bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex flex-col justify-between">
                            <div class="flex items-center justify-between">
                                <span class="text-[10px] text-emerald-500 uppercase tracking-wider font-semibold">"Stage 03"</span>
                                {render_icon("zap", IconSize::Sm, Some("w-3.5 h-3.5 text-emerald-500"), None)}
                            </div>
                            <div class="my-2.5">
                                <span class="font-semibold text-emerald-600 dark:text-emerald-400 text-sm">"Native Byte Stream"</span>
                                <p class="text-[11px] text-zinc-500 font-sans mt-1 leading-relaxed">
                                    "Pre-escaped bytes transmitted directly to the client with optional gzip/brotli."
                                </p>
                            </div>
                            <span class="text-[10px] text-emerald-600 dark:text-emerald-400 font-medium">"Zero Node.js Overhead"</span>
                        </div>
                    </div>
                </div>

                // Technical Specifications
                <div class="space-y-4">
                    <div class="flex items-center gap-2">
                        {render_icon("layers", IconSize::Sm, Some("w-4 h-4 text-zinc-400"), None)}
                        <h2 class="text-xs font-mono uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-semibold">
                            "System Composition"
                        </h2>
                    </div>

                    <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/30 divide-y divide-zinc-100 dark:divide-zinc-800/60 text-xs overflow-hidden shadow-xs">
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <div class="flex items-center gap-2.5 text-zinc-600 dark:text-zinc-400 font-medium">
                                {render_icon("globe", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                                <span>"Core HTTP Framework"</span>
                            </div>
                            <span class="font-mono font-medium text-zinc-900 dark:text-white">"Axum 0.8 & Hyper 1.x"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <div class="flex items-center gap-2.5 text-zinc-600 dark:text-zinc-400 font-medium">
                                {render_icon("cpu", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                                <span>"Asynchronous Runtime"</span>
                            </div>
                            <span class="font-mono font-medium text-zinc-900 dark:text-white">"Tokio Multi-threaded"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <div class="flex items-center gap-2.5 text-zinc-600 dark:text-zinc-400 font-medium">
                                {render_icon("code", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                                <span>"Component Templating"</span>
                            </div>
                            <span class="font-mono font-medium text-zinc-900 dark:text-white">"yav-macros (rsx!)"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <div class="flex items-center gap-2.5 text-zinc-600 dark:text-zinc-400 font-medium">
                                {render_icon("system:layout", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                                <span>"CSS Pipeline"</span>
                            </div>
                            <span class="font-mono font-medium text-zinc-900 dark:text-white">"Standalone Tailwind CLI"</span>
                        </div>
                        <div class="px-5 py-3.5 flex items-center justify-between">
                            <div class="flex items-center gap-2.5 text-zinc-600 dark:text-zinc-400 font-medium">
                                {render_icon("check-circle", IconSize::Sm, Some("w-3.5 h-3.5 text-emerald-500"), None)}
                                <span>"Runtime Engine"</span>
                            </div>
                            <span class="font-mono font-medium text-emerald-600 dark:text-emerald-400">"None (Native Machine Code)"</span>
                        </div>
                    </div>
                </div>

                // Code Example: How to add a new route
                <div class="space-y-4">
                    <div class="flex items-center gap-2">
                        {render_icon("terminal", IconSize::Sm, Some("w-4 h-4 text-zinc-400"), None)}
                        <h2 class="text-xs font-mono uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-semibold">
                            "Defining a New Route"
                        </h2>
                    </div>

                    <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-[#121215] text-zinc-300 p-5 font-mono text-xs overflow-x-auto shadow-xs leading-relaxed">
                        <span class="text-zinc-500">"// 1. Create src/routes/hello.rs"</span><br />
                        <span class="text-amber-400">"use"</span>" yav::prelude::*;"<br />
                        <span class="text-amber-400">"use"</span>" axum::extract::Path;"<br /><br />
                        <span class="text-amber-400">"pub async fn"</span>" hello(Path(name): Path<String>) -> yav::server::HtmlPage {"<br />
                        "    let page = "<span class="text-sky-400">"rsx!"</span>" {"<br />
                        "        <AppLayout>"<br />
                        "            <h1 class=\"text-2xl font-bold\">\"Hello, \" {name} \"!\"</h1>"<br />
                        "        </AppLayout>"<br />
                        "    };"<br />
                        "    yav::server::HtmlPage::new(page)"<br />
                        "}"
                    </div>
                </div>
            </div>
        </AppLayout>
    };

    let head_extra = r#"<link rel="icon" type="image/svg+xml" href="/public/favicon.svg">
<link rel="alternate icon" type="image/x-icon" href="/favicon.ico">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<script src="https://cdn.tailwindcss.com"></script>
<script>
  tailwind.config = {
    darkMode: 'class',
    theme: {
      extend: {
        fontFamily: {
          sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
          mono: ['JetBrains Mono', 'monospace'],
        },
      },
    },
  }
</script>
<link rel="stylesheet" href="/public/styles.css" />"#;

    yav::server::HtmlPage::new(page)
        .title("Architecture — Yav Framework")
        .head_extra(head_extra)
        .favicon("/public/favicon.svg")
}
"##;
    std::fs::write(target_dir.join("src/routes/about.rs"), about_rs)?;

    // 9. Generate src/components/mod.rs
    let components_mod = r#"//! Reusable UI components.
"#;
    std::fs::write(target_dir.join("src/components/mod.rs"), components_mod)?;

    // 10. Generate src/styles/app.css
    let app_css = r#"@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  body {
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  code, pre, .font-mono {
    font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace;
  }
}
"#;
    std::fs::write(target_dir.join("src/styles/app.css"), app_css)?;

    // 11. Generate tailwind.config.js for class-based dark mode
    let tailwind_config_js = r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: [
    "./src/**/*.{rs,html,yav,rsx,yavml}",
    "./public/**/*.{html,js}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
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

    // 10b. Generate initial public/styles.css fallback
    let styles_css = r#"/* Yav Compiled Stylesheet */
"#;
    std::fs::write(target_dir.join("public/styles.css"), styles_css)?;

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

/// CLI entry point for the `yav new` command.
pub async fn run(name: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("\n\x1b[1;32m🎉 Successfully created project '{name}'!\x1b[0m");

    // Check and offer/install Yav editor extension
    crate::ext::prompt_and_install_if_needed();

    // Check and offer/install Tailwind CSS standalone compiler
    let _ = crate::tailwind_setup::prompt_and_install_in_new("3.4.17").await;

    println!("👉 To get started:");
    println!("   cd {name}");
    println!("   yav dev\n");

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
        assert!(!main_content.contains(".port("));

        let cargo_content = std::fs::read_to_string(temp_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_content.contains("name = \"test-site\""));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
