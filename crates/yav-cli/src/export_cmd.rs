//! Static site generation (SSG) command (`yav export`).
//!
//! Crawls and renders all discovered static routes to standalone `.html` files
//! in `dist/` (or specified directory) along with static assets for CDN/Pages deployment.
//!
//! Automatically handles:
//! - JIT Tailwind CSS compilation before asset copy
//! - Stale `asset-manifest.json` cleanup
//! - Asset mirroring to both root (`dist/`) and `/public/` (`dist/public/`)
//! - Auto-generation of CDN security headers (`_headers`) and fallback rewrites (`_redirects`)
//! - Pre-rendering of all discovered static routes via SSR engine

use std::{fs, path::Path, time::Instant};
use yav_build::{PackageYav, TailwindManager};
use yav_router::FileSystemRouter;

/// Tracks the outcome of the SSR export subprocess.
enum SsrExportStatus {
    /// No SSR build was attempted (no project files or running in test mode).
    NotAttempted,
    /// `cargo run` succeeded.
    Succeeded,
    /// `cargo run` ran but returned a non-zero exit code.
    Failed(Option<i32>),
    /// `cargo run` could not be launched at all.
    LaunchFailed(String),
}

/// Runs the static site export command.
pub async fn run(output_dir: &str, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n  \x1b[1;36myav export\x1b[0m \x1b[90m— Static Site Generator (SSG)\x1b[0m\n");

    let start_time = Instant::now();
    let out_path = Path::new(output_dir);
    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }

    println!(
        "  Target output directory: \x1b[1m{}\x1b[0m",
        out_path.display()
    );

    // 0. JIT Tailwind compilation if enabled in package.yavml
    if let Ok(manifest) = PackageYav::discover_in(Path::new(".")) {
        if manifest.tailwind.enabled {
            let input_css = Path::new(&manifest.tailwind.input);
            let output_css = Path::new(&manifest.tailwind.output);
            if input_css.is_file() {
                println!(
                    "  Compiling Tailwind CSS JIT (v{})...",
                    manifest.tailwind.version
                );
                let mgr = TailwindManager::new(&manifest.tailwind.version);
                let content_paths = vec![
                    Path::new("src").to_path_buf(),
                    Path::new("public").to_path_buf(),
                ];
                match mgr
                    .run_build(input_css, output_css, &content_paths, true)
                    .await
                {
                    Ok(()) => println!(
                        "    \x1b[32m[OK]\x1b[0m Compiled Tailwind CSS ➜ {}",
                        output_css.display()
                    ),
                    Err(e) => eprintln!(
                        "    \x1b[33m! Warning: Tailwind JIT compilation failed: {e}\x1b[0m"
                    ),
                }
            }
        }
    }

    // 1. Purge stale asset-manifest.json to prevent SSR engine from resolving obsolete hashes
    let stale_manifest = out_path.join("asset-manifest.json");
    if stale_manifest.is_file() {
        let _ = fs::remove_file(&stale_manifest);
    }

    // 2. Copy and mirror public static assets into output_dir and output_dir/public
    let public_dir = Path::new("public");
    let mut copied_assets = 0;
    if public_dir.exists() && public_dir.is_dir() {
        println!("  Copying and mirroring public assets...");
        // Copy to root: dist/ (allows /styles.css, /favicon.ico)
        copied_assets = copy_dir_all(public_dir, out_path)?;

        // Mirror to subfolder: dist/public/ (guarantees /public/* URLs resolve statically on CDNs)
        let target_public = out_path.join("public");
        if !target_public.exists() {
            fs::create_dir_all(&target_public)?;
        }
        let _ = copy_dir_all(public_dir, &target_public)?;
        println!(
            "    \x1b[32m[OK]\x1b[0m Copied {copied_assets} static asset files (mirrored to root & /public/)"
        );
    }

    // 3. Scan routes
    let routes_dir = Path::new("src/routes");
    let router = if routes_dir.exists() {
        FileSystemRouter::scan_dir(routes_dir)?
    } else {
        FileSystemRouter::new()
    };

    let all_routes = router.routes();
    let mut static_routes: Vec<String> = all_routes
        .iter()
        .map(|r| r.path_pattern.clone())
        .filter(|p| !p.contains(':') && !p.contains('*') && *p != "/health")
        .collect();

    if static_routes.is_empty() {
        static_routes.push("/".to_string());
    }
    static_routes.sort();
    static_routes.dedup();

    println!(
        "  Discovered {} static route(s) to prerender:",
        static_routes.len()
    );
    for route in &static_routes {
        println!("    - {}", route);
    }

    // 4. If inside an executable Yav application project, run SSR export via YAV_EXPORT_DIR
    let mut ssr_status: SsrExportStatus = SsrExportStatus::NotAttempted;
    if !cfg!(test)
        && Path::new("package.yavml").is_file()
        && Path::new("Cargo.toml").is_file()
        && Path::new("src/main.rs").is_file()
    {
        println!("  Rendering dynamic RSX routes via SSR export engine...");
        match tokio::process::Command::new("cargo")
            .arg("run")
            .arg("--quiet")
            .env("YAV_EXPORT_DIR", out_path.as_os_str())
            .env("YAV_EXPORT_BASE_URL", base_url)
            .status()
            .await
        {
            Ok(status) if status.success() => {
                ssr_status = SsrExportStatus::Succeeded;
            }
            Ok(status) => {
                ssr_status = SsrExportStatus::Failed(status.code());
            }
            Err(e) => {
                ssr_status = SsrExportStatus::LaunchFailed(e.to_string());
            }
        }
    }

    // 5. Verify SSR output for each discovered static route.
    //    If SSR was attempted and failed, do NOT silently write placeholder
    //    HTML — report the failure honestly and return an error.
    let mut exported_count = 0;
    let mut missing_routes: Vec<String> = Vec::new();

    for route in &static_routes {
        let rel_file = if route == "/" {
            out_path.join("index.html")
        } else {
            let clean = route.trim_start_matches('/');
            let folder = out_path.join(clean);
            if !folder.exists() {
                fs::create_dir_all(&folder)?;
            }
            folder.join("index.html")
        };

        if rel_file.exists() {
            // SSR produced this file — verify it's not empty
            if let Ok(meta) = fs::metadata(&rel_file) {
                if meta.len() > 0 {
                    exported_count += 1;
                    println!(
                        "    \x1b[32m✔\x1b[0m Prerendered {} ➜ {}",
                        route,
                        rel_file.display()
                    );
                    continue;
                }
            }
        }

        // File is missing or empty
        match &ssr_status {
            SsrExportStatus::Failed(code) => {
                missing_routes.push(route.clone());
                eprintln!(
                    "    \x1b[31m✗\x1b[0m SSR failed to render {} (cargo run exit code: {:?})",
                    route, code
                );
            }
            SsrExportStatus::LaunchFailed(msg) => {
                missing_routes.push(route.clone());
                eprintln!(
                    "    \x1b[31m✗\x1b[0m Could not launch SSR for {}: {}",
                    route, msg
                );
            }
            SsrExportStatus::NotAttempted => {
                // No SSR expected (no project files or running in test mode).
                // Write a minimal honest placeholder so the export directory is
                // structurally complete, but clearly label it.
                let clean_title = if route == "/" {
                    "Home"
                } else {
                    route.trim_start_matches('/')
                };
                let raw_title = format!("{} — Yav Application", clean_title.replace('/', " — "));
                let title = yav_core::escape_html(&raw_title);
                let body = format!(
                    r#"<main class="min-h-screen bg-slate-950 text-white flex flex-col items-center justify-center p-8">
  <div class="max-w-xl text-center space-y-4">
    <h1 class="text-3xl font-bold tracking-tight">{}</h1>
    <p class="text-slate-400">This page was generated by yav export without an SSR build. Configure src/main.rs with YAV_EXPORT_DIR support for full prerendering.</p>
    <a href="/" class="inline-block mt-4 px-4 py-2 bg-indigo-600 rounded-lg text-sm font-semibold hover:bg-indigo-500">Back Home</a>
  </div>
</main>"#,
                    yav_core::escape_html(clean_title)
                );
                let html =
                    yav_core::document_shell(&title, "", &body, yav_core::YAV_MICRO_ACTIONS_SCRIPT);
                let minified = yav_build::minify_html(&html);
                fs::write(&rel_file, minified)?;
                exported_count += 1;
                println!(
                    "    \x1b[33m~\x1b[0m Placeholder {} ➜ {} (no SSR build configured)",
                    route,
                    rel_file.display()
                );
            }
            SsrExportStatus::Succeeded => {
                // SSR succeeded but this route file is still missing — treat as
                // a partial failure so the operator knows to investigate.
                missing_routes.push(route.clone());
                eprintln!(
                    "    \x1b[31m✗\x1b[0m SSR succeeded but route {} was not produced by the engine",
                    route
                );
            }
        }
    }

    // If SSR was attempted but failed, report honestly and exit with error.
    if !missing_routes.is_empty() {
        let duration = start_time.elapsed();
        eprintln!(
            "\n\x1b[1;31m✗ Static site export FAILED in {:.2?}\x1b[0m",
            duration
        );
        eprintln!(
            "  • {} route(s) could not be rendered due to SSR failure",
            missing_routes.len()
        );
        eprintln!("  • Routes: {}", missing_routes.join(", "));
        eprintln!("  • Partial output in: {}", out_path.display());
        return Err(format!(
            "SSR export failed for {} route(s): {}",
            missing_routes.len(),
            missing_routes.join(", ")
        )
        .into());
    }

    // 6. Generate sitemap and robots.txt in export directory
    let sitemap_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{}
</urlset>
"#,
        static_routes
            .iter()
            .map(|r| {
                let full = format!("{}{}", base_url.trim_end_matches('/'), r);
                let escaped = yav_core::escape_html(&full);
                format!("  <url><loc>{escaped}</loc><priority>0.8</priority></url>")
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    fs::write(out_path.join("sitemap.xml"), sitemap_xml)?;

    let robots_txt = format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n",
        base_url.trim_end_matches('/')
    );
    fs::write(out_path.join("robots.txt"), robots_txt)?;

    // 7. Generate Cloudflare Pages & Static CDN configuration (_headers and _redirects)
    let headers_file = out_path.join("_headers");
    if !headers_file.exists() {
        let default_headers = r#"# Cloudflare Pages Custom Security & Cache Headers
/*
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin

/*.css
  Cache-Control: public, max-age=31536000, immutable
  Content-Type: text/css; charset=utf-8

/public/*.css
  Cache-Control: public, max-age=31536000, immutable
  Content-Type: text/css; charset=utf-8

/*.js
  Cache-Control: public, max-age=31536000, immutable

/public/*.js
  Cache-Control: public, max-age=31536000, immutable

/*.svg
  Cache-Control: public, max-age=31536000, immutable

/public/*.svg
  Cache-Control: public, max-age=31536000, immutable

/*.png
  Cache-Control: public, max-age=31536000, immutable

/public/*.png
  Cache-Control: public, max-age=31536000, immutable

/*.html
  Cache-Control: public, max-age=0, must-revalidate
"#;
        fs::write(&headers_file, default_headers)?;
        if let Err(e) = fs::write(out_path.join("public").join("_headers"), default_headers) {
            tracing::warn!("Could not write public/_headers: {e}");
        }
    }

    let redirects_file = out_path.join("_redirects");
    if !redirects_file.exists() {
        let default_redirects = r#"# Cloudflare Pages / Static CDN Fallback Redirects
/public/*  /:splat  200
"#;
        fs::write(&redirects_file, default_redirects)?;
        if let Err(e) = fs::write(
            out_path.join("public").join("_redirects"),
            default_redirects,
        ) {
            tracing::warn!("Could not write public/_redirects: {e}");
        }
    }

    let duration = start_time.elapsed();
    let status_label = match &ssr_status {
        SsrExportStatus::Succeeded => "✔ Static site export completed",
        SsrExportStatus::NotAttempted => "✔ Static site export completed (no SSR build)",
        SsrExportStatus::Failed(_) => "✗ Static site export completed with errors",
        SsrExportStatus::LaunchFailed(_) => "✗ Static site export completed with errors",
    };
    println!("\n\x1b[1;32m{} in {:.2?}\x1b[0m", status_label, duration);
    println!("  • Prerendered HTML pages : {exported_count}");
    println!("  • Static assets copied   : {copied_assets}");
    println!("  • Export directory       : {}", out_path.display());
    if matches!(ssr_status, SsrExportStatus::NotAttempted) {
        println!("\n\x1b[1mNote: No SSR build was detected. Pages are placeholders.\x1b[0m");
        println!("  To enable full prerendering, ensure src/main.rs handles YAV_EXPORT_DIR.");
    } else {
        println!("\n\x1b[1mReady for deployment to GitHub Pages, Cloudflare Pages, Vercel, or AWS S3!\x1b[0m");
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            if !target.exists() {
                fs::create_dir_all(&target)?;
            }
            count += copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_export_cmd_run() {
        let temp_dir = std::env::temp_dir().join(format!("yav_export_test_{}", std::process::id()));
        let res = run(temp_dir.to_str().unwrap(), "https://example.com").await;
        assert!(res.is_ok());

        assert!(temp_dir.join("index.html").exists());
        assert!(temp_dir.join("sitemap.xml").exists());
        assert!(temp_dir.join("robots.txt").exists());
        assert!(temp_dir.join("_headers").exists());
        assert!(temp_dir.join("_redirects").exists());

        let sitemap = fs::read_to_string(temp_dir.join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("https://example.com"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
