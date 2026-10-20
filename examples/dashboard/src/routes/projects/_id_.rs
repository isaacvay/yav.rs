//! Dynamic single project route (`/projects/:id`).

use super::_data::*;
use crate::layouts::*;
use axum::extract::Request;
use yav::prelude::*;

pub async fn _id_(req: Request) -> yav::server::HtmlPage {
    let ctx = RouteContext::from_request(&req);
    let project_id = ctx.param("id").unwrap_or("unknown");

    let projects = get_sample_projects();
    let current_proj = projects.into_iter().find(|p| p.id == project_id);

    let content = match current_proj {
        Some(p) => rsx! {
            <div class="space-y-6">
                <a href="/projects" class="inline-flex items-center gap-1.5 text-xs font-mono text-zinc-500 hover:text-zinc-900 dark:hover:text-white transition-colors">
                    {render_icon("arrow-left", IconSize::Sm, None, None)}
                    <span>"Back to Projects"</span>
                </a>

                <div class="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-6">
                    <div class="flex items-center justify-between flex-wrap gap-4">
                        <div>
                            <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-emerald-50 dark:bg-emerald-950/40 text-emerald-600 dark:text-emerald-400 text-xs font-mono">
                                <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
                                <span>{p.status}</span>
                            </div>
                            <h1 class="text-3xl font-black text-zinc-900 dark:text-white mt-2">{p.name}</h1>
                            <p class="text-xs font-mono text-zinc-400 mt-1">{format!("Dynamic Route: /projects/{}", p.id)}</p>
                        </div>

                        <div class="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 text-right">
                            <span class="text-xs text-zinc-400 block font-mono">"P99 Response"</span>
                            <span class="text-2xl font-bold font-mono text-zinc-900 dark:text-white">{p.latency}</span>
                        </div>
                    </div>

                    <p class="text-sm text-zinc-600 dark:text-zinc-300 leading-relaxed">{p.description}</p>

                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-4 border-t border-zinc-100 dark:border-zinc-800/80 text-xs font-mono">
                        <div class="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80">
                            <span class="text-zinc-400 block">"Instances"</span>
                            <span class="text-base font-bold text-zinc-900 dark:text-white">{format!("{} edge replicas", p.nodes)}</span>
                        </div>
                        <div class="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80">
                            <span class="text-zinc-400 block">"SSL / TLS"</span>
                            <span class="text-base font-bold text-emerald-600 dark:text-emerald-400">"TLS 1.3 Active"</span>
                        </div>
                        <div class="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80">
                            <span class="text-zinc-400 block">"Data Loader"</span>
                            <span class="text-base font-bold text-orange-600 dark:text-orange-400">"Yav SSR Loader<T>"</span>
                        </div>
                    </div>
                </div>
            </div>
        },
        None => rsx! {
            <div class="p-12 text-center rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 space-y-4">
                <h2 class="text-2xl font-bold text-zinc-900 dark:text-white">"Project Not Found"</h2>
                <p class="text-xs text-zinc-500">{format!("No project found with ID '{}'", project_id)}</p>
                <a href="/projects" class="inline-block px-4 py-2 rounded-xl bg-orange-600 text-white text-xs font-semibold">
                    "Return to Projects List"
                </a>
            </div>
        },
    };

    let page = rsx! {
        <DashboardLayout current_path="/projects">
            {content}
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
