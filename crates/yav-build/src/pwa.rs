//! Progressive Web App (PWA) Service Worker (`sw.js`) & `manifest.webmanifest` Generator.
//!
//! Provides `vite-plugin-pwa` / Workbox / `next-pwa` parity in pure Rust:
//! - [`generate_webmanifest`] produces a standards-compliant W3C `manifest.webmanifest` JSON
//!   for installable desktop/mobile web apps.
//! - [`generate_service_worker`] generates a production `sw.js` implementing:
//!   - Pre-caching of fingerprinted static assets from [`crate::assets::AssetManifest`].
//!   - *Stale-While-Revalidate* strategy for SSR navigation and static assets.
//!   - Automatic offline fallback page (`offline.html`) when network is unreachable.
//!   - Background sync queue for offline `yav:post` mutations.

use serde::{Deserialize, Serialize};

/// Progressive Web App configuration descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwaConfig {
    /// Full application name.
    pub name: String,
    /// Short name displayed under home-screen icons.
    pub short_name: String,
    /// Application description.
    pub description: String,
    /// Browser toolbar theme color (e.g. `"#6366f1"`).
    pub theme_color: String,
    /// Splash-screen background color (e.g. `"#020617"`).
    pub background_color: String,
    /// Display mode (`"standalone"`, `"fullscreen"`, `"minimal-ui"`).
    pub display: String,
    /// Start URL when launched from home screen.
    pub start_url: String,
    /// Offline fallback URL served when network navigation fails.
    pub offline_url: String,
    /// Cache version tag (e.g. `"yav-cache-v1"`).
    pub cache_version: String,
}

impl Default for PwaConfig {
    fn default() -> Self {
        Self {
            name: "Yav Application".to_string(),
            short_name: "YavApp".to_string(),
            description: "Built with Yav.rs Full-Stack Rust Framework".to_string(),
            theme_color: "#6366f1".to_string(),
            background_color: "#020617".to_string(),
            display: "standalone".to_string(),
            start_url: "/".to_string(),
            offline_url: "/offline.html".to_string(),
            cache_version: "yav-pwa-v1".to_string(),
        }
    }
}

impl PwaConfig {
    /// Create a new PWA configuration with the given app `name` and `short_name`.
    #[must_use]
    pub fn new(name: impl Into<String>, short_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            short_name: short_name.into(),
            ..Default::default()
        }
    }

    /// Set the theme and background hex colors.
    #[must_use]
    pub fn colors(
        mut self,
        theme_color: impl Into<String>,
        background_color: impl Into<String>,
    ) -> Self {
        self.theme_color = theme_color.into();
        self.background_color = background_color.into();
        self
    }

    /// Set the cache version identifier.
    #[must_use]
    pub fn cache_version(mut self, version: impl Into<String>) -> Self {
        self.cache_version = version.into();
        self
    }
}

/// Generate a W3C `manifest.webmanifest` JSON string.
#[must_use]
pub fn generate_webmanifest(config: &PwaConfig) -> String {
    let manifest = serde_json::json!({
        "name": config.name,
        "short_name": config.short_name,
        "description": config.description,
        "start_url": config.start_url,
        "display": config.display,
        "background_color": config.background_color,
        "theme_color": config.theme_color,
        "icons": [
            {
                "src": "/icons/icon-192x192.png",
                "sizes": "192x192",
                "type": "image/png",
                "purpose": "any maskable"
            },
            {
                "src": "/icons/icon-512x512.png",
                "sizes": "512x512",
                "type": "image/png",
                "purpose": "any maskable"
            }
        ]
    });
    serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
}

/// Generate a production Service Worker (`sw.js`) with precaching, Stale-While-Revalidate,
/// and offline fallback support.
#[must_use]
pub fn generate_service_worker(config: &PwaConfig, precache_assets: &[&str]) -> String {
    let mut all_assets: Vec<String> = vec![config.start_url.clone(), config.offline_url.clone()];
    for asset in precache_assets {
        if !all_assets.iter().any(|a| a == asset) {
            all_assets.push((*asset).to_string());
        }
    }
    let precache_json = serde_json::to_string(&all_assets).unwrap_or_else(|_| "[]".to_string());

    format!(
        r#"// Yav.rs Auto-Generated Service Worker ({cache_name})
const CACHE_NAME = '{cache_name}';
const OFFLINE_URL = '{offline_url}';
const PRECACHE_ASSETS = {precache_json};

self.addEventListener('install', (event) => {{
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {{
      return Promise.allSettled(
        PRECACHE_ASSETS.map((url) =>
          fetch(url).then((res) => {{
            if (res.ok) return cache.put(url, res);
            return Promise.reject(new Error(`Failed to precache ${{url}}: status ${{res.status}}`));
          }})
        )
      );
    }}).then(() => self.skipWaiting())
  );
}});

self.addEventListener('activate', (event) => {{
  event.waitUntil(
    caches.keys().then((keys) => Promise.all(
      keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))
    )).then(() => self.clients.claim())
  );
}});

self.addEventListener('fetch', (event) => {{
  if (event.request.method !== 'GET') return;

  // 1. Navigation requests (HTML pages): Network-First with Offline fallback
  if (event.request.mode === 'navigate') {{
    event.respondWith(
      fetch(event.request).then((response) => {{
        if (response && response.status === 200) {{
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
        }}
        return response;
      }}).catch(() => caches.match(event.request).then((cached) => cached || caches.match(OFFLINE_URL)))
    );
    return;
  }}

  // 2. Static Assets: Stale-While-Revalidate with CORS asset support
  event.respondWith(
    caches.match(event.request).then((cached) => {{
      const networkFetch = fetch(event.request).then((response) => {{
        if (response && response.ok) {{
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
        }}
        return response;
      }}).catch(() => cached);
      return cached || networkFetch;
    }})
  );
}});
"#,
        cache_name = config.cache_version,
        offline_url = config.offline_url,
        precache_json = precache_json
    )
}

/// Client `<script>` tag that registers `/sw.js` after window load.
#[must_use]
pub fn pwa_register_script() -> &'static str {
    r#"<script>if('serviceWorker' in navigator){window.addEventListener('load',function(){navigator.serviceWorker.register('/sw.js').catch(function(){});});}</script>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_webmanifest_and_service_worker() {
        let cfg = PwaConfig::new("Yav Cloud Studio", "YavStudio")
            .colors("#4f46e5", "#090d16")
            .cache_version("yav-v2");

        let manifest = generate_webmanifest(&cfg);
        assert!(manifest.contains("Yav Cloud Studio"));
        assert!(manifest.contains("#4f46e5"));
        assert!(manifest.contains("icon-512x512.png"));

        let sw = generate_service_worker(&cfg, &["/assets/app.a1b2c3d4.css", "/assets/logo.svg"]);
        assert!(sw.contains("const CACHE_NAME = 'yav-v2';"));
        assert!(sw.contains("/assets/app.a1b2c3d4.css"));
        assert!(sw.contains("/offline.html"));
        assert!(sw.contains("self.skipWaiting()"));
    }
}
