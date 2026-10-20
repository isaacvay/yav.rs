# SEO & Metadata Engine

Yav includes a built-in search engine optimization (SEO) and social metadata engine.

---

## 1. Page Metadata & OpenGraph (`PageMeta`)

Configure rich metadata on your pages with builder methods:

```rust
use yav::prelude::*;

pub async fn blog_post() -> HtmlPage {
    let meta = PageMeta::new()
        .title("Comprendre le SSR avec Yav")
        .description("Guide complet sur l'architecture et les performances du framework Yav.")
        .canonical("https://exemple.com/blog/ssr-yav")
        .favicon("/favicon.ico")
        .open_graph(
            "Comprendre le SSR avec Yav",
            "Guide complet sur l'architecture et les performances du framework Yav.",
            "https://exemple.com/og-image.jpg"
        )
        .twitter_card("summary_large_image")
        .json_ld(r#"{
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Comprendre le SSR avec Yav"
        }"#);

    let content = rsx! {
        <article class="max-w-2xl mx-auto py-8">
            <h1 class="text-3xl font-bold">Comprendre le SSR avec Yav</h1>
            <p class="mt-4">Article complet...</p>
        </article>
    };

    HtmlPage::from_meta(&meta, content)
}
```

This automatically generates complete HTML `<head>` tags:
- `<meta name="description" ...>`
- `<link rel="canonical" ...>`
- `<link rel="icon" ...>`
- `<meta property="og:title" ...>`
- `<meta property="og:description" ...>`
- `<meta property="og:image" ...>`
- `<meta property="og:type" content="website" />`
- `<meta name="twitter:card" content="summary_large_image" />`
- `<script type="application/ld+json">...</script>`

---

## 2. Dynamic Sitemap XML (`sitemap.xml`)

Generate standard XML sitemaps for Google, Bing, and other search engine crawlers:

```rust
use yav::prelude::*;
use yav_core::{Sitemap, SitemapEntry};

pub async fn sitemap_handler() -> Response {
    let sitemap = Sitemap::new()
        .add_entry(SitemapEntry {
            loc: "https://exemple.com/".into(),
            changefreq: Some("daily".into()),
            priority: Some(1.0),
            ..Default::default()
        })
        .add_entry(SitemapEntry {
            loc: "https://exemple.com/about".into(),
            changefreq: Some("monthly".into()),
            priority: Some(0.8),
            ..Default::default()
        })
        .add_url("https://exemple.com/components");

    sitemap_response(&sitemap)
}
```

Register in your `YavApp`:
```rust
app.get("/sitemap.xml", sitemap_handler);
```

---

## 3. Dynamic `robots.txt`

```rust
use yav::prelude::*;
use yav_core::RobotsTxt;

pub async fn robots_handler() -> Response {
    let robots = RobotsTxt::new()
        .disallow("/admin")
        .disallow("/api/private")
        .sitemap("https://exemple.com/sitemap.xml");

    robots_response(&robots)
}
```

Register in your `YavApp`:
```rust
app.get("/robots.txt", robots_handler);
```

---

## 4. Internationalization (i18n)

Yav provides a zero-dependency translation and locale resolution engine:

```rust
use yav::prelude::*;

// 1. Configure translations catalog
let i18n = I18n::new("en")
    .supported_locales(["en", "fr", "es"])
    .add_translation("en", "welcome", "Welcome, {name}!")
    .add_translation("fr", "welcome", "Bienvenue, {name} !");

// 2. Resolve preferred locale from user headers or cookies
let locale = i18n.resolve_locale(Some("fr-FR,fr;q=0.9,en;q=0.8"), None);
assert_eq!(locale, "fr");

// 3. Translate with parameter interpolation
let greeting = i18n.translate(&locale, "welcome", &[("name", "Alice")]);
assert_eq!(greeting, "Bienvenue, Alice !");
```

---

## 5. Flash Messages & Server Actions

Cookie-backed one-time notifications that persist across redirects:

```rust
use yav::prelude::*;

pub async fn handle_checkout(flash: Flash) -> ActionResult {
    // Process order...
    flash.success("Votre commande #1234 a été confirmée !");
    ActionResponse::redirect("/orders")
}

pub async fn orders_page(flash: Flash) -> HtmlPage {
    rsx! {
        <ToastContainer position="bottom-right">
            {flash.incoming().iter().map(|msg| {
                rsx! {
                    <Toast 
                        level={match msg.level {
                            FlashLevel::Success => ToastLevel::Success,
                            FlashLevel::Error => ToastLevel::Error,
                            FlashLevel::Warning => ToastLevel::Warning,
                            FlashLevel::Info => ToastLevel::Info,
                        }} 
                        message={msg.message.clone()} 
                    />
                }
            })}
        </ToastContainer>
    }
}
```

---

## 6. Production Health Checks & Observability

Kubernetes-ready probes and in-memory SSR telemetry:

```rust
use yav::prelude::*;

let app = YavApp::new()
    .get("/healthz", healthz)
    .get("/readyz", readyz);
```

* `/healthz`: returns HTTP 200 `{"status": "ok", "service": "yav", "version": "0.1.0"}`.
* `/readyz`: returns HTTP 200 `"ready"`.
* `ServerMetrics`: tracks total requests, errors, and average SSR rendering latencies in milliseconds.
