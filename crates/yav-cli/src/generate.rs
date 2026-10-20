//! Code generation subcommand (`yav generate` / `yav g`).
//!
//! Generates routes, components, and layouts with idiomatic Yav conventions.

use std::fs;
use std::path::{Path, PathBuf};

/// Target generator kind.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum GenerateTarget {
    /// Generate a new route handler in src/routes/
    Route {
        /// Route path (e.g. "dashboard", "blog/[slug]", "api/users")
        path: String,
    },
    /// Generate a new functional component in src/components/
    Component {
        /// Component name (e.g. "UserCard" or "stats-card")
        name: String,
    },
    /// Generate a new layout in src/layouts/
    Layout {
        /// Layout name (e.g. "AdminLayout" or "auth")
        name: String,
    },
    /// Generate TypeScript interfaces and route types
    #[command(alias = "ts")]
    Typescript {
        /// Destination file path (default: types.d.ts)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Export OpenAPI 3.0 specification from discovered application routes
    #[command(alias = "api")]
    Openapi {
        /// Destination file path (default: openapi.json)
        #[arg(short, long)]
        output: Option<String>,
    },
}

/// Run code generator for the selected target.
pub fn run(target: &GenerateTarget) -> Result<(), Box<dyn std::error::Error>> {
    let project_root = find_project_root().unwrap_or_else(|| PathBuf::from("."));

    match target {
        GenerateTarget::Route { path } => generate_route(&project_root, path)?,
        GenerateTarget::Component { name } => generate_component(&project_root, name)?,
        GenerateTarget::Layout { name } => generate_layout(&project_root, name)?,
        GenerateTarget::Typescript { output } => {
            generate_typescript(&project_root, output.clone())?
        }
        GenerateTarget::Openapi { output } => generate_openapi(&project_root, output.clone())?,
    }

    Ok(())
}

fn generate_route(root: &Path, raw_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let clean_path = raw_path.trim_matches('/').replace('\\', "/");
    let mut file_path = root.join("src").join("routes");

    let segments: Vec<&str> = clean_path.split('/').collect();
    for &seg in &segments[..segments.len() - 1] {
        file_path.push(seg);
    }
    let last = segments.last().copied().unwrap_or("index");
    let file_name = if last.ends_with(".rs") {
        last.to_string()
    } else {
        format!("{last}.rs")
    };
    file_path.push(file_name);

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if file_path.exists() {
        eprintln!(
            "\x1b[33m⚠️ Route file already exists: {}\x1b[0m",
            file_path.display()
        );
        return Ok(());
    }

    let route_title = to_title_case(last.trim_end_matches(".rs"));
    let code = format!(
        r#"//! Route handler for /{clean_path}

use axum::extract::Request;
use yav::prelude::*;

pub async fn handler(_req: Request) -> HtmlPage {{
    HtmlPage::new(rsx! {{
        <main class="min-h-screen p-8 bg-gray-50 dark:bg-gray-900">
            <div class="max-w-4xl mx-auto">
                <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                    "{route_title}"
                </h1>
                <p class="mt-4 text-gray-600 dark:text-gray-300">
                    "Page content for /{clean_path}."
                </p>
            </div>
        </main>
    }})
    .title("{route_title}")
}}
"#
    );

    fs::write(&file_path, code)?;
    println!("\x1b[32m✔ Created route:\x1b[0m {}", file_path.display());

    let stem = last.trim_end_matches(".rs");
    if let Some(parent) = file_path.parent() {
        let mod_file = parent.join("mod.rs");
        let _ = ensure_mod_registered(&mod_file, stem, false);
    }

    let route_pattern = yav_router::fs_router::file_path_to_route_pattern(&clean_path);
    let mod_ident = if stem.contains('[') || stem.contains(']') {
        stem.replace('[', "").replace(']', "")
    } else {
        stem.to_string()
    };
    println!(
        "\x1b[36m💡 Wire in main.rs:\x1b[0m .get(\"{route_pattern}\", routes::{mod_ident}::handler)"
    );

    Ok(())
}

fn generate_component(root: &Path, raw_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pascal_name = to_pascal_case(raw_name);
    let snake_name = to_snake_case(raw_name);
    let dir = root.join("src").join("components");
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("{snake_name}.rs"));
    if file_path.exists() {
        eprintln!(
            "\x1b[33m⚠️ Component file already exists: {}\x1b[0m",
            file_path.display()
        );
        return Ok(());
    }

    let code = format!(
        r#"//! {pascal_name} component.

use yav::prelude::*;

#[derive(Clone, Default, PartialEq)]
pub struct {pascal_name}Props {{
    pub class: String,
    pub children: Vec<Node>,
}}

#[component]
pub fn {pascal_name}(props: {pascal_name}Props) -> Node {{
    rsx! {{
        <div class={{format!("rounded-lg border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-700 dark:bg-gray-800 {{}}", props.class)}}>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                "{pascal_name}"
            </h3>
            <div class="text-gray-600 dark:text-gray-300">
                {{props.children}}
            </div>
        </div>
    }}
}}
"#
    );

    fs::write(&file_path, code)?;
    println!(
        "\x1b[32m✔ Created component:\x1b[0m {}",
        file_path.display()
    );

    let mod_file = dir.join("mod.rs");
    let _ = ensure_mod_registered(&mod_file, &snake_name, true);

    Ok(())
}

fn generate_layout(root: &Path, raw_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pascal_name = to_pascal_case(raw_name);
    let snake_name = to_snake_case(raw_name);
    let dir = root.join("src").join("layouts");
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("{snake_name}.rs"));
    if file_path.exists() {
        eprintln!(
            "\x1b[33m⚠️ Layout file already exists: {}\x1b[0m",
            file_path.display()
        );
        return Ok(());
    }

    let code = format!(
        r#"//! {pascal_name} layout.

use yav::prelude::*;

#[derive(Clone, Default, PartialEq)]
pub struct {pascal_name}Props {{
    pub children: Vec<Node>,
}}

#[component]
pub fn {pascal_name}(props: {pascal_name}Props) -> Node {{
    rsx! {{
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <header class="border-b border-gray-200 bg-white px-6 py-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
                <div class="max-w-7xl mx-auto flex items-center justify-between">
                    <span class="font-bold text-lg text-gray-900 dark:text-white">
                        "Yav Application"
                    </span>
                </div>
            </header>
            <main class="flex-1 max-w-7xl w-full mx-auto p-6">
                {{props.children}}
            </main>
        </div>
    }}
}}
"#
    );

    fs::write(&file_path, code)?;
    println!("\x1b[32m✔ Created layout:\x1b[0m {}", file_path.display());

    let mod_file = dir.join("mod.rs");
    let _ = ensure_mod_registered(&mod_file, &snake_name, true);

    Ok(())
}

fn ensure_mod_registered(
    mod_file: &Path,
    mod_name: &str,
    reexport: bool,
) -> Result<(), std::io::Error> {
    if !mod_file.exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(mod_file)?;

    // Handle parameter syntax like [slug] or [...path] using #[path = "..."] attribute
    let (mod_ident, path_attr) =
        if mod_name.contains('[') || mod_name.contains(']') || mod_name.contains('.') {
            let clean = mod_name.replace('[', "").replace(']', "").replace('.', "_");
            let ident = if clean.is_empty() {
                "param".to_string()
            } else if clean.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                format!("route_{clean}")
            } else {
                clean
            };
            (ident, Some(format!("{mod_name}.rs")))
        } else {
            (mod_name.to_string(), None)
        };

    let decl = match path_attr {
        Some(file) => {
            if reexport {
                format!("#[path = \"{file}\"]\npub mod {mod_ident};\npub use {mod_ident}::*;")
            } else {
                format!("#[path = \"{file}\"]\npub mod {mod_ident};")
            }
        }
        None => {
            if reexport {
                format!("pub mod {mod_ident};\npub use {mod_ident}::*;")
            } else {
                format!("pub mod {mod_ident};")
            }
        }
    };

    if !existing.contains(&format!("pub mod {mod_ident}"))
        && !existing.contains(&format!("mod {mod_ident}"))
    {
        let mut updated = existing;
        if !updated.ends_with('\n') && !updated.is_empty() {
            updated.push('\n');
        }
        updated.push_str(&decl);
        updated.push('\n');
        fs::write(mod_file, updated)?;
        println!(
            "\x1b[32m✔ Registered module:\x1b[0m '{}' in {}",
            mod_ident,
            mod_file.display()
        );
    }
    Ok(())
}

fn find_project_root() -> Option<PathBuf> {
    let mut curr = std::env::current_dir().ok()?;
    loop {
        if curr.join("package.yavml").exists()
            || curr.join("package.yav").exists()
            || curr.join("yav.toml").exists()
        {
            return Some(curr);
        }
        if !curr.pop() {
            break;
        }
    }
    None
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '-' || c == '_' || c == ' ' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c == '-' || c == ' ' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }
    result
}

fn to_title_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '-' || c == '_' || c == '/' || c == '[' || c == ']' {
            if !result.ends_with(' ') && !result.is_empty() {
                result.push(' ');
            }
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result.trim().to_string()
}

/// Discover all route patterns from `src/routes/`.
pub fn scan_route_patterns(root: &Path) -> Vec<String> {
    let routes_dir = root.join("src").join("routes");
    let mut patterns = Vec::new();
    if routes_dir.exists() && routes_dir.is_dir() {
        collect_patterns_recursive(&routes_dir, &routes_dir, &mut patterns);
    }
    if patterns.is_empty() {
        patterns.push("/".to_string());
    }
    patterns.sort();
    patterns.dedup();
    patterns
}

fn collect_patterns_recursive(base_dir: &Path, current_dir: &Path, acc: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_patterns_recursive(base_dir, &path, acc);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    let mut segments = Vec::new();
                    for comp in rel.components() {
                        let s = comp.as_os_str().to_str().unwrap_or("");
                        let clean = s.strip_suffix(".rs").unwrap_or(s);
                        if clean == "page" || clean == "index" || clean == "mod" {
                            continue;
                        }
                        if clean.starts_with('[') && clean.ends_with(']') && clean.len() >= 2 {
                            let inner = &clean[1..clean.len() - 1];
                            if let Some(slug) = inner.strip_prefix("...") {
                                segments.push(format!("*{}", slug));
                            } else {
                                segments.push(format!(":{}", inner));
                            }
                        } else {
                            segments.push(clean.to_string());
                        }
                    }
                    let pattern = if segments.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}", segments.join("/"))
                    };
                    acc.push(pattern);
                }
            }
        }
    }
}

/// Generate TypeScript interface declarations for routes, actions, and models.
pub fn generate_typescript(
    root: &Path,
    output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("types.d.ts"));

    let routes = scan_route_patterns(root);
    let mut route_union = routes
        .iter()
        .map(|r| format!("  | \"{}\"", r))
        .collect::<Vec<_>>()
        .join("\n");

    if route_union.is_empty() {
        route_union = "  | \"/\"".to_string();
    }

    let mut ts = String::new();
    ts.push_str("// Auto-generated by Yav CLI (yav codegen --ts). Do not edit manually.\n\n");
    ts.push_str("/**\n * Strongly typed union of all discovered application routes.\n */\n");
    ts.push_str(&format!("export type YavRoute =\n{};\n\n", route_union));

    ts.push_str(
        r#"/**
 * Standard server action response payload.
 */
export type ActionResponse<T = Record<string, unknown>> =
  | { type: "redirect"; url: string }
  | { type: "errors"; errors: Record<string, string[]> }
  | { type: "data"; data: T }
  | { type: "html"; fragment: string };

/**
 * Validation error map indexed by field name.
 */
export type FormErrors = Record<string, string[]>;

/**
 * Client-side route navigation params mapping.
 */
export type RouteParams = {
"#,
    );

    for route in &routes {
        if route.contains(':') || route.contains('*') {
            let mut params = Vec::new();
            for seg in route.split('/') {
                if let Some(param) = seg.strip_prefix(':') {
                    params.push(format!("{}: string", param));
                } else if let Some(slug) = seg.strip_prefix('*') {
                    params.push(format!("{}: string", slug));
                }
            }
            ts.push_str(&format!("  \"{}\": {{ {} }};\n", route, params.join("; ")));
        } else {
            ts.push_str(&format!("  \"{}\": Record<string, never>;\n", route));
        }
    }
    ts.push_str("};\n");

    fs::write(&out_path, ts)?;
    println!(
        "\x1b[32m✔ Generated TypeScript definitions:\x1b[0m {}",
        out_path.display()
    );
    Ok(())
}

/// Export OpenAPI 3.0.3 specification JSON from application routes.
pub fn generate_openapi(
    root: &Path,
    output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("openapi.json"));

    let routes = scan_route_patterns(root);

    let mut paths_obj = serde_json::Map::new();

    for route in routes {
        // Convert :id to {id} for OpenAPI standards
        let mut openapi_path = String::new();
        let mut path_params = Vec::new();

        for seg in route.split('/') {
            if seg.is_empty() {
                continue;
            }
            openapi_path.push('/');
            if let Some(param) = seg.strip_prefix(':') {
                openapi_path.push('{');
                openapi_path.push_str(param);
                openapi_path.push('}');
                path_params.push(param.to_string());
            } else if let Some(slug) = seg.strip_prefix('*') {
                openapi_path.push('{');
                openapi_path.push_str(slug);
                openapi_path.push('}');
                path_params.push(slug.to_string());
            } else {
                openapi_path.push_str(seg);
            }
        }
        if openapi_path.is_empty() {
            openapi_path = "/".to_string();
        }

        let mut get_op = serde_json::json!({
            "summary": format!("Handler for {}", route),
            "operationId": format!("get_{}", route.replace('/', "_").replace([':', '*', '{', '}'], "")),
            "responses": {
                "200": {
                    "description": "Successful HTML or JSON response",
                    "content": {
                        "text/html": {
                            "schema": { "type": "string" }
                        }
                    }
                }
            }
        });

        if !path_params.is_empty() {
            let params_json: Vec<serde_json::Value> = path_params
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p,
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    })
                })
                .collect();
            get_op["parameters"] = serde_json::Value::Array(params_json);
        }

        let mut path_item = serde_json::Map::new();
        path_item.insert("get".to_string(), get_op);
        paths_obj.insert(openapi_path, serde_json::Value::Object(path_item));
    }

    let spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Yav Application API",
            "description": "Auto-generated OpenAPI specification from Yav routes",
            "version": "1.0.0"
        },
        "paths": paths_obj
    });

    let json_text = serde_json::to_string_pretty(&spec)?;
    fs::write(&out_path, json_text)?;
    println!(
        "\x1b[32m✔ Exported OpenAPI specification:\x1b[0m {}",
        out_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_pascal_case("user-card"), "UserCard");
        assert_eq!(to_pascal_case("stat_grid"), "StatGrid");
        assert_eq!(to_snake_case("UserCard"), "user_card");
        assert_eq!(to_snake_case("user-card"), "user_card");
        assert_eq!(to_title_case("users/[id]"), "Users Id");
    }

    #[test]
    fn test_scan_and_generate_artifacts() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_test_codegen_{}", std::process::id()));
        let routes_dir = temp_dir.join("src").join("routes");
        fs::create_dir_all(&routes_dir).unwrap();

        fs::write(routes_dir.join("page.rs"), "// index").unwrap();
        fs::write(routes_dir.join("about.rs"), "// about").unwrap();

        let patterns = scan_route_patterns(&temp_dir);
        assert!(patterns.contains(&"/".to_string()));
        assert!(patterns.contains(&"/about".to_string()));

        let ts_path = temp_dir.join("types.d.ts");
        generate_typescript(&temp_dir, Some(ts_path.to_str().unwrap().to_string())).unwrap();
        let ts_content = fs::read_to_string(&ts_path).unwrap();
        assert!(ts_content.contains("export type YavRoute ="));
        assert!(ts_content.contains("\"/\""));
        assert!(ts_content.contains("\"/about\""));

        let api_path = temp_dir.join("openapi.json");
        generate_openapi(&temp_dir, Some(api_path.to_str().unwrap().to_string())).unwrap();
        let api_content = fs::read_to_string(&api_path).unwrap();
        assert!(api_content.contains("\"openapi\": \"3.0.3\""));
        assert!(api_content.contains("\"/about\""));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
