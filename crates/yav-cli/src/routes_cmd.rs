//! Route table inspector command (`yav routes`).
//!
//! Scans `src/routes/` and prints a formatted Editorial Blueprint table showing all
//! discovered URL patterns, HTTP methods, source files, and active `_layout.rs` / `_middleware.rs` chains.

use std::fs;
use std::path::Path;
use yav_router::fs_router::FileSystemRouter;

/// Metadata row for a single discovered route in `yav routes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRow {
    pub methods: String,
    pub pattern: String,
    pub file: String,
    pub layouts: Vec<String>,
    pub middlewares: Vec<String>,
}

/// Inspect a routes directory (e.g. `src/routes`) and return structured [`RouteRow`] entries.
pub fn inspect_routes(routes_dir: &Path) -> Vec<RouteRow> {
    if !routes_dir.is_dir() {
        return Vec::new();
    }

    let router = match FileSystemRouter::scan_dir(routes_dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut rows = Vec::new();
    for route in router.routes() {
        let rel = route
            .file_path
            .strip_prefix(routes_dir)
            .unwrap_or(&route.file_path);
        let full_path = if route.file_path.is_absolute() {
            route.file_path.clone()
        } else {
            routes_dir.join(rel)
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let methods = detect_http_methods(&full_path);
        let layouts = detect_ancestors(routes_dir, rel, &["_layout.rs", "layout.rs"]);
        let middlewares = detect_ancestors(routes_dir, rel, &["_middleware.rs", "middleware.rs"]);

        rows.push(RouteRow {
            methods,
            pattern: route.path_pattern.clone(),
            file: format!("src/routes/{rel_str}"),
            layouts,
            middlewares,
        });
    }

    rows
}

fn detect_http_methods(file_path: &Path) -> String {
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "yav" || ext == "rsx" {
        return "GET".to_string();
    }
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return "GET".to_string(),
    };
    let mut methods = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let after_fn = trimmed
            .strip_prefix("pub async fn ")
            .or_else(|| trimmed.strip_prefix("pub fn "));
        if let Some(rest) = after_fn {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if (name == "get"
                || name.ends_with("_get")
                || name == "page"
                || name == "index"
                || name == "handler")
                && !methods.contains(&"GET")
            {
                methods.push("GET");
            } else if (name == "post" || name.ends_with("_post")) && !methods.contains(&"POST") {
                methods.push("POST");
            } else if (name == "put" || name.ends_with("_put")) && !methods.contains(&"PUT") {
                methods.push("PUT");
            } else if (name == "delete" || name.ends_with("_delete"))
                && !methods.contains(&"DELETE")
            {
                methods.push("DELETE");
            } else if (name == "patch" || name.ends_with("_patch")) && !methods.contains(&"PATCH") {
                methods.push("PATCH");
            }
        }
    }
    if methods.is_empty() {
        "GET".to_string()
    } else {
        methods.join(",")
    }
}

fn detect_ancestors(routes_dir: &Path, rel_file: &Path, candidates: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for cand in candidates {
        if routes_dir.join(cand).is_file() {
            out.push((*cand).to_string());
            break;
        }
    }
    let mut dirs = Vec::new();
    let mut curr = rel_file.parent();
    while let Some(p) = curr {
        if !p.as_os_str().is_empty() {
            dirs.push(p);
        }
        curr = p.parent();
    }
    dirs.reverse();
    for d in dirs {
        for cand in candidates {
            let candidate_path = routes_dir.join(d).join(cand);
            if candidate_path.is_file() {
                let d_str = d.to_string_lossy().replace('\\', "/");
                out.push(format!("{d_str}/{cand}"));
                break;
            }
        }
    }
    out
}

/// Execute `yav routes`.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n  \x1b[1;38;2;224;89;47m//\x1b[0m \x1b[1myav routes\x1b[0m \x1b[90m— File-System Route Table\x1b[0m\n");
    let routes_dir = Path::new("src").join("routes");
    let rows = inspect_routes(&routes_dir);

    if rows.is_empty() {
        println!("  \x1b[90mNo file-system routes found in src/routes/.\x1b[0m\n");
        return Ok(());
    }

    println!(
        "  \x1b[90m{:<10} {:<24} {:<30} PIPELINE (LAYOUTS / MIDDLEWARE)\x1b[0m",
        "METHOD", "PATTERN", "SOURCE"
    );
    println!("  \x1b[90m{}\x1b[0m", "─".repeat(86));

    for row in &rows {
        let mut chain = Vec::new();
        for mw in &row.middlewares {
            chain.push(format!("mw:{mw}"));
        }
        for ly in &row.layouts {
            chain.push(format!("layout:{ly}"));
        }
        let pipeline = if chain.is_empty() {
            "—".to_string()
        } else {
            chain.join(" → ")
        };
        println!(
            "  \x1b[1;38;2;224;89;47m{:<10}\x1b[0m \x1b[1m{:<24}\x1b[0m \x1b[36m{:<30}\x1b[0m \x1b[90m{}\x1b[0m",
            row.methods, row.pattern, row.file, pipeline
        );
    }

    println!("\n  \x1b[32m✔ {} route(s) active.\x1b[0m\n", rows.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_routes_table() {
        let temp = std::env::temp_dir().join(format!("yav_routes_cmd_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("users")).unwrap();
        fs::write(
            temp.join("_layout.rs"),
            "pub fn layout(n: Node) -> Node { n }",
        )
        .unwrap();
        fs::write(
            temp.join("index.rs"),
            "pub async fn get() -> String { String::new() }",
        )
        .unwrap();
        fs::write(
            temp.join("users").join("[id].rs"),
            "pub async fn get() -> String { String::new() }\npub async fn post() -> String { String::new() }",
        )
        .unwrap();

        let rows = inspect_routes(&temp);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.pattern == "/" && r.methods == "GET"));
        assert!(rows.iter().any(|r| r.pattern == "/users/:id"
            && r.methods == "GET,POST"
            && !r.layouts.is_empty()));

        let _ = fs::remove_dir_all(&temp);
    }
}
