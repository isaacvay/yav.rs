//! Progressive Web App (PWA) scaffolding command (`yav pwa`).
//!
//! Generates `public/manifest.json`, `public/sw.js` (Service Worker),
//! `public/offline.html`, and scalable PWA icons.

use std::fs;
use std::path::Path;
use yav_build::manifest::PackageYav;

/// Executes the `yav pwa` command.
pub fn run(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(".");
    let public_dir = root.join("public");
    fs::create_dir_all(&public_dir)?;

    let manifest = PackageYav::discover_in(root).unwrap_or_default();
    let app_name = if !manifest.pwa.name.is_empty() {
        manifest.pwa.name.clone()
    } else {
        manifest.package.name.clone()
    };
    let short_name = if !manifest.pwa.short_name.is_empty() {
        manifest.pwa.short_name.clone()
    } else {
        app_name.clone()
    };
    let theme_color = manifest.pwa.theme_color;
    let bg_color = manifest.pwa.background_color;
    let display = manifest.pwa.display;
    let start_url = manifest.pwa.start_url;

    // 1. Generate public/manifest.json
    let manifest_path = public_dir.join("manifest.json");
    if manifest_path.exists() && !force {
        println!("ℹ️  `public/manifest.json` already exists (use --force to overwrite).");
    } else {
        let manifest_json = format!(
            r#"{{
  "name": "{app_name}",
  "short_name": "{short_name}",
  "description": "{description}",
  "start_url": "{start_url}",
  "display": "{display}",
  "background_color": "{bg_color}",
  "theme_color": "{theme_color}",
  "orientation": "any",
  "icons": [
    {{
      "src": "/public/icon-192.svg",
      "sizes": "192x192",
      "type": "image/svg+xml",
      "purpose": "any maskable"
    }},
    {{
      "src": "/public/icon-512.svg",
      "sizes": "512x512",
      "type": "image/svg+xml",
      "purpose": "any maskable"
    }}
  ]
}}
"#,
            description = manifest.package.description
        );
        fs::write(&manifest_path, manifest_json)?;
        println!("✔ Generated \x1b[1;32mpublic/manifest.json\x1b[0m");
    }

    // 2. Generate public/sw.js
    let sw_path = public_dir.join("sw.js");
    if sw_path.exists() && !force {
        println!("ℹ️  `public/sw.js` already exists (use --force to overwrite).");
    } else {
        let sw_js = r#"// Yav Progressive Web App Service Worker
const CACHE_NAME = 'yav-pwa-v1';
const PRECACHE_ASSETS = [
  '/',
  '/public/styles.css',
  '/public/manifest.json',
  '/public/offline.html',
  '/public/favicon.ico'
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE_ASSETS)).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      );
    }).then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  const url = new URL(req.url);

  // HTML page navigation: Network-first with offline fallback
  if (req.mode === 'navigate') {
    event.respondWith(
      fetch(req).catch(() => caches.match('/public/offline.html'))
    );
    return;
  }

  // Static assets (CSS, images, icons, fonts): Cache-first with network fallback
  if (url.pathname.startsWith('/public/') || req.destination === 'image' || req.destination === 'style' || req.destination === 'font') {
    event.respondWith(
      caches.match(req).then((cached) => {
        if (cached) return cached;
        return fetch(req).then((res) => {
          if (!res || res.status !== 200 || res.type !== 'basic') return res;
          const clone = res.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(req, clone));
          return res;
        });
      })
    );
    return;
  }

  // Default network fetch
  event.respondWith(fetch(req));
});
"#;
        fs::write(&sw_path, sw_js)?;
        println!("✔ Generated \x1b[1;32mpublic/sw.js\x1b[0m (Service Worker)");
    }

    // 3. Generate public/offline.html
    let offline_path = public_dir.join("offline.html");
    if offline_path.exists() && !force {
        println!("ℹ️  `public/offline.html` already exists (use --force to overwrite).");
    } else {
        let offline_html = format!(
            r#"<!DOCTYPE html>
<html lang="fr" class="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Mode Hors-Ligne - {app_name}</title>
  <style>
    body {{
      margin: 0;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #090d16;
      color: #f1f5f9;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      text-align: center;
      padding: 1.5rem;
    }}
    .card {{
      max-width: 440px;
      padding: 2.5rem 2rem;
      border-radius: 1.25rem;
      background: #111827;
      border: 1px solid #1f2937;
      box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
    }}
    .icon {{
      width: 56px;
      height: 56px;
      margin: 0 auto 1.5rem;
      color: #60a5fa;
    }}
    h1 {{
      font-size: 1.5rem;
      font-weight: 700;
      margin: 0 0 0.75rem;
    }}
    p {{
      color: #94a3b8;
      font-size: 0.95rem;
      line-height: 1.5;
      margin: 0 0 1.75rem;
    }}
    button {{
      background: #2563eb;
      color: white;
      border: none;
      padding: 0.75rem 1.75rem;
      font-size: 0.95rem;
      font-weight: 600;
      border-radius: 0.75rem;
      cursor: pointer;
      transition: background 0.2s;
    }}
    button:hover {{
      background: #1d4ed8;
    }}
  </style>
</head>
<body>
  <div class="card">
    <svg class="icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 2.829a4.978 4.978 0 01-1.414-2.83m-1.414 5.658a9 9 0 01-2.167-9.238m7.824 2.166a1 1 0 111.414 1.414M3 3l18 18" />
    </svg>
    <h1>Connexion Interrompue</h1>
    <p>Vous êtes actuellement hors-ligne. Vérifiez votre connexion Internet et réessayez.</p>
    <button onclick="window.location.reload()">Réessayer la connexion</button>
  </div>
</body>
</html>
"#
        );
        fs::write(&offline_path, offline_html)?;
        println!("✔ Generated \x1b[1;32mpublic/offline.html\x1b[0m (Offline Fallback)");
    }

    // 4. Generate public/icon-192.svg and public/icon-512.svg
    let icon_192_path = public_dir.join("icon-192.svg");
    let icon_512_path = public_dir.join("icon-512.svg");
    let svg_icon = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="108" fill="{bg_color}"/>
  <path d="M140 160 L256 340 L372 160" stroke="#60a5fa" stroke-width="48" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
  <circle cx="256" cy="380" r="24" fill="#38bdf8"/>
</svg>"##
    );
    if !icon_192_path.exists() || force {
        fs::write(&icon_192_path, &svg_icon)?;
        println!("✔ Generated \x1b[1;32mpublic/icon-192.svg\x1b[0m");
    }
    if !icon_512_path.exists() || force {
        fs::write(&icon_512_path, &svg_icon)?;
        println!("✔ Generated \x1b[1;32mpublic/icon-512.svg\x1b[0m");
    }

    println!();
    println!("🎉 \x1b[1;32mPWA assets generated successfully!\x1b[0m");
    println!("────────────────────────────────────────────────────────────");
    println!(
        "To activate the PWA in your application, add the following to your HTML <head> or layout:"
    );
    println!();
    println!("  \x1b[36m<link rel=\"manifest\" href=\"/public/manifest.json\" />\x1b[0m");
    println!(
        "  \x1b[36m<meta name=\"theme-color\" content=\"{}\" />\x1b[0m",
        theme_color
    );
    println!("  \x1b[36m<script>\x1b[0m");
    println!("  \x1b[36m  if ('serviceWorker' in navigator) {{\x1b[0m");
    println!("  \x1b[36m    window.addEventListener('load', () => {{\x1b[0m");
    println!("  \x1b[36m      navigator.serviceWorker.register('/sw.js');\x1b[0m");
    println!("  \x1b[36m    }});\x1b[0m");
    println!("  \x1b[36m  }}\x1b[0m");
    println!("  \x1b[36m</script>\x1b[0m");
    println!();

    Ok(())
}
