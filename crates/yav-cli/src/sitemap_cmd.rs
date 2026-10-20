//! Static sitemap and robots.txt generation command.
//!
//! Scans the application's `src/routes/` directory to generate compliant
//! `public/sitemap.xml` and `public/robots.txt` files for production builds.

use std::{fs, path::Path};

/// Generates static sitemap.xml and robots.txt into `public/`.
pub fn run(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let base = base_url.trim_end_matches('/');
    let routes_dir = Path::new("src/routes");
    let public_dir = Path::new("public");

    if !public_dir.exists() {
        fs::create_dir_all(public_dir)?;
    }

    let mut urls = vec![format!("{base}/")];

    if routes_dir.exists() {
        scan_routes_recursive(routes_dir, "", &mut urls, base);
        urls.sort();
        urls.dedup();
    }

    // 1. Generate sitemap.xml
    let mut sitemap_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    sitemap_xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for url in &urls {
        sitemap_xml.push_str("  <url>\n");
        sitemap_xml.push_str(&format!("    <loc>{}</loc>\n", xml_escape(url)));
        sitemap_xml.push_str("    <changefreq>weekly</changefreq>\n");
        sitemap_xml.push_str("    <priority>0.8</priority>\n");
        sitemap_xml.push_str("  </url>\n");
    }
    sitemap_xml.push_str("</urlset>\n");

    let sitemap_path = public_dir.join("sitemap.xml");
    fs::write(&sitemap_path, sitemap_xml)?;
    println!(
        "  generated {} with {} URLs",
        sitemap_path.display(),
        urls.len()
    );

    // 2. Generate robots.txt
    let robots_txt = format!("User-agent: *\nAllow: /\n\nSitemap: {base}/sitemap.xml\n");
    let robots_path = public_dir.join("robots.txt");
    fs::write(&robots_path, robots_txt)?;
    println!("  generated {}", robots_path.display());

    println!("✨ Search engine discovery files created in public/");
    Ok(())
}

fn scan_routes_recursive(dir: &Path, prefix: &str, urls: &mut Vec<String>, base_url: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name.starts_with('_') || dir_name.starts_with('.') {
                    continue;
                }
                let new_prefix = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{prefix}/{dir_name}")
                };
                scan_routes_recursive(&path, &new_prefix, urls, base_url);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "rs" || ext == "yav" || ext == "rsx")
            {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                if stem == "mod"
                    || stem == "layout"
                    || stem == "loading"
                    || stem == "error"
                    || stem == "template"
                    || stem == "not_found"
                    || stem.starts_with('_')
                    || stem.starts_with('.')
                {
                    continue;
                }
                if stem == "index" || stem == "page" {
                    if !prefix.is_empty() {
                        urls.push(format!("{base_url}/{prefix}"));
                    }
                } else if !stem.starts_with('[') {
                    let route_path = if prefix.is_empty() {
                        stem.to_string()
                    } else {
                        format!("{prefix}/{stem}")
                    };
                    urls.push(format!("{base_url}/{route_path}"));
                }
            }
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_routes_filtering() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_sitemap_test_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("admin"));
        fs::write(temp_dir.join("layout.rs"), "// layout").unwrap();
        fs::write(temp_dir.join("page.rs"), "// home").unwrap();
        fs::write(temp_dir.join("about.rs"), "// about").unwrap();
        fs::write(temp_dir.join("_secret.rs"), "// secret").unwrap();
        fs::write(temp_dir.join("admin").join("layout.rs"), "// admin layout").unwrap();
        fs::write(temp_dir.join("admin").join("page.rs"), "// admin page").unwrap();

        let mut urls = Vec::new();
        let base = "https://example.com";
        scan_routes_recursive(&temp_dir, "", &mut urls, base);
        urls.sort();

        assert!(!urls.contains(&"https://example.com/layout".to_string()));
        assert!(!urls.contains(&"https://example.com/admin/layout".to_string()));
        assert!(!urls.contains(&"https://example.com/_secret".to_string()));
        assert!(urls.contains(&"https://example.com/about".to_string()));
        assert!(urls.contains(&"https://example.com/admin".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(
            xml_escape("https://example.com/search?q=a&b=c"),
            "https://example.com/search?q=a&amp;b=c"
        );
    }
}
