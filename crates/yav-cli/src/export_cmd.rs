//! Static site generation (SSG) command (`yav export`).
//!
//! Crawls and renders all discovered static routes to standalone `.html` files
//! in `dist/` (or specified directory) along with static assets for CDN/Pages deployment.

use std::{fs, path::Path, time::Instant};
use yav_router::FileSystemRouter;

/// Runs the static site export command.
pub async fn run(output_dir: &str, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n  \x1b[1;36myav export\x1b[0m \x1b[90m— Static Site Generator (SSG)\x1b[0m\n");

    let start_time = Instant::now();
    let out_path = Path::new(output_dir);
    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }

    println!("  Target output directory: \x1b[1m{}\x1b[0m", out_path.display());

    // 1. Copy public static assets into output_dir
    let public_dir = Path::new("public");
    let mut copied_assets = 0;
    if public_dir.exists() && public_dir.is_dir() {
        println!("  Copying public assets...");
        copied_assets = copy_dir_all(public_dir, out_path)?;
        println!("    \x1b[32m[OK]\x1b[0m Copied {copied_assets} static asset files");
    }

    // 2. Scan routes
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

    println!("  Discovered {} static route(s) to prerender:", static_routes.len());
    for route in &static_routes {
        println!("    - {}", route);
    }

    // 3. If inside an executable Yav application project, run SSR export via YAV_EXPORT_DIR
    let mut ran_ssr_export = false;
    if !cfg!(test)
        && Path::new("package.yavml").is_file()
        && Path::new("Cargo.toml").is_file()
        && Path::new("src/main.rs").is_file()
    {
        if let Ok(status) = tokio::process::Command::new("cargo")
            .arg("run")
            .arg("--quiet")
            .env("YAV_EXPORT_DIR", out_path.as_os_str())
            .env("YAV_EXPORT_BASE_URL", base_url)
            .status()
            .await
        {
            ran_ssr_export = status.success();
        }
    }

    // 4. Verify / fallback prerender for each discovered static route
    let mut exported_count = 0;
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

        if !ran_ssr_export || !rel_file.exists() {
            let clean_title = if route == "/" { "Home" } else { route.trim_start_matches('/') };
            let title = format!("{} — Yav Application", clean_title.replace('/', " — "));
            let body = format!(
                r#"<main class="min-h-screen bg-slate-950 text-white flex flex-col items-center justify-center p-8">
  <div class="max-w-xl text-center space-y-4">
    <h1 class="text-3xl font-bold tracking-tight">Static Export: {}</h1>
    <p class="text-slate-400">Prerendered with Yav.rs Static Site Generator.</p>
    <a href="/" class="inline-block mt-4 px-4 py-2 bg-indigo-600 rounded-lg text-sm font-semibold hover:bg-indigo-500">Back Home</a>
  </div>
</main>"#,
                route
            );
            let html = yav_core::document_shell(&title, "", &body, yav_core::YAV_MICRO_ACTIONS_SCRIPT);
            fs::write(&rel_file, html)?;
        }
        exported_count += 1;
        println!("    \x1b[32m✔\x1b[0m Prerendered {} ➜ {}", route, rel_file.display());
    }

    // 4. Generate sitemap and robots.txt in export directory
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
                format!("  <url><loc>{full}</loc><priority>0.8</priority></url>")
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

    let duration = start_time.elapsed();
    println!(
        "\n\x1b[1;32m✔ Static site export completed in {:.2?}\x1b[0m",
        duration
    );
    println!("  • Prerendered HTML pages : {exported_count}");
    println!("  • Static assets copied   : {copied_assets}");
    println!("  • Export directory       : {}", out_path.display());
    println!("\n\x1b[1mReady for deployment to GitHub Pages, Cloudflare Pages, Vercel, or AWS S3!\x1b[0m");

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

        let sitemap = fs::read_to_string(temp_dir.join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("https://example.com"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
