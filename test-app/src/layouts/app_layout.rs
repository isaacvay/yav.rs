//! Default application layout with navigation and footer.

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
