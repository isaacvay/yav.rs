//! Unified SaaS Dashboard Layout with sidebar, navbar, command palette, and dark mode.

use yav::icons::{render_icon, IconSize};
use yav::prelude::*;

#[derive(Clone, Default)]
pub struct DashboardLayoutProps {
    pub current_path: &'static str,
    pub children: Vec<Node>,
}

#[component]
pub fn DashboardLayout(props: DashboardLayoutProps) -> Node {
    let current = props.current_path;

    // Navigation item helper
    let nav_item = |name: &'static str,
                    path: &'static str,
                    icon: &'static str,
                    badge: Option<&'static str>|
     -> Node {
        let is_active = current == path;
        let active_class = if is_active {
            "bg-orange-500/10 text-orange-600 dark:text-orange-400 font-semibold"
        } else {
            "text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-900 hover:text-zinc-900 dark:hover:text-zinc-100 font-medium"
        };

        rsx! {
            <a
                href={path}
                class={format!("flex items-center justify-between px-3.5 py-2.5 rounded-lg text-sm transition-all duration-150 {active_class}")}
            >
                <div class="flex items-center gap-3">
                    {render_icon(icon, IconSize::Sm, Some(if is_active { "w-4 h-4 text-orange-500" } else { "w-4 h-4 text-zinc-400" }), None)}
                    <span>{name}</span>
                </div>
                {badge.map(|b| rsx! {
                    <span class="px-2 py-0.5 text-xs font-mono rounded-full bg-zinc-200/80 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300">
                        {b}
                    </span>
                })}
            </a>
        }
    };

    let anti_fouc = r#"(function(){var t=localStorage.getItem('theme');if(t==='dark'||(!t&&window.matchMedia('(prefers-color-scheme: dark)').matches)){document.documentElement.classList.add('dark')}else{document.documentElement.classList.remove('dark')}})();"#;
    let tailwind_config = r#"tailwind.config={darkMode:'class',theme:{extend:{fontFamily:{sans:['Inter','system-ui','sans-serif']},colors:{brand:{50:'#fff7ed',100:'#ffedd5',500:'#f97316',600:'#ea580c',700:'#c2410c'}}}}};"#;

    rsx! {
        <html lang="en" class="h-full">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Yav SaaS Platform — Rust-first Web Framework"</title>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
                <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
                <script src="https://cdn.tailwindcss.com"></script>
                <script>{Node::raw(tailwind_config)}</script>
                <link rel="stylesheet" href="/styles.css" />
                <script>{Node::raw(anti_fouc)}</script>
            </head>
            <body class="min-h-screen bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100 flex flex-col antialiased">
                <div class="flex-1 flex min-h-screen">
                    // Sidebar
                    <aside class="w-64 border-r border-zinc-200 dark:border-zinc-800/80 bg-white dark:bg-zinc-900/60 flex flex-col justify-between shrink-0 hidden md:flex">
                        <div class="p-5 space-y-6">
                            // Brand
                            <a href="/" class="flex items-center gap-2.5 px-2">
                                <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-400 via-orange-500 to-red-600 flex items-center justify-center shadow-md">
                                    {render_icon("zap", IconSize::Sm, Some("w-4 h-4 text-white"), None)}
                                </div>
                                <div>
                                    <span class="font-bold text-base tracking-tight text-zinc-900 dark:text-white">"Yav SaaS"</span>
                                    <span class="block text-[10px] font-mono text-zinc-400 uppercase tracking-widest">"Enterprise v0.1"</span>
                                </div>
                            </a>

                            // Main Nav
                            <nav class="space-y-1">
                                <span class="px-3 text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">"Core Platform"</span>
                                <div class="pt-2 space-y-1">
                                    {nav_item("Dashboard", "/", "layout", None)}
                                    {nav_item("Projects", "/projects", "folder", Some("12"))}
                                    {nav_item("Component Gallery", "/components", "layers", Some("40+"))}
                                    {nav_item("Icon Catalog", "/icons", "grid", Some("80+"))}
                                </div>
                            </nav>

                            <nav class="space-y-1">
                                <span class="px-3 text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">"System"</span>
                                <div class="pt-2 space-y-1">
                                    {nav_item("Settings & Forms", "/settings", "settings", None)}
                                </div>
                            </nav>
                        </div>

                        // Footer Status Card
                        <div class="p-4 m-4 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950/60 text-xs">
                            <div class="flex items-center gap-2 text-emerald-600 dark:text-emerald-400 font-medium">
                                <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
                                <span>"Cluster Online"</span>
                            </div>
                            <p class="text-zinc-500 mt-1 text-[11px]">"Axum 0.8 / Tokio Native Engine"</p>
                        </div>
                    </aside>

                    // Main Content Area
                    <div class="flex-1 flex flex-col min-w-0">
                        // Top Navbar
                        <header class="h-16 border-b border-zinc-200 dark:border-zinc-800/80 bg-white/80 dark:bg-zinc-900/40 backdrop-blur-md px-6 flex items-center justify-between sticky top-0 z-30">
                            <div class="flex items-center gap-3">
                                // Mobile menu button
                                <button class="md:hidden p-2 rounded-lg text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800" aria-label="menu">
                                    {render_icon("menu", IconSize::Md, None, None)}
                                </button>

                                <div class="relative max-w-xs sm:max-w-md w-full">
                                    <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-zinc-50/80 dark:bg-zinc-950/40 text-xs text-zinc-400">
                                        {render_icon("search", IconSize::Sm, Some("w-3.5 h-3.5 text-zinc-400"), None)}
                                        <input
                                            type="text"
                                            placeholder="Search commands, routes, tokens... (Press ⌘K)"
                                            class="bg-transparent border-none outline-hidden w-full text-zinc-800 dark:text-zinc-200 placeholder-zinc-400"
                                        />
                                        <kbd class="hidden sm:inline-block px-1.5 py-0.5 rounded bg-zinc-200 dark:bg-zinc-800 text-[10px] font-mono">"⌘K"</kbd>
                                    </div>
                                </div>
                            </div>

                            <div class="flex items-center gap-3">
                                // Light / Dark Mode Toggle
                                <button
                                    onclick="var d=document.documentElement.classList.toggle('dark');localStorage.setItem('theme',d?'dark':'light');"
                                    class="p-2 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white transition-colors cursor-pointer"
                                    aria-label="Toggle theme"
                                >
                                    <span class="dark:hidden">{render_icon("moon", IconSize::Sm, Some("w-4 h-4"), None)}</span>
                                    <span class="hidden dark:inline">{render_icon("sun", IconSize::Sm, Some("w-4 h-4 text-amber-400"), None)}</span>
                                </button>

                                // Notifications
                                <button class="p-2 rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white transition-colors cursor-pointer" aria-label="Notifications">
                                    {render_icon("bell", IconSize::Sm, Some("w-4 h-4"), None)}
                                </button>

                                // User Avatar
                                <div class="w-8 h-8 rounded-full bg-gradient-to-tr from-orange-500 to-amber-400 text-white font-semibold text-xs flex items-center justify-center shadow-xs">
                                    "YV"
                                </div>
                            </div>
                        </header>

                        // Page Container
                        <main class="flex-1 p-6 sm:p-8 max-w-7xl w-full mx-auto space-y-8">
                            {props.children}
                        </main>
                    </div>
                </div>
            </body>
        </html>
    }
}
