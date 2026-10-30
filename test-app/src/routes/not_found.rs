use yav::prelude::*;
use yav::icons::{render_icon, IconSize};
use axum::extract::Request;
use crate::layouts::*;

/// Custom 404 Not Found error page with application layout.
pub async fn not_found(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <AppLayout>
            <div class="min-h-[50vh] flex flex-col items-center justify-center text-center px-4 py-16">
                <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-amber-500/10 border border-amber-500/20 text-xs font-mono text-amber-600 dark:text-amber-400 mb-6">
                    {render_icon("alert-circle", IconSize::Sm, Some("w-3.5 h-3.5 text-amber-500"), None)}
                    <span>"404 Not Found"</span>
                </div>

                <h1 class="text-5xl sm:text-6xl font-black tracking-tight text-zinc-900 dark:text-zinc-50 mb-3 font-mono">
                    "404"
                </h1>

                <h2 class="text-xl sm:text-2xl font-bold text-zinc-800 dark:text-zinc-200 mb-3">
                    "Page not found"
                </h2>

                <p class="text-sm text-zinc-600 dark:text-zinc-400 max-w-md mx-auto mb-8 leading-relaxed">
                    "The page you requested could not be found. It may have been moved, renamed, or is temporarily unavailable."
                </p>

                <div class="flex items-center gap-3">
                    <a
                        href="/"
                        class="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 dark:bg-white dark:hover:bg-zinc-100 text-white dark:text-zinc-900 text-xs font-semibold shadow-xs transition-colors"
                    >
                        {render_icon("home", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                        <span>"Back to Overview"</span>
                    </a>
                    <a
                        href="/about"
                        class="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-zinc-200 dark:border-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-700 dark:text-zinc-300 text-xs font-semibold transition-colors"
                    >
                        {render_icon("layers", IconSize::Sm, Some("w-3.5 h-3.5"), None)}
                        <span>"Architecture"</span>
                    </a>
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
        .status(axum::http::StatusCode::NOT_FOUND)
        .title("404 — Page Not Found | Yav")
        .head_extra(head_extra)
        .favicon("/public/favicon.svg")
}
