//! Interactive Icon Explorer showcasing all 80+ embedded Lucide SVG icons.

use crate::layouts::*;
use axum::extract::Request;
use yav::prelude::*;

pub async fn icons(_req: Request) -> yav::server::HtmlPage {
    let icon_card = |name: &'static str| -> Node {
        rsx! {
            <div
                class="icon-item p-4 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 hover:border-orange-500/50 dark:hover:border-orange-500/50 hover:shadow-sm flex flex-col items-center justify-center gap-3 transition-all cursor-pointer group"
                onclick={format!("navigator.clipboard.writeText('render_icon(\"{name}\", IconSize::Md, None, None)'); alert('Copied icon code for \"{name}\" to clipboard!');")}
                title={format!("Click to copy Rust code for {name}")}
                data-name={name}
            >
                <div class="w-10 h-10 rounded-lg bg-zinc-50 dark:bg-zinc-800/80 flex items-center justify-center text-zinc-700 dark:text-zinc-200 group-hover:scale-110 group-hover:text-orange-500 transition-all">
                    {render_icon(name, IconSize::Md, None, None)}
                </div>
                <span class="text-[11px] font-mono text-zinc-600 dark:text-zinc-400 truncate max-w-full text-center">
                    {name}
                </span>
            </div>
        }
    };

    let search_js = r#"
    function filterIcons(query) {
        var q = query.toLowerCase();
        var items = document.querySelectorAll('.icon-item');
        var count = 0;
        items.forEach(function(item) {
            var name = item.getAttribute('data-name');
            if (!q || name.indexOf(q) !== -1) {
                item.style.display = 'flex';
                count++;
            } else {
                item.style.display = 'none';
            }
        });
        document.getElementById('icon-count').innerText = count;
    }
    "#;

    let icons = [
        "activity",
        "a-large-small",
        "alert-circle",
        "alert-triangle",
        "arrow-down",
        "arrow-left",
        "arrow-right",
        "arrow-up",
        "bell",
        "bell-off",
        "calendar",
        "chart-bar",
        "check",
        "check-circle",
        "chevron-down",
        "chevron-left",
        "chevron-right",
        "chevron-up",
        "circle",
        "clock",
        "cloud",
        "code",
        "copy",
        "cpu",
        "database",
        "download",
        "edit",
        "external-link",
        "eye",
        "eye-off",
        "file",
        "file-text",
        "filter",
        "folder",
        "globe",
        "grid",
        "heart",
        "help-circle",
        "home",
        "image",
        "inbox",
        "info",
        "layers",
        "link",
        "list",
        "loader",
        "lock",
        "mail",
        "map-pin",
        "maximize",
        "menu",
        "message-circle",
        "message-square",
        "minimize",
        "moon",
        "more-horizontal",
        "more-vertical",
        "package",
        "play",
        "plus",
        "refresh-cw",
        "search",
        "send",
        "server",
        "settings",
        "share",
        "shield",
        "shopping-cart",
        "slash",
        "sliders",
        "star",
        "sun",
        "terminal",
        "trash",
        "trending-down",
        "trending-up",
        "unlock",
        "upload",
        "user",
        "users",
        "volume",
        "volume-2",
        "volume-x",
        "wifi",
        "wifi-off",
        "x",
        "x-circle",
        "zap",
    ];

    let page = rsx! {
        <DashboardLayout current_path="/icons">
            <div class="space-y-8">
                <script>{Node::raw(search_js)}</script>

                // Header
                <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                    <div>
                        <div class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-orange-50 dark:bg-orange-950/40 border border-orange-200 dark:border-orange-800/60 text-xs font-mono text-orange-600 dark:text-orange-400">
                            {render_icon("grid", IconSize::Sm, None, None)}
                            <span>"yav-icons • Zero-Dependency SVGs"</span>
                        </div>
                        <h1 class="text-3xl font-black tracking-tight text-zinc-900 dark:text-white mt-2">
                            "Built-in Lucide Icons Catalog"
                        </h1>
                        <p class="text-sm text-zinc-500 dark:text-zinc-400 mt-1">
                            "Click any icon card to copy its type-safe Rust rendering snippet directly to your clipboard."
                        </p>
                    </div>

                    // Search input
                    <div class="max-w-xs w-full">
                        <div class="flex items-center gap-2 px-3 py-2 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-xs text-zinc-400 shadow-xs">
                            {render_icon("search", IconSize::Sm, None, None)}
                            <input
                                type="text"
                                placeholder="Filter icons..."
                                oninput="filterIcons(this.value);"
                                class="bg-transparent border-none outline-hidden w-full text-zinc-800 dark:text-zinc-200 placeholder-zinc-400"
                            />
                        </div>
                    </div>
                </div>

                // Metrics counter bar
                <div class="flex items-center justify-between text-xs text-zinc-500 pb-2 border-b border-zinc-200 dark:border-zinc-800">
                    <span>
                        "Showing " <strong id="icon-count" class="text-zinc-900 dark:text-white font-mono">{icons.len().to_string()}</strong> " icons"
                    </span>
                    <span class="font-mono text-zinc-400">"Embedded SVG format"</span>
                </div>

                // Icons Grid
                <div class="grid grid-cols-2 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-3">
                    {icons.iter().map(|&name| icon_card(name)).collect::<Vec<_>>()}
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
