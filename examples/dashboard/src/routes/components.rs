//! Component Gallery showcase demonstrating 40+ Yav UI primitives.

use crate::layouts::*;
use axum::extract::Request;
use yav::prelude::*;

pub async fn components(_req: Request) -> yav::server::HtmlPage {
    let section_header = |title: &'static str, desc: &'static str| -> Node {
        rsx! {
            <div class="space-y-1 pb-2 border-b border-zinc-200 dark:border-zinc-800">
                <h2 class="text-lg font-bold text-zinc-900 dark:text-white tracking-tight">{title}</h2>
                <p class="text-xs text-zinc-500 dark:text-zinc-400">{desc}</p>
            </div>
        }
    };

    let page = rsx! {
        <DashboardLayout current_path="/components">
            <div class="space-y-10">
                // Header
                <div class="space-y-2">
                    <div class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-orange-50 dark:bg-orange-950/40 border border-orange-200 dark:border-orange-800/60 text-xs font-mono text-orange-600 dark:text-orange-400">
                        {render_icon("layers", IconSize::Sm, None, None)}
                        <span>"yav-ui • 40+ Accessible UI Primitives"</span>
                    </div>
                    <h1 class="text-3xl font-black tracking-tight text-zinc-900 dark:text-white">
                        "Component Showroom"
                    </h1>
                    <p class="text-sm text-zinc-500 dark:text-zinc-400 max-w-2xl leading-relaxed">
                        "Pre-designed, fully accessible, and composable UI primitives styled with pure Tailwind CSS. Zero external JavaScript runtime required."
                    </p>
                </div>

                // 1. Buttons & Variants
                <div class="space-y-4">
                    {section_header("Buttons & Actions", "Diverse variants, sizes, and icon integrations")}
                    <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 space-y-6">
                        <div class="flex flex-wrap items-center gap-3">
                            {Button::new("Primary Action").variant(Variant::Primary)}
                            {Button::new("Secondary Action").variant(Variant::Secondary)}
                            {Button::new("Outline Border").variant(Variant::Outline)}
                            {Button::new("Ghost Button").variant(Variant::Ghost)}
                            {Button::new("Destructive").variant(Variant::Danger)}
                        </div>

                        <div class="flex flex-wrap items-center gap-3 pt-3 border-t border-zinc-100 dark:border-zinc-800/60">
                            {Button::new("Small").size(Size::Sm).variant(Variant::Primary)}
                            {Button::new("Medium (Default)").size(Size::Md).variant(Variant::Primary)}
                            {Button::new("Large Action").size(Size::Lg).variant(Variant::Primary)}
                        </div>
                    </div>
                </div>

                // 2. Badges & Indicators
                <div class="space-y-4">
                    {section_header("Status Badges & Chips", "Contextual signals for tenant health and deployment states")}
                    <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 flex flex-wrap gap-3">
                        {Badge::new("Cluster Active").variant(Variant::Success)}
                        {Badge::new("High Latency Warning").variant(Variant::Warning)}
                        {Badge::new("Service Degradation").variant(Variant::Danger)}
                        {Badge::new("Syncing Database").variant(Variant::Primary)}
                        {Badge::new("Archived Snapshot").variant(Variant::Ghost)}
                    </div>
                </div>

                // 3. Alerts & Callouts
                <div class="space-y-4">
                    {section_header("Feedback Alerts", "Semantic announcements matching ARIA standards")}
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {Alert::info().title("Zero-Copy VDOM Engine").child(Node::text("Yav streams rendered HTML directly using Cow<'static, str> without heap reallocations."))}
                        {Alert::success().title("Production Build Verified").child(Node::text("Standalone Tailwind CLI finished minifying public/styles.css without Node.js."))}
                        {Alert::warning().title("Memory Threshold Notice").child(Node::text("Regional node fra-1 reached 78% of memory quota."))}
                        {Alert::danger().title("Certificate Rotation Required").child(Node::text("TLS cert expires in 48 hours for api.internal domain."))}
                    </div>
                </div>

                // 4. Interactive Micro-Interactions (Counter & Accordion)
                <div class="space-y-4">
                    {section_header("Client Islands & Micro-Interactions", "Reactive state without the heavy client hydration penalty")}
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        // Reactive Island Counter
                        <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 space-y-4">
                            <div class="flex items-center justify-between">
                                <h3 class="font-bold text-sm text-zinc-900 dark:text-white">"State Island Counter"</h3>
                                <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-amber-500/10 text-amber-600 dark:text-amber-400 font-semibold">"Island"</span>
                            </div>
                            <p class="text-xs text-zinc-400">"Interactive client micro-island running vanilla Web API logic."</p>
                            <div class="flex items-center gap-3 pt-2">
                                <button
                                    onclick="const el=document.getElementById('gallery-count');el.innerText=Math.max(0,parseInt(el.innerText)-1);"
                                    class="w-10 h-10 rounded-xl border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-base font-bold flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                >
                                    "-"
                                </button>
                                <div class="px-6 py-2 rounded-xl bg-zinc-100 dark:bg-zinc-800/80 font-mono text-lg font-black min-w-[80px] text-center">
                                    <span id="gallery-count">"10"</span>
                                </div>
                                <button
                                    onclick="const el=document.getElementById('gallery-count');el.innerText=parseInt(el.innerText)+1;"
                                    class="w-10 h-10 rounded-xl border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 font-mono text-base font-bold flex items-center justify-center transition-colors cursor-pointer active:scale-95"
                                >
                                    "+"
                                </button>
                            </div>
                        </div>

                        // Native Details Accordion
                        <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 space-y-3">
                            <div class="flex items-center justify-between">
                                <h3 class="font-bold text-sm text-zinc-900 dark:text-white">"Accessible Accordion"</h3>
                                <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-semibold">"Zero JS"</span>
                            </div>
                            <details class="group border border-zinc-200 dark:border-zinc-800 rounded-xl p-3.5 [&_summary::-webkit-details-marker]:hidden cursor-pointer">
                                <summary class="flex items-center justify-between font-semibold text-xs text-zinc-900 dark:text-zinc-100">
                                    <span>"How does Yav avoid Node.js?"</span>
                                    <span class="transition-transform group-open:rotate-180">
                                        {render_icon("chevron-down", IconSize::Sm, None, None)}
                                    </span>
                                </summary>
                                <p class="mt-2.5 text-xs text-zinc-500 leading-relaxed">
                                    "Yav downloads and validates the official standalone Tailwind CLI executable. Assets and CSS are compiled directly through native Rust process pipes."
                                </p>
                            </details>

                            <details class="group border border-zinc-200 dark:border-zinc-800 rounded-xl p-3.5 [&_summary::-webkit-details-marker]:hidden cursor-pointer">
                                <summary class="flex items-center justify-between font-semibold text-xs text-zinc-900 dark:text-zinc-100">
                                    <span>"Is SSR default?"</span>
                                    <span class="transition-transform group-open:rotate-180">
                                        {render_icon("chevron-down", IconSize::Sm, None, None)}
                                    </span>
                                </summary>
                                <p class="mt-2.5 text-xs text-zinc-500 leading-relaxed">
                                    "Yes! HTML is streamed from Axum 0.8 on the server, ensuring instant First Contentful Paint and flawless SEO without hydration lag."
                                </p>
                            </details>
                        </div>
                    </div>
                </div>

                // 5. Stepper & Progress
                <div class="space-y-4">
                    {section_header("Steppers & Progress Tracks", "Sequential multi-step workflow indicators")}
                    <div class="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 space-y-6">
                        <div class="flex items-center justify-between max-w-xl mx-auto">
                            <div class="flex items-center gap-3">
                                <div class="w-8 h-8 rounded-full bg-emerald-500 text-white flex items-center justify-center font-bold text-xs">
                                    {render_icon("check", IconSize::Sm, None, None)}
                                </div>
                                <span class="text-xs font-semibold text-zinc-900 dark:text-white">"Configure"</span>
                            </div>
                            <div class="h-0.5 flex-1 mx-4 bg-emerald-500"></div>
                            <div class="flex items-center gap-3">
                                <div class="w-8 h-8 rounded-full bg-orange-500 text-white flex items-center justify-center font-bold text-xs">
                                    "2"
                                </div>
                                <span class="text-xs font-semibold text-orange-600 dark:text-orange-400">"Deploy Cluster"</span>
                            </div>
                            <div class="h-0.5 flex-1 mx-4 bg-zinc-200 dark:bg-zinc-800"></div>
                            <div class="flex items-center gap-3">
                                <div class="w-8 h-8 rounded-full bg-zinc-200 dark:bg-zinc-800 text-zinc-400 flex items-center justify-center font-bold text-xs">
                                    "3"
                                </div>
                                <span class="text-xs font-medium text-zinc-400">"Production Verification"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
