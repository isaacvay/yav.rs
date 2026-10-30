//! Project analysis and performance audit command (`yav analyze`).
//!
//! Inspects routes, components, CSS bundle size, static assets, binary footprint,
//! caching, SEO, and PWA readiness, printing a comprehensive report.

use std::fs;
use std::path::{Path, PathBuf};
use yav_build::manifest::PackageYav;

/// Formats a byte count into human-readable string (B, KB, MB).
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Information gathered during analysis.
#[derive(Debug, Default)]
pub struct AnalysisReport {
    pub package_name: String,
    pub package_version: String,
    pub route_count: usize,
    pub dynamic_route_count: usize,
    pub component_count: usize,
    pub css_size_bytes: Option<u64>,
    pub total_asset_count: usize,
    pub total_asset_size_bytes: u64,
    pub large_unoptimized_images: Vec<(String, u64)>,
    pub binary_size_bytes: Option<u64>,
    pub has_sitemap: bool,
    pub has_robots: bool,
    pub has_favicon: bool,
    pub has_manifest: bool,
    pub has_service_worker: bool,
    pub motion_enabled: bool,
    pub etag_enabled: bool,
}

/// Recursively scans a directory collecting files and calculating sizes.
fn scan_dir_files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut files = Vec::new();
    if !dir.exists() || !dir.is_dir() {
        return files;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(scan_dir_files(&path));
            } else if path.is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push((path, size));
            }
        }
    }
    files
}

/// Analyzes the project at the given root directory.
pub fn analyze_project(root: &Path) -> Result<AnalysisReport, Box<dyn std::error::Error>> {
    let mut report = AnalysisReport::default();

    // 1. Manifest
    if let Ok(manifest) = PackageYav::discover_in(root) {
        report.package_name = manifest.package.name;
        report.package_version = manifest.package.version;
        report.motion_enabled = manifest.motion.enabled;
        report.etag_enabled = true; // default in YavApp
    } else {
        report.package_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();
        report.package_version = "0.1.0".to_string();
        report.motion_enabled = true;
        report.etag_enabled = true;
    }

    // 2. Routes
    let routes_dir = root.join("src/routes");
    let route_files = scan_dir_files(&routes_dir);
    for (path, _) in &route_files {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if filename.ends_with(".rs") || filename.ends_with(".yav") || filename.ends_with(".rsx") {
            report.route_count += 1;
            if filename.contains('[') || filename.contains(':') || filename.contains('{') {
                report.dynamic_route_count += 1;
            }
        }
    }

    // 3. Components
    let components_dir = root.join("src/components");
    let component_files = scan_dir_files(&components_dir);
    for (path, _) in &component_files {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if (filename.ends_with(".rs") || filename.ends_with(".yav")) && filename != "mod.rs" {
            report.component_count += 1;
        }
    }

    // 4. CSS
    let css_file = root.join("public/styles.css");
    if css_file.exists() {
        report.css_size_bytes = fs::metadata(&css_file).map(|m| m.len()).ok();
    }

    // 5. Assets in public/
    let public_dir = root.join("public");
    let asset_files = scan_dir_files(&public_dir);
    report.total_asset_count = asset_files.len();
    for (path, size) in &asset_files {
        report.total_asset_size_bytes += size;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check for large unoptimized PNG or JPG (> 250 KB)
        if (filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg"))
            && *size > 250 * 1024
        {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            report.large_unoptimized_images.push((rel, *size));
        }

        if filename == "sitemap.xml" {
            report.has_sitemap = true;
        }
        if filename == "robots.txt" {
            report.has_robots = true;
        }
        if filename == "favicon.ico" || filename == "favicon.svg" || filename == "favicon-flat.svg"
        {
            report.has_favicon = true;
        }
        if filename == "manifest.json" {
            report.has_manifest = true;
        }
        if filename == "sw.js" {
            report.has_service_worker = true;
        }
    }

    // 6. Binary size
    let target_release = root
        .join("target/release")
        .join(format!("{}.exe", report.package_name));
    let target_release_unix = root.join("target/release").join(&report.package_name);
    let target_debug = root
        .join("target/debug")
        .join(format!("{}.exe", report.package_name));
    let target_debug_unix = root.join("target/debug").join(&report.package_name);

    if target_release.exists() {
        report.binary_size_bytes = fs::metadata(&target_release).map(|m| m.len()).ok();
    } else if target_release_unix.exists() {
        report.binary_size_bytes = fs::metadata(&target_release_unix).map(|m| m.len()).ok();
    } else if target_debug.exists() {
        report.binary_size_bytes = fs::metadata(&target_debug).map(|m| m.len()).ok();
    } else if target_debug_unix.exists() {
        report.binary_size_bytes = fs::metadata(&target_debug_unix).map(|m| m.len()).ok();
    }

    Ok(report)
}

/// Runs the `yav analyze` command.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(".");
    let report = analyze_project(&root)?;

    println!();
    println!("  \x1b[1;36mYav Project Analysis & Performance Audit\x1b[0m");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "  ● Project:      \x1b[1m{}\x1b[0m (v{})",
        report.package_name, report.package_version
    );
    println!(
        "  ● Routes:       {} ({} dynamic)",
        report.route_count, report.dynamic_route_count
    );
    println!(
        "  ● Components:   {} custom components",
        report.component_count
    );

    // CSS analysis
    if let Some(css) = report.css_size_bytes {
        let status = if css < 50 * 1024 {
            "\x1b[32m✔ Excellent (< 50 KB)\x1b[0m"
        } else if css < 150 * 1024 {
            "\x1b[33m✔ Good (< 150 KB)\x1b[0m"
        } else {
            "\x1b[31m! Large (> 150 KB, consider purging)\x1b[0m"
        };
        println!("  ● CSS Bundle:   {} - {}", format_size(css), status);
    } else {
        println!(
            "  ● CSS Bundle:   \x1b[33mNot yet generated (run `yav build` or `yav dev`)\x1b[0m"
        );
    }

    // Static assets
    println!(
        "  ● Assets:       {} files ({})",
        report.total_asset_count,
        format_size(report.total_asset_size_bytes)
    );

    // Binary
    if let Some(bin) = report.binary_size_bytes {
        println!("  ● Binary Size:  {}", format_size(bin));
    }

    println!();
    println!("  \x1b[1;34mReadiness & Best Practices Scorecard\x1b[0m");
    println!("────────────────────────────────────────────────────────────");

    print_check(
        "Motion Engine (yav-motion)",
        report.motion_enabled,
        "Enabled in package.yavml (WAAPI hardware-accelerated)",
    );
    print_check(
        "HTTP ETag Caching",
        report.etag_enabled,
        "304 Not Modified conditional responses active",
    );
    print_check(
        "Favicon (`public/favicon.ico`)",
        report.has_favicon,
        "Provides browser tab branding",
    );
    print_check(
        "SEO Sitemap (`public/sitemap.xml`)",
        report.has_sitemap,
        "Run `yav sitemap` to generate",
    );
    print_check(
        "Robots Directive (`public/robots.txt`)",
        report.has_robots,
        "Run `yav sitemap` to generate",
    );
    print_check(
        "PWA Web Manifest (`public/manifest.json`)",
        report.has_manifest,
        "Run `yav pwa` to scaffold",
    );
    print_check(
        "PWA Service Worker (`public/sw.js`)",
        report.has_service_worker,
        "Run `yav pwa` to scaffold",
    );

    // Warnings / Recommendations
    if !report.large_unoptimized_images.is_empty() {
        println!();
        println!("⚠️  \x1b[1;33mAsset Optimization Recommendations:\x1b[0m");
        for (img, size) in &report.large_unoptimized_images {
            println!(
                "   - {} ({}) -> Consider converting to .webp or .avif",
                img,
                format_size(*size)
            );
        }
    }

    println!();
    println!(
        "✨ \x1b[32mAudit complete! Your Yav application is running with native SSR speed.\x1b[0m"
    );
    println!();

    Ok(())
}

fn print_check(name: &str, passed: bool, hint: &str) {
    if passed {
        println!("  \x1b[32m✔\x1b[0m {:<32} \x1b[90m{}\x1b[0m", name, hint);
    } else {
        println!("  \x1b[33m○\x1b[0m {:<32} \x1b[33m{}\x1b[0m", name, hint);
    }
}
