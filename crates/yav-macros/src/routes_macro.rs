//! Compile-time route directory scanner and auto-wiring macros.
//!
//! Scans a route directory at compile-time to discover routes and automatically wire
//! them onto a `YavApp` instance without requiring tedious manual registration.

use proc_macro2::TokenStream;
use quote::quote;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use syn::LitStr;

/// Parse optional directory path string or default to `"src/routes"`.
pub struct IncludeRoutesInput {
    pub path: Option<LitStr>,
}

impl syn::parse::Parse for IncludeRoutesInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self { path: None })
        } else {
            let lit: LitStr = input.parse()?;
            Ok(Self { path: Some(lit) })
        }
    }
}

pub fn include_routes_impl(input: IncludeRoutesInput) -> syn::Result<TokenStream> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    let rel_dir = input
        .path
        .map(|lit| lit.value())
        .unwrap_or_else(|| "src/routes".to_string());

    let routes_dir = PathBuf::from(&manifest_dir).join(&rel_dir);

    let mut route_entries = Vec::new();
    if routes_dir.exists() && routes_dir.is_dir() {
        collect_routes_recursive(&routes_dir, &routes_dir, &mut route_entries);
    }

    // Sort routes for deterministic output
    route_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let patterns: Vec<&str> = route_entries.iter().map(|(p, _)| p.as_str()).collect();
    let paths: Vec<&str> = route_entries.iter().map(|(_, f)| f.as_str()).collect();

    let tokens = quote! {
        &[
            #( (#patterns, #paths) ),*
        ] as &[(&'static str, &'static str)]
    };

    Ok(tokens)
}

/// Parse input for `auto_routes!(app)` or `auto_routes!(app, "src/routes")`.
pub struct AutoRoutesInput {
    pub app_expr: syn::Expr,
    pub path: Option<LitStr>,
}

impl syn::parse::Parse for AutoRoutesInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let app_expr: syn::Expr = input.parse()?;
        let path = if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            Some(input.parse::<syn::LitStr>()?)
        } else {
            None
        };
        Ok(Self { app_expr, path })
    }
}

pub fn auto_routes_impl(input: AutoRoutesInput) -> syn::Result<TokenStream> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let rel_dir = input
        .path
        .map(|lit| lit.value())
        .unwrap_or_else(|| "src/routes".to_string());
    let routes_dir = PathBuf::from(&manifest_dir).join(&rel_dir);

    let mut route_entries = Vec::new();
    if routes_dir.exists() && routes_dir.is_dir() {
        collect_routes_recursive(&routes_dir, &routes_dir, &mut route_entries);
    }
    route_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let app_expr = input.app_expr;
    let mut registrations = Vec::new();

    for (url_pattern, rel_path_str) in route_entries {
        let file_path = routes_dir.join(&rel_path_str);
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(syntax_file) = syn::parse_file(&content) {
                let mod_path_segments: Vec<syn::Ident> = rel_path_str
                    .trim_end_matches(".rs")
                    .split('/')
                    .map(|seg| {
                        let clean = seg.replace('[', "").replace(']', "").replace('.', "_");
                        syn::Ident::new(&clean, proc_macro2::Span::call_site())
                    })
                    .collect();

                let stem = rel_path_str
                    .split('/')
                    .last()
                    .unwrap_or("")
                    .trim_end_matches(".rs");

                for item in syntax_file.items {
                    if let syn::Item::Fn(fn_item) = item {
                        if matches!(fn_item.vis, syn::Visibility::Public(_)) {
                            let fn_ident = fn_item.sig.ident;
                            let fn_name = fn_ident.to_string();

                            let method_and_pattern: Option<(&str, syn::Ident)> =
                                if fn_name == "get" || fn_name.ends_with("_get") {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("GET", proc_macro2::Span::call_site()),
                                    ))
                                } else if fn_name == "post" || fn_name.ends_with("_post") {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("POST", proc_macro2::Span::call_site()),
                                    ))
                                } else if fn_name == "put" || fn_name.ends_with("_put") {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("PUT", proc_macro2::Span::call_site()),
                                    ))
                                } else if fn_name == "delete" || fn_name.ends_with("_delete") {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("DELETE", proc_macro2::Span::call_site()),
                                    ))
                                } else if fn_name == "handler"
                                    || fn_name == "page"
                                    || fn_name == stem
                                    || fn_name.starts_with(stem)
                                    || (stem == "index" && fn_name == "index")
                                    || (stem.starts_with('_')
                                        && (fn_name == "detail" || fn_name.ends_with("_detail")))
                                {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("GET", proc_macro2::Span::call_site()),
                                    ))
                                } else {
                                    None
                                };

                            if let Some((pattern, method_ident)) = method_and_pattern {
                                registrations.push(quote! {
                                    .route(#pattern, ::axum::http::Method::#method_ident, routes::#( #mod_path_segments )::* :: #fn_ident)
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(quote! {
        #app_expr
            #( #registrations )*
    })
}

fn collect_routes_recursive(base_dir: &Path, current_dir: &Path, acc: &mut Vec<(String, String)>) {
    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_routes_recursive(base_dir, &path, acc);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "rs" || ext == "yav" || ext == "rsx")
            {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if (file_name.starts_with('_') && !file_name.ends_with("_.rs"))
                    || file_name == "layout.rs"
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
                {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    let pattern = file_path_to_pattern(rel);
                    let clean_rel = rel.to_string_lossy().replace('\\', "/");
                    acc.push((pattern, clean_rel));
                }
            }
        }
    }
}

fn file_path_to_pattern(rel: &Path) -> String {
    let mut segments = Vec::new();
    for comp in rel.components() {
        let s = comp.as_os_str().to_str().unwrap_or("");
        let clean = s
            .strip_suffix(".rs")
            .or_else(|| s.strip_suffix(".yav"))
            .or_else(|| s.strip_suffix(".rsx"))
            .unwrap_or(s);
        if clean == "page" || clean == "index" || clean == "mod" {
            continue;
        }
        if clean.starts_with('(') && clean.ends_with(')') && clean.len() >= 2 {
            continue;
        }
        // [id] -> :id or _id_ -> :id (Rust-friendly identifier)
        if clean.starts_with('[') && clean.ends_with(']') && clean.len() >= 2 {
            let inner = &clean[1..clean.len() - 1];
            if let Some(slug) = inner.strip_prefix("...") {
                segments.push(format!("*{}", slug));
            } else {
                segments.push(format!(":{}", inner));
            }
        } else if clean.starts_with("__") && clean.ends_with("__") && clean.len() > 4 {
            let inner = &clean[2..clean.len() - 2];
            segments.push(format!("*{}", inner));
        } else if clean.starts_with('_') && clean.ends_with('_') && clean.len() > 2 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_to_pattern() {
        assert_eq!(file_path_to_pattern(Path::new("page.rs")), "/");
        assert_eq!(file_path_to_pattern(Path::new("index.rs")), "/");
        assert_eq!(file_path_to_pattern(Path::new("about.rs")), "/about");
        assert_eq!(
            file_path_to_pattern(Path::new("users/[id].rs")),
            "/users/:id"
        );
        assert_eq!(
            file_path_to_pattern(Path::new("users/_id_.rs")),
            "/users/:id"
        );
        assert_eq!(
            file_path_to_pattern(Path::new("blog/[...slug].rs")),
            "/blog/*slug"
        );
        assert_eq!(
            file_path_to_pattern(Path::new("docs/__slug__.rs")),
            "/docs/*slug"
        );
    }

    #[test]
    fn test_collect_routes_recursive_filtering() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_routes_macro_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("sub"));
        fs::write(temp_dir.join("mod.rs"), "// mod").unwrap();
        fs::write(temp_dir.join("layout.rs"), "// layout").unwrap();
        fs::write(temp_dir.join("_private.rs"), "// private").unwrap();
        fs::write(temp_dir.join("page.rs"), "// page").unwrap();
        fs::write(temp_dir.join("sub").join("mod.rs"), "// sub mod").unwrap();
        fs::write(temp_dir.join("sub").join("page.rs"), "// sub page").unwrap();

        let mut acc = Vec::new();
        collect_routes_recursive(&temp_dir, &temp_dir, &mut acc);
        acc.sort();

        assert_eq!(acc.len(), 2);
        assert_eq!(acc[0], ("/".to_string(), "page.rs".to_string()));
        assert_eq!(acc[1], ("/sub".to_string(), "sub/page.rs".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
