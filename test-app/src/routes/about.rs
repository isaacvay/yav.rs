use yav::prelude::*;
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
