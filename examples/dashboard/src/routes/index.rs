//! SaaS Dashboard Overview page.

use crate::layouts::*;
use axum::extract::Request;
use yav::icons::{render_icon, IconSize};
use yav::prelude::*;

pub async fn index(_req: Request) -> yav::server::HtmlPage {
    let stat_card = |title: &'static str,
                     value: &'static str,
                     change: &'static str,
                     is_pos: bool,
                     icon: &'static str|
     -> Node {
        let (badge_bg, badge_text, trend_icon) = if is_pos {
            (
                "bg-emerald-50 dark:bg-emerald-950/40 border-emerald-200 dark:border-emerald-800",
                "text-emerald-700 dark:text-emerald-400",
                "trending-up",
            )
        } else {
            (
                "bg-rose-50 dark:bg-rose-950/40 border-rose-200 dark:border-rose-800",
                "text-rose-700 dark:text-rose-400",
                "trending-down",
            )
        };

        rsx! {
            <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs flex flex-col justify-between">
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-zinc-500 dark:text-zinc-400">{title}</span>
                    <div class="w-8 h-8 rounded-lg bg-zinc-100 dark:bg-zinc-800 flex items-center justify-center text-zinc-600 dark:text-zinc-300">
                        {render_icon(icon, IconSize::Sm, None, None)}
                    </div>
                </div>
                <div class="mt-4">
                    <span class="text-2xl sm:text-3xl font-black text-zinc-900 dark:text-white tracking-tight">{value}</span>
                    <div class="mt-2 flex items-center gap-2">
                        <span class={format!("inline-flex items-center gap-1 px-2 py-0.5 rounded-full border text-xs font-medium {badge_bg} {badge_text}")}>
                            {render_icon(trend_icon, IconSize::Sm, None, None)}
                            <span>{change}</span>
                        </span>
                        <span class="text-xs text-zinc-400">"vs last month"</span>
                    </div>
                </div>
            </div>
        }
    };

    let table_row = |name: &'static str,
                     email: &'static str,
                     role: &'static str,
                     status: &'static str,
                     amount: &'static str|
     -> Node {
        let (status_class, status_dot) = match status {
            "Active" => ("bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-800", "bg-emerald-500"),
            "Pending" => ("bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-200 dark:border-amber-800", "bg-amber-500"),
            _ => ("bg-zinc-500/10 text-zinc-600 dark:text-zinc-400 border-zinc-200 dark:border-zinc-800", "bg-zinc-400"),
        };

        rsx! {
            <tr class="border-b border-zinc-100 dark:border-zinc-800/60 hover:bg-zinc-50/60 dark:hover:bg-zinc-900/40 transition-colors">
                <td class="py-3.5 px-4">
                    <div class="flex items-center gap-3">
                        <div class="w-8 h-8 rounded-full bg-zinc-200 dark:bg-zinc-800 flex items-center justify-center text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                            {name.chars().next().unwrap_or('?').to_string()}
                        </div>
                        <div>
                            <span class="font-medium text-sm text-zinc-900 dark:text-white block">{name}</span>
                            <span class="text-xs text-zinc-400 block">{email}</span>
                        </div>
                    </div>
                </td>
                <td class="py-3.5 px-4 text-xs text-zinc-600 dark:text-zinc-400 font-medium">{role}</td>
                <td class="py-3.5 px-4">
                    <span class={format!("inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full border text-xs font-medium {status_class}")}>
                        <span class={format!("w-1.5 h-1.5 rounded-full {status_dot}")}></span>
                        <span>{status}</span>
                    </span>
                </td>
                <td class="py-3.5 px-4 text-xs font-mono font-bold text-zinc-900 dark:text-white text-right">{amount}</td>
            </tr>
        }
    };

    let page = rsx! {
        <DashboardLayout current_path="/">
            <div class="space-y-8">
                // Page Header
                <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                    <div>
                        <h1 class="text-2xl sm:text-3xl font-black tracking-tight text-zinc-900 dark:text-white">
                            "Enterprise Metrics & Activity"
                        </h1>
                        <p class="text-sm text-zinc-500 dark:text-zinc-400 mt-1">
                            "Real-time overview of your distributed Yav microservices and client telemetry."
                        </p>
                    </div>

                    <div class="flex items-center gap-2.5">
                        <a
                            href="/projects"
                            class="inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-xs font-medium text-zinc-800 dark:text-zinc-200 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors shadow-xs"
                        >
                            {render_icon("folder", IconSize::Sm, None, None)}
                            <span>"View Projects"</span>
                        </a>

                        <button
                            onclick="alert('Interactive Yav Modal triggered via client island!');"
                            class="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-orange-600 hover:bg-orange-700 text-white text-xs font-semibold shadow-sm transition-colors cursor-pointer"
                        >
                            {render_icon("plus", IconSize::Sm, None, None)}
                            <span>"New Service"</span>
                        </button>
                    </div>
                </div>

                // KPI Stat Cards Grid
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                    {stat_card("Monthly Recurring Revenue", "$148,290", "+14.2%", true, "activity")}
                    {stat_card("Active Enterprise Tenants", "2,845", "+8.1%", true, "users")}
                    {stat_card("Edge Server Latency", "1.42 ms", "-18.5%", true, "zap")}
                    {stat_card("Memory Utilization", "42.8 MB", "+2.4%", false, "cpu")}
                </div>

                // Middle section: Interactive Chart & Realtime Insights
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    // SVG Visual Chart
                    <div class="lg:col-span-2 p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <h3 class="font-bold text-base text-zinc-900 dark:text-white">"Throughput & Response Times"</h3>
                                <p class="text-xs text-zinc-400">"Requests processed per millisecond across edge nodes"</p>
                            </div>
                            <span class="px-2.5 py-1 rounded-lg bg-zinc-100 dark:bg-zinc-800 text-xs font-mono text-zinc-600 dark:text-zinc-300">
                                "Last 7 Days"
                            </span>
                        </div>

                        // SVG Bar Chart Visual
                        <div class="h-48 w-full flex items-end gap-3 pt-6 pb-2 px-2 border-b border-zinc-100 dark:border-zinc-800/80">
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/20 hover:bg-orange-500/30 rounded-t-lg transition-all" style="height: 45%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Mon"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/20 hover:bg-orange-500/30 rounded-t-lg transition-all" style="height: 65%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Tue"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/20 hover:bg-orange-500/30 rounded-t-lg transition-all" style="height: 55%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Wed"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/30 hover:bg-orange-500/40 rounded-t-lg transition-all" style="height: 80%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Thu"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/40 hover:bg-orange-500/50 rounded-t-lg transition-all" style="height: 70%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Fri"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-orange-500/30 hover:bg-orange-500/40 rounded-t-lg transition-all" style="height: 90%;"></div>
                                <span class="text-[10px] font-mono text-zinc-400">"Sat"</span>
                            </div>
                            <div class="flex-1 flex flex-col items-center gap-2 h-full justify-end">
                                <div class="w-full bg-gradient-to-t from-orange-600 to-amber-400 rounded-t-lg shadow-sm" style="height: 100%;"></div>
                                <span class="text-[10px] font-mono font-bold text-orange-500">"Sun"</span>
                            </div>
                        </div>

                        <div class="flex items-center justify-between text-xs text-zinc-500 pt-2">
                            <span class="flex items-center gap-1.5">
                                <span class="w-2 h-2 rounded-full bg-orange-500"></span>
                                <span>"99.998% Uptime SLA achieved"</span>
                            </span>
                            <span class="font-mono">"Peak: 42,500 req/s"</span>
                        </div>
                    </div>

                    // Quick System Diagnostics
                    <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-4 flex flex-col justify-between">
                        <div>
                            <h3 class="font-bold text-base text-zinc-900 dark:text-white">"Framework Runtime"</h3>
                            <p class="text-xs text-zinc-400">"Zero Node.js dependency runtime stats"</p>
                        </div>

                        <div class="space-y-3 text-xs font-mono">
                            <div class="p-3 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex items-center justify-between">
                                <span class="text-zinc-500">"HTTP Server"</span>
                                <span class="text-zinc-900 dark:text-zinc-100 font-bold">"Axum 0.8 / Hyper 1"</span>
                            </div>
                            <div class="p-3 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex items-center justify-between">
                                <span class="text-zinc-500">"Tailwind Compiler"</span>
                                <span class="text-emerald-600 dark:text-emerald-400 font-bold">"Standalone CLI v3.4"</span>
                            </div>
                            <div class="p-3 rounded-xl bg-zinc-50 dark:bg-zinc-950/60 border border-zinc-200/80 dark:border-zinc-800/80 flex items-center justify-between">
                                <span class="text-zinc-500">"Compression"</span>
                                <span class="text-zinc-900 dark:text-zinc-100 font-bold">"Brotli + Gzip"</span>
                            </div>
                        </div>

                        <a
                            href="/components"
                            class="w-full py-2.5 rounded-xl border border-orange-500/20 bg-orange-500/10 hover:bg-orange-500/20 text-orange-600 dark:text-orange-400 text-xs font-semibold flex items-center justify-center gap-2 transition-colors"
                        >
                            <span>"Explore 40+ UI Components"</span>
                            {render_icon("arrow-right", IconSize::Sm, None, None)}
                        </a>
                    </div>
                </div>

                // Recent Activity Data Table
                <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-4">
                    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div>
                            <h3 class="font-bold text-base text-zinc-900 dark:text-white">"Recent Tenant Provisioning"</h3>
                            <p class="text-xs text-zinc-400">"Latest deployments orchestrated across your cluster"</p>
                        </div>
                        <span class="text-xs font-mono text-zinc-400">"Showing 5 of 128 tenants"</span>
                    </div>

                    <div class="overflow-x-auto">
                        <table class="w-full text-left border-collapse">
                            <thead>
                                <tr class="border-b border-zinc-200 dark:border-zinc-800 text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">
                                    <th class="py-3 px-4">"Customer"</th>
                                    <th class="py-3 px-4">"Plan"</th>
                                    <th class="py-3 px-4">"Status"</th>
                                    <th class="py-3 px-4 text-right">"ARR"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {table_row("Sarah Connor", "sarah@cyberdyne.io", "Enterprise Scale", "Active", "$48,000")}
                                {table_row("Marcus Vance", "marcus@aerodyne.de", "Team Dedicated", "Active", "$12,400")}
                                {table_row("Elena Rostova", "elena@vostok.space", "Growth Cluster", "Pending", "$6,800")}
                                {table_row("David Zhao", "d.zhao@neo-tokyo.jp", "Enterprise Scale", "Active", "$36,000")}
                                {table_row("Claire Dupont", "c.dupont@lumina.fr", "Developer Pro", "Active", "$2,400")}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
