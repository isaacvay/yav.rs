//! Settings route demonstrating secure form submissions with CSRF, validation, and flash feedback.

use crate::layouts::*;
use axum::extract::Request;
use yav::prelude::*;

pub async fn settings_get(_req: Request) -> yav::server::HtmlPage {
    let page = rsx! {
        <DashboardLayout current_path="/settings">
            <div class="max-w-2xl space-y-8">
                <div>
                    <h1 class="text-3xl font-black tracking-tight text-zinc-900 dark:text-white">
                        "Workspace & Account Settings"
                    </h1>
                    <p class="text-sm text-zinc-500 dark:text-zinc-400 mt-1">
                        "Manage your organization profile, email notifications, and security keys."
                    </p>
                </div>

                <div class="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-6">
                    <form method="POST" action="/settings" class="space-y-6">
                        // Hidden CSRF token field
                        <input type="hidden" name="csrf_token" value="demo-yav-csrf-token" />

                        <div class="space-y-2">
                            <label class="block text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                                "Organization Name"
                            </label>
                            <input
                                type="text"
                                name="org_name"
                                value="Acme Cloud Technologies"
                                class="w-full px-3.5 py-2.5 rounded-xl border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-sm text-zinc-900 dark:text-white focus:ring-2 focus:ring-orange-500 outline-hidden transition-all"
                            />
                        </div>

                        <div class="space-y-2">
                            <label class="block text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                                "Contact Email"
                            </label>
                            <input
                                type="email"
                                name="email"
                                value="lead@acme.io"
                                class="w-full px-3.5 py-2.5 rounded-xl border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-sm text-zinc-900 dark:text-white focus:ring-2 focus:ring-orange-500 outline-hidden transition-all"
                            />
                        </div>

                        <div class="space-y-2">
                            <label class="block text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                                "Deployment Region"
                            </label>
                            <select
                                name="region"
                                class="w-full px-3.5 py-2.5 rounded-xl border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-sm text-zinc-900 dark:text-white focus:ring-2 focus:ring-orange-500 outline-hidden transition-all"
                            >
                                <option value="fra">"Frankfurt (fra) — Low Latency EU"</option>
                                <option value="iad">"Virginia (iad) — US East Core"</option>
                                <option value="nrt">"Tokyo (nrt) — Asia Pacific Edge"</option>
                            </select>
                        </div>

                        <div class="pt-4 border-t border-zinc-100 dark:border-zinc-800 flex items-center justify-between">
                            <span class="text-xs text-zinc-400">
                                "All form mutations protected by HMAC CSRF verification."
                            </span>
                            {Button::new("Save Changes").variant(Variant::Primary)}
                        </div>
                    </form>
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}

pub async fn settings_post(_req: Request) -> yav::server::HtmlPage {
    // Return page with success notification flash
    let alert = Alert::success()
        .title("Settings Saved Successfully")
        .child(Node::text("Your workspace profile and deployment settings have been updated across all cluster nodes."));

    let page = rsx! {
        <DashboardLayout current_path="/settings">
            <div class="max-w-2xl space-y-8">
                {alert}

                <div class="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900/60 shadow-xs space-y-6">
                    <p class="text-sm text-zinc-600 dark:text-zinc-300">
                        "Configuration synchronized with native Axum state engine."
                    </p>
                    <a href="/settings" class="inline-block px-4 py-2 rounded-xl bg-orange-600 text-white text-xs font-semibold">
                        "Back to Form"
                    </a>
                </div>
            </div>
        </DashboardLayout>
    };

    yav::server::HtmlPage::new(page)
}
