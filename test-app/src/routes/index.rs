use yav::prelude::*;
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
                                data-workbench-tab="preview"
                                class="inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-white dark:bg-zinc-900 text-zinc-900 dark:text-white shadow-xs font-medium cursor-pointer transition-all"
                            >
                                {render_icon("play", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                                <span>"Live Preview"</span>
                            </button>
                            <button
                                id="tab-btn-code"
                                data-workbench-tab="code"
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
                                    data-workbench-count="dec"
                                    class="w-8 h-8 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-sm font-semibold text-zinc-800 dark:text-zinc-200 flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                    aria-label="decrement"
                                >
                                    "-"
                                </button>

                                <div class="px-4 py-1.5 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 font-mono text-sm font-bold text-zinc-900 dark:text-white min-w-[70px] text-center shadow-xs">
                                    <span id="workbench-count">"0"</span>
                                </div>

                                <button
                                    data-workbench-count="inc"
                                    class="w-8 h-8 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-sm font-semibold text-zinc-800 dark:text-zinc-200 flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                    aria-label="increment"
                                >
                                    "+"
                                </button>

                                <button
                                    data-workbench-count="reset"
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
                        <span class="text-xs font-mono text-zinc-400">"7 core files"</span>
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
                                {render_icon("check-circle", IconSize::Sm, Some("w-4 h-4 text-emerald-500"), None)}
                                <span class="text-amber-600 dark:text-amber-400 font-semibold">"src/routes/health.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <a href="/health" class="text-zinc-700 dark:text-zinc-300 font-sans text-xs hover:underline flex items-center gap-1">
                                    "Production health check endpoint returning JSON status"
                                </a>
                            </div>
                            <a href="/health" class="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-zinc-100 hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-zinc-800 dark:text-zinc-200 text-xs font-sans font-medium transition-colors">
                                <span>"GET /health"</span>
                                {render_icon("arrow-right", IconSize::Sm, Some("w-3 h-3"), None)}
                            </a>
                        </div>

                        <div class="p-4 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/40 transition-colors">
                            <div class="flex items-center gap-3">
                                {render_icon("alert-circle", IconSize::Sm, Some("w-4 h-4 text-amber-500"), None)}
                                <span class="text-amber-600 dark:text-amber-400 font-semibold">"src/routes/not_found.rs"</span>
                                <span class="text-zinc-400">"→"</span>
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Custom styled 404 fallback page replacing Axum plain text"</span>
                            </div>
                            <span class="px-2 py-0.5 rounded bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-400 font-sans text-[11px] font-semibold">"404 Fallback"</span>
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
                                <span class="text-zinc-700 dark:text-zinc-300 font-sans text-xs">"Axum router registration, PORT binding, and server listen"</span>
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
                            data-copy-text="yav dev"
                            data-copy-label="click to copy"
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
                            data-copy-text="yav build --release"
                            data-copy-label="click to copy"
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
                            data-copy-text="cargo test"
                            data-copy-label="click to copy"
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
  };
  document.addEventListener('click', function(e) {
    var tabBtn = e.target.closest && e.target.closest('[data-workbench-tab]');
    if (tabBtn) {
      var mode = tabBtn.getAttribute('data-workbench-tab');
      var prev = document.getElementById('tab-preview');
      var code = document.getElementById('tab-code');
      var bPrev = document.getElementById('tab-btn-preview');
      var bCode = document.getElementById('tab-btn-code');
      var active = 'inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-white dark:bg-zinc-900 text-zinc-900 dark:text-white shadow-xs font-medium cursor-pointer transition-all';
      var inactive = 'inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white cursor-pointer transition-all';
      if (mode === 'preview' && prev && code) {
        prev.classList.remove('hidden'); code.classList.add('hidden');
        if (bPrev) bPrev.className = active;
        if (bCode) bCode.className = inactive;
      } else if (mode === 'code' && prev && code) {
        code.classList.remove('hidden'); prev.classList.add('hidden');
        if (bCode) bCode.className = active;
        if (bPrev) bPrev.className = inactive;
      }
      return;
    }
    var cntBtn = e.target.closest && e.target.closest('[data-workbench-count]');
    if (cntBtn) {
      var op = cntBtn.getAttribute('data-workbench-count');
      var el = document.getElementById('workbench-count');
      if (el) {
        var cur = parseInt(el.innerText || '0', 10) || 0;
        el.innerText = op === 'inc' ? String(cur + 1) : op === 'dec' ? String(Math.max(0, cur - 1)) : '0';
      }
      return;
    }
    var cpBtn = e.target.closest && e.target.closest('[data-copy-text]');
    if (cpBtn && navigator.clipboard) {
      var txt = cpBtn.getAttribute('data-copy-text');
      var lbl = cpBtn.getAttribute('data-copy-label') || txt;
      navigator.clipboard.writeText(txt);
      var out = cpBtn.querySelector('.cmd-copy, .copy-txt');
      if (out) { out.innerText = 'copied!'; setTimeout(function() { out.innerText = lbl; }, 1200); }
    }
  });
</script>
<link rel="stylesheet" href="/public/styles.css" />"#;

    yav::server::HtmlPage::new(page)
        .title("Yav — Rust Web Framework")
        .head_extra(head_extra)
        .favicon("/public/favicon.svg")
}
