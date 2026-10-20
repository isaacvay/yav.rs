//! Project list handler (`/projects`).

use super::_data::*;
use crate::layouts::*;
use axum::extract::Request;
use yav::prelude::*;

pub async fn index(_req: Request) -> yav::server::HtmlPage {
    let projects = get_sample_projects();

    let page = rsx! {
        <DashboardLayout current_path="/projects">
            <div class="space-y-8">
                <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                    <div>
                        <h1 class="text-3xl font-black tracking-tight text-zinc-900 dark:text-white">
                            "Microservice Projects"
                        </h1>
                        <p class="text-sm text-zinc-500 dark:text-zinc-400 mt-1">
                            "Dynamic route segments and server-side data loading demonstration."
                        </p>
                    </div>

                    <a
                        href="/projects/yav-auth-service"
                        class="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-orange-600 hover:bg-orange-700 text-white text-xs font-semibold shadow-sm transition-colors"
                    >
                        <span>"Open Demo Project (:id)"</span>
                        {render_icon("arrow-right", IconSize::Sm, None, None)}
                    </a>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                    {projects.iter().map(|p| {
                        let is_healthy = p.status == "Healthy";
                        let status_badge = if is_healthy {
                            "bg-emerald-50 dark:bg-emerald-950/40 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-800"
                        } else {
                            "bg-amber-50 dark:bg-amber-950/40 text-amber-600 dark:text-amber-400 border-amber-200 dark:border-amber-800"
                        };

                        rsx! {
                            <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs flex flex-col justify-between space-y-4">
                                <div class="space-y-2">
                                    <div class="flex items-center justify-between">
                                        <span class={format!("px-2 py-0.5 rounded-full border text-[10px] font-semibold {status_badge}")}>
                                            {p.status}
                                        </span>
                                        <span class="text-xs font-mono text-zinc-400">{format!("{} nodes", p.nodes)}</span>
                                    </div>
                                    <h3 class="font-bold text-base text-zinc-900 dark:text-white mt-1">{p.name}</h3>
                                    <p class="text-xs text-zinc-500 leading-relaxed">{p.description}</p>
                                </div>

                                <div class="pt-4 border-t border-zinc-100 dark:border-zinc-800/80 flex items-center justify-between">
                                    <span class="text-xs font-mono text-zinc-400">
                                        "Latency: " <strong class="text-zinc-700 dark:text-zinc-200">{p.latency}</strong>
                                    </span>
                                    <a
                                        href={format!("/projects/{}", p.id)}
                                        class="inline-flex items-center gap-1.5 text-xs font-semibold text-orange-600 dark:text-orange-400 hover:underline"
                                    >
                                        <span>"Details"</span>
                                        {render_icon("arrow-right", IconSize::Sm, None, None)}
                                    </a>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
