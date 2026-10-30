//! File-system routing convention and scanner.
//!
//! Maps file structures (similar to Next.js App Router) to URL routes:
//! - `page.rs` or `index.rs` -> `/`
//! - `about.rs` or `about/page.rs` -> `/about`
//! - `users/[id].rs` or `users/[id]/page.rs` -> `/users/:id`
//! - `files/[...slug].rs` -> `/files/*slug`

use crate::route::Route;
use crate::table::RouteTable;
use std::path::{Path, PathBuf};

/// Represents a discovered file route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredRoute {
    /// URL pattern (e.g. `/`, `/about`, `/users/:id`, `/files/*slug`).
    pub path_pattern: String,
    /// Absolute or relative path to the route source file.
    pub file_path: PathBuf,
    /// Associated layout file paths in hierarchical order (root layout first).
    pub layouts: Vec<PathBuf>,
}

/// Convert a relative file path inside a `routes` directory into a URL route pattern.
///
/// # Examples
/// ```
/// use yav_router::fs_router::file_path_to_route_pattern;
///
/// assert_eq!(file_path_to_route_pattern("page.rs"), "/");
/// assert_eq!(file_path_to_route_pattern("about.rs"), "/about");
/// assert_eq!(file_path_to_route_pattern("users/[id].rs"), "/users/:id");
/// assert_eq!(file_path_to_route_pattern("blog/[...slug]/page.rs"), "/blog/*slug");
/// ```
pub fn file_path_to_route_pattern(rel_path: impl AsRef<Path>) -> String {
    let p = rel_path.as_ref();
    let mut segments = Vec::new();

    for comp in p.components() {
        let s = comp.as_os_str().to_str().unwrap_or("");
        // Remove .rs, .yav, or .rsx extension if present
        let clean = s
            .strip_suffix(".rs")
            .or_else(|| s.strip_suffix(".yav"))
            .or_else(|| s.strip_suffix(".rsx"))
            .unwrap_or(s);

        // Skip root files like "page", "index", "mod"
        if clean == "page" || clean == "index" || clean == "mod" {
            continue;
        }

        // Route groups: folders enclosed in parentheses (e.g. `(auth)`, `(marketing)`) organize code without affecting URL
        if clean.starts_with('(') && clean.ends_with(')') && clean.len() >= 2 {
            continue;
        }

        // Convert [id] -> :id or _id_ -> :id (Rust-friendly identifier syntax)
        if clean.starts_with('[') && clean.ends_with(']') && clean.len() >= 2 {
            let inner = &clean[1..clean.len() - 1];
            if let Some(slug) = inner.strip_prefix("...") {
                // Catch-all: [...slug] -> *slug
                segments.push(format!("*{}", slug));
            } else {
                // Dynamic: [id] -> :id
                segments.push(format!(":{}", inner));
            }
        } else if clean.starts_with("__") && clean.ends_with("__") && clean.len() > 4 {
            // Catch-all: __slug__ -> *slug (Rust-safe syntax)
            let inner = &clean[2..clean.len() - 2];
            segments.push(format!("*{}", inner));
        } else if clean.starts_with('_') && clean.ends_with('_') && clean.len() > 2 {
            // Dynamic: _id_ -> :id (Rust-safe syntax)
            let inner = &clean[1..clean.len() - 1];
            segments.push(format!(":{}", inner));
        } else {
            segments.push(clean.to_string());
        }
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

impl DiscoveredRoute {
    /// Return the Axum 0.8 compatible path pattern (e.g. `/users/{id}`, `/files/{*slug}`).
    pub fn axum_pattern(&self) -> String {
        crate::route::to_axum_pattern(&self.path_pattern)
    }

    /// Returns true if this route has dynamic segments (`:param` or `*slug`).
    pub fn is_dynamic(&self) -> bool {
        self.path_pattern.contains(':') || self.path_pattern.contains('*')
    }
}

/// Recursively collect relative paths from a directory for route discovery.
fn collect_fs_files(base: &Path, current: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_fs_files(base, &p, acc)?;
        } else if p
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "yav" || ext == "rsx")
        {
            if let Ok(rel) = p.strip_prefix(base) {
                acc.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

/// File-system routing scanner that discovers routes and nested layouts in a directory.
#[derive(Debug, Clone, Default)]
pub struct FileSystemRouter {
    discovered: Vec<DiscoveredRoute>,
}

impl FileSystemRouter {
    /// Create a new empty FileSystemRouter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recursively scan a directory on the file system to discover routes.
    pub fn scan_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<Self> {
        let dir_path = dir.as_ref();
        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(Self::new());
        }
        let mut paths = Vec::new();
        collect_fs_files(dir_path, dir_path, &mut paths)?;
        Ok(Self::from_paths(&paths))
    }

    /// Add a route mapping manually.
    pub fn add_route(&mut self, rel_path: impl AsRef<Path>) -> &mut Self {
        let pattern = file_path_to_route_pattern(rel_path.as_ref());
        self.discovered.push(DiscoveredRoute {
            path_pattern: pattern,
            file_path: rel_path.as_ref().to_path_buf(),
            layouts: Vec::new(),
        });
        self
    }

    /// Discover routes from a slice of file paths.
    pub fn from_paths<P: AsRef<Path>>(paths: &[P]) -> Self {
        let mut fs = Self::new();
        // Collect layouts first
        let layouts: Vec<PathBuf> = paths
            .iter()
            .filter(|p| {
                let s = p
                    .as_ref()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                s == "layout.rs"
                    || s == "layout.yav"
                    || s == "layout.rsx"
                    || s == "_layout.rs"
                    || s == "_layout.yav"
                    || s == "_layout.rsx"
            })
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        for p in paths {
            let file_name = p
                .as_ref()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if file_name == "layout.rs"
                || file_name == "layout.yav"
                || file_name == "layout.rsx"
                || file_name == "loading.rs"
                || file_name == "loading.yav"
                || file_name == "loading.rsx"
                || file_name == "error.rs"
                || file_name == "error.yav"
                || file_name == "error.rsx"
                || file_name == "not_found.rs"
                || file_name == "not_found.yav"
                || file_name == "not_found.rsx"
                || file_name == "mod.rs"
                || file_name.starts_with('_') && !file_name.ends_with("_.rs")
            {
                continue;
            }

            let pattern = file_path_to_route_pattern(p.as_ref());
            // Find applicable parent layouts
            let mut applicable_layouts = Vec::new();
            for l in &layouts {
                if let Some(layout_parent) = l.parent() {
                    if p.as_ref().starts_with(layout_parent) {
                        applicable_layouts.push(l.clone());
                    }
                }
            }
            // Sort applicable layouts hierarchically by path component count (root layout first)
            applicable_layouts.sort_by_key(|l| l.components().count());

            fs.discovered.push(DiscoveredRoute {
                path_pattern: pattern,
                file_path: p.as_ref().to_path_buf(),
                layouts: applicable_layouts,
            });
        }
        fs
    }

    /// Returns all discovered routes.
    pub fn routes(&self) -> &[DiscoveredRoute] {
        &self.discovered
    }

    /// Populate a [`RouteTable`] with the discovered routes for a given HTTP method.
    pub fn populate_table(&self, table: &mut RouteTable, method: &str) {
        for r in &self.discovered {
            table.add(Route::new(
                method,
                &r.path_pattern,
                r.file_path.to_string_lossy(),
            ));
        }
    }

    /// Populate a [`RouteTable`] for multiple HTTP methods.
    pub fn populate_table_methods(&self, table: &mut RouteTable, methods: &[&str]) {
        for method in methods {
            self.populate_table(table, method);
        }
    }

    /// Find a discovered route matching the exact path pattern.
    pub fn find_by_pattern(&self, pattern: &str) -> Option<&DiscoveredRoute> {
        self.discovered.iter().find(|r| r.path_pattern == pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_to_route_pattern() {
        assert_eq!(file_path_to_route_pattern("page.rs"), "/");
        assert_eq!(file_path_to_route_pattern("index.rs"), "/");
        assert_eq!(file_path_to_route_pattern("about.rs"), "/about");
        assert_eq!(file_path_to_route_pattern("about/page.rs"), "/about");
        assert_eq!(file_path_to_route_pattern("contact.yav"), "/contact");
        assert_eq!(file_path_to_route_pattern("team/page.rsx"), "/team");
        assert_eq!(file_path_to_route_pattern("users/[id].rs"), "/users/:id");
        assert_eq!(
            file_path_to_route_pattern("users/[id]/page.rs"),
            "/users/:id"
        );
        assert_eq!(
            file_path_to_route_pattern("docs/[...slug].rs"),
            "/docs/*slug"
        );
        assert_eq!(
            file_path_to_route_pattern("docs/[...slug]/page.rs"),
            "/docs/*slug"
        );
    }

    #[test]
    fn test_file_system_router_discovery_and_layouts() {
        let files = vec![
            PathBuf::from("layout.rs"),
            PathBuf::from("mod.rs"),
            PathBuf::from("_private.rs"),
            PathBuf::from("page.rs"),
            PathBuf::from("about/page.rs"),
            PathBuf::from("dashboard/layout.rs"),
            PathBuf::from("dashboard/mod.rs"),
            PathBuf::from("dashboard/page.rs"),
            PathBuf::from("dashboard/settings/page.rs"),
        ];

        let fs = FileSystemRouter::from_paths(&files);
        let routes = fs.routes();

        assert_eq!(routes.len(), 4);
        assert_eq!(routes[0].path_pattern, "/");
        assert_eq!(routes[1].path_pattern, "/about");
        assert_eq!(routes[2].path_pattern, "/dashboard");
        assert_eq!(routes[3].path_pattern, "/dashboard/settings");

        // Dashboard routes should have both root layout and dashboard layout
        assert_eq!(routes[2].layouts.len(), 2);
    }

    #[test]
    fn test_rust_friendly_identifiers() {
        assert_eq!(file_path_to_route_pattern("users/_id_.rs"), "/users/:id");
        assert_eq!(
            file_path_to_route_pattern("users/_id_/posts.rs"),
            "/users/:id/posts"
        );
        assert_eq!(
            file_path_to_route_pattern("docs/__slug__.rs"),
            "/docs/*slug"
        );

        let files = vec![
            PathBuf::from("index.rs"),
            PathBuf::from("users/_id_.rs"),
            PathBuf::from("docs/__slug__.rs"),
        ];
        let fs = FileSystemRouter::from_paths(&files);
        assert_eq!(fs.routes().len(), 3);
        assert_eq!(fs.routes()[1].axum_pattern(), "/users/{id}");
        assert_eq!(fs.routes()[2].axum_pattern(), "/docs/{*slug}");
        assert!(fs.routes()[1].is_dynamic());
    }

    #[test]
    fn test_route_groups_parentheses() {
        assert_eq!(file_path_to_route_pattern("(auth)/login.rs"), "/login");
        assert_eq!(
            file_path_to_route_pattern("(auth)/register/page.rs"),
            "/register"
        );
        assert_eq!(
            file_path_to_route_pattern("(marketing)/pricing/page.rs"),
            "/pricing"
        );
    }
}
