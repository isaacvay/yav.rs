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

/// Parse input for `typed_routes!()` or `typed_routes!("src/routes")`.
pub struct TypedRoutesInput {
    pub path: Option<LitStr>,
}

impl syn::parse::Parse for TypedRoutesInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self { path: None })
        } else {
            let lit: LitStr = input.parse()?;
            Ok(Self { path: Some(lit) })
        }
    }
}

pub fn typed_routes_impl(input: TypedRoutesInput) -> syn::Result<TokenStream> {
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

    let mut const_items = Vec::new();
    let mut used_names = std::collections::HashSet::new();
    for (url_pattern, rel_path) in route_entries {
        let stripped = rel_path
            .trim_end_matches(".rs")
            .trim_end_matches(".yav")
            .trim_end_matches(".rsx");
        let without_index = stripped
            .strip_suffix("/index")
            .or_else(|| stripped.strip_suffix("/page"))
            .unwrap_or(stripped);
        let raw_clean: String = without_index
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else if c == '/' || c == '\\' || c == '-' || c == '.' || c == '_' {
                    '_'
                } else {
                    // Strip '[', ']', '(', ')'
                    '\0'
                }
            })
            .filter(|&c| c != '\0')
            .collect();
        let collapsed: String = raw_clean
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_");

        let base_name = if collapsed.is_empty() || collapsed == "INDEX" || collapsed == "PAGE" {
            "ROOT".to_string()
        } else if collapsed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            format!("ROUTE_{collapsed}")
        } else {
            collapsed
        };

        let mut final_name = base_name.clone();
        let mut suffix = 2usize;
        while !used_names.insert(final_name.clone()) {
            final_name = format!("{base_name}_{suffix}");
            suffix += 1;
        }

        let const_ident = syn::parse_str::<syn::Ident>(&final_name).unwrap_or_else(|_| {
            syn::Ident::new(
                &format!("ROUTE_{}", used_names.len()),
                proc_macro2::Span::call_site(),
            )
        });
        let fn_name_str = final_name.to_ascii_lowercase();
        let fn_ident = syn::parse_str::<syn::Ident>(&fn_name_str).unwrap_or_else(|_| {
            syn::Ident::new(
                &format!("route_{}", used_names.len()),
                proc_macro2::Span::call_site(),
            )
        });

        let pattern_str = url_pattern.as_str();
        let mut param_names = Vec::new();
        for seg in pattern_str.split('/') {
            if let Some(p) = seg.strip_prefix(':').or_else(|| seg.strip_prefix('*')) {
                if !p.is_empty() {
                    param_names.push(p.to_string());
                }
            }
        }

        let fn_item = if param_names.is_empty() {
            quote! {
                pub fn #fn_ident() -> String {
                    #pattern_str.to_string()
                }
            }
        } else {
            let param_idents: Vec<syn::Ident> = param_names
                .iter()
                .map(|p| sanitize_module_ident(p))
                .collect();
            let pairs: Vec<TokenStream> = param_names
                .iter()
                .zip(param_idents.iter())
                .map(|(raw_p, id)| {
                    quote! { (#raw_p, &#id.to_string()) }
                })
                .collect();
            quote! {
                pub fn #fn_ident(#( #param_idents: impl ::std::fmt::Display ),*) -> String {
                    let __pairs: &[(&str, &str)] = &[ #( #pairs ),* ];
                    ::yav::router::url_for(#pattern_str, __pairs)
                }
            }
        };

        const_items.push(quote! {
            pub const #const_ident: &'static str = #pattern_str;
            #fn_item
        });
    }

    Ok(quote! {
        pub struct Routes;
        impl Routes {
            #( #const_items )*
        }
    })
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
    let mut registered_server_fns = std::collections::HashSet::new();

    for (url_pattern, rel_path_str) in route_entries {
        let file_path = routes_dir.join(&rel_path_str);
        if let Ok(content) = fs::read_to_string(&file_path) {
            if rel_path_str.ends_with(".yav") || rel_path_str.ends_with(".rsx") {
                let abs_str = file_path
                    .canonicalize()
                    .unwrap_or_else(|_| file_path.clone())
                    .to_string_lossy()
                    .to_string();
                let (fm, template) = crate::file_component::split_frontmatter(&content);
                let fm_tokens: TokenStream = if let Some(fm_str) = fm {
                    let transpiled = crate::file_component::transpile_frontmatter(fm_str);
                    transpiled.parse().unwrap_or_default()
                } else {
                    TokenStream::new()
                };
                if let Ok(tpl_tokens) = template.parse::<TokenStream>() {
                    if let Ok(rsx_node) = crate::parser::parse_rsx(tpl_tokens) {
                        let generated = crate::codegen::generate(rsx_node);
                        let pattern = url_pattern.as_str();

                        // Nested layout hierarchy wrapping
                        let layouts = find_layout_modules(&routes_dir, &rel_path_str);
                        let mut wrapped_node = quote! { #generated };
                        for layout_mod in layouts {
                            wrapped_node = quote! {
                                routes::#( #layout_mod )::* ::layout(#wrapped_node)
                            };
                        }

                        registrations.push(quote! {
                            .get(#pattern, |_req: ::axum::extract::Request| async move {
                                const _: &[u8] = include_bytes!(#abs_str);
                                #fm_tokens
                                let __node = #wrapped_node;
                                ::yav::server::HtmlPage::new(__node)
                            })
                        });
                    }
                }
            } else if let Ok(syntax_file) = syn::parse_file(&content) {
                let mod_path_segments: Vec<syn::Ident> = rel_path_str
                    .trim_end_matches(".rs")
                    .split('/')
                    .map(sanitize_module_ident)
                    .collect();

                let layouts = find_layout_modules(&routes_dir, &rel_path_str);
                let middlewares = find_middleware_modules(&routes_dir, &rel_path_str);

                let stem = rel_path_str
                    .split('/')
                    .next_back()
                    .unwrap_or("")
                    .trim_end_matches(".rs");

                for item in syntax_file.items {
                    if let syn::Item::Fn(fn_item) = item {
                        if matches!(fn_item.vis, syn::Visibility::Public(_)) {
                            let param_count = fn_item.sig.inputs.len();
                            let single_non_req = param_count == 1
                                && !is_axum_request_param(fn_item.sig.inputs.first().unwrap());
                            let fn_ident = fn_item.sig.ident;
                            let fn_name = fn_ident.to_string();

                            // Auto-register #[server] / #[yav::server] RPC functions as POST endpoints
                            let has_server_attr = fn_item.attrs.iter().any(|attr| {
                                attr.path()
                                    .segments
                                    .last()
                                    .is_some_and(|seg| seg.ident == "server")
                            });
                            if has_server_attr {
                                let server_key = format!("{rel_path_str}::{fn_name}");
                                if registered_server_fns.insert(server_key) {
                                    let url_fn_ident = quote::format_ident!("{}_url", fn_ident);
                                    let handler_fn_ident =
                                        quote::format_ident!("{}_handler", fn_ident);
                                    let url_call = quote! { routes::#( #mod_path_segments )::* :: #url_fn_ident() };
                                    let handler_call = quote! { routes::#( #mod_path_segments )::* :: #handler_fn_ident() };
                                    registrations.push(quote! {
                                        .post(#url_call, #handler_call)
                                    });
                                }
                                continue;
                            }

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
                                } else if fn_name == "patch" || fn_name.ends_with("_patch") {
                                    Some((
                                        url_pattern.as_str(),
                                        syn::Ident::new("PATCH", proc_macro2::Span::call_site()),
                                    ))
                                } else if fn_name == "handler"
                                    || fn_name == "page"
                                    || fn_name == stem
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
                                let fn_path =
                                    quote! { routes::#( #mod_path_segments )::* :: #fn_ident };
                                let handler_expr = if param_count == 2 {
                                    quote! { ::yav::server::BoxedHandler::from_extractor2(#fn_path) }
                                } else if single_non_req {
                                    quote! { ::yav::server::BoxedHandler::from_extractor1(#fn_path) }
                                } else if !middlewares.is_empty() {
                                    if param_count == 0 {
                                        quote! { ::yav::server::BoxedHandler::from_zero_arg(#fn_path) }
                                    } else {
                                        quote! { ::yav::server::BoxedHandler::new(#fn_path) }
                                    }
                                } else {
                                    fn_path
                                };

                                let mut mw_wrapped = handler_expr;
                                for mw_mod in &middlewares {
                                    mw_wrapped = quote! {
                                        #mw_wrapped.with_middleware(routes::#( #mw_mod )::* ::middleware)
                                    };
                                }

                                if layouts.is_empty() {
                                    registrations.push(quote! {
                                        .route(#pattern, ::axum::http::Method::#method_ident, #mw_wrapped)
                                    });
                                } else {
                                    let mut wrapped_node = quote! { __node };
                                    for layout_mod in &layouts {
                                        wrapped_node = quote! {
                                            routes::#( #layout_mod )::* ::layout(#wrapped_node)
                                        };
                                    }
                                    registrations.push(quote! {
                                        .route_with_layout(
                                            #pattern,
                                            ::axum::http::Method::#method_ident,
                                            #mw_wrapped,
                                            |__node| #wrapped_node
                                        )
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for not_found handler in routes_dir
    let not_found_rs = routes_dir.join("not_found.rs");
    let not_found_yav = routes_dir.join("not_found.yav");
    let not_found_rsx = routes_dir.join("not_found.rsx");

    if not_found_rs.exists() {
        if let Ok(content) = fs::read_to_string(&not_found_rs) {
            if let Ok(syntax_file) = syn::parse_file(&content) {
                let mut not_found_fn = None;
                for item in syntax_file.items {
                    if let syn::Item::Fn(fn_item) = item {
                        if matches!(fn_item.vis, syn::Visibility::Public(_)) {
                            let ident = fn_item.sig.ident;
                            let name = ident.to_string();
                            if name == "not_found"
                                || name == "handler"
                                || name == "page"
                                || name == "get"
                                || name == "not_found_handler"
                            {
                                not_found_fn = Some(ident);
                                break;
                            }
                        }
                    }
                }
                if let Some(fn_ident) = not_found_fn {
                    let nf_layouts = find_layout_modules(&routes_dir, "not_found.rs");
                    if nf_layouts.is_empty() {
                        registrations.push(quote! {
                            .not_found_handler(routes::not_found::#fn_ident)
                        });
                    } else {
                        let mut wrapped_node = quote! { __node };
                        for layout_mod in &nf_layouts {
                            wrapped_node = quote! {
                                routes::#( #layout_mod )::* ::layout(#wrapped_node)
                            };
                        }
                        registrations.push(quote! {
                            .not_found_handler_with_layout(
                                routes::not_found::#fn_ident,
                                |__node| #wrapped_node
                            )
                        });
                    }
                }
            }
        }
    } else if not_found_yav.exists() || not_found_rsx.exists() {
        let nf_path = if not_found_yav.exists() {
            not_found_yav
        } else {
            not_found_rsx
        };
        if let Ok(content) = fs::read_to_string(&nf_path) {
            let abs_str = nf_path
                .canonicalize()
                .unwrap_or_else(|_| nf_path.clone())
                .to_string_lossy()
                .to_string();
            let (fm, template) = crate::file_component::split_frontmatter(&content);
            let fm_tokens: TokenStream = if let Some(fm_str) = fm {
                let transpiled = crate::file_component::transpile_frontmatter(fm_str);
                transpiled.parse().unwrap_or_default()
            } else {
                TokenStream::new()
            };
            if let Ok(tpl_tokens) = template.parse::<TokenStream>() {
                if let Ok(rsx_node) = crate::parser::parse_rsx(tpl_tokens) {
                    let generated = crate::codegen::generate(rsx_node);
                    let nf_layouts = find_layout_modules(&routes_dir, "not_found.yav");
                    let mut wrapped_node = quote! { #generated };
                    for layout_mod in nf_layouts {
                        wrapped_node = quote! {
                            routes::#( #layout_mod )::* ::layout(#wrapped_node)
                        };
                    }
                    registrations.push(quote! {
                        .not_found_handler(|_req: ::axum::extract::Request| async move {
                            const _: &[u8] = include_bytes!(#abs_str);
                            #fm_tokens
                            let __node = #wrapped_node;
                            ::yav::server::HtmlPage::new(__node).status(::axum::http::StatusCode::NOT_FOUND)
                        })
                    });
                }
            }
        }
    }

    // Auto-mount sitemap.rs as GET /sitemap.xml
    let sitemap_rs = routes_dir.join("sitemap.rs");
    if sitemap_rs.exists() {
        if let Ok(content) = fs::read_to_string(&sitemap_rs) {
            if let Ok(syntax_file) = syn::parse_file(&content) {
                let mut sitemap_fn = None;
                for item in syntax_file.items {
                    if let syn::Item::Fn(fn_item) = item {
                        if matches!(fn_item.vis, syn::Visibility::Public(_)) {
                            let ident = fn_item.sig.ident;
                            let name = ident.to_string();
                            if name == "sitemap" || name == "handler" || name == "get" {
                                sitemap_fn = Some(ident);
                                break;
                            }
                        }
                    }
                }
                if let Some(fn_ident) = sitemap_fn {
                    registrations.push(quote! {
                        .route("/sitemap.xml", ::axum::http::Method::GET, routes::sitemap::#fn_ident)
                    });
                }
            }
        }
    }

    // Auto-mount robots.rs as GET /robots.txt
    let robots_rs = routes_dir.join("robots.rs");
    if robots_rs.exists() {
        if let Ok(content) = fs::read_to_string(&robots_rs) {
            if let Ok(syntax_file) = syn::parse_file(&content) {
                let mut robots_fn = None;
                for item in syntax_file.items {
                    if let syn::Item::Fn(fn_item) = item {
                        if matches!(fn_item.vis, syn::Visibility::Public(_)) {
                            let ident = fn_item.sig.ident;
                            let name = ident.to_string();
                            if name == "robots" || name == "handler" || name == "get" {
                                robots_fn = Some(ident);
                                break;
                            }
                        }
                    }
                }
                if let Some(fn_ident) = robots_fn {
                    registrations.push(quote! {
                        .route("/robots.txt", ::axum::http::Method::GET, routes::robots::#fn_ident)
                    });
                }
            }
        }
    }

    Ok(quote! {
        #app_expr
            #( #registrations )*
    })
}

fn sanitize_module_ident(seg: &str) -> syn::Ident {
    let clean = seg
        .replace('@', "slot_")
        .replace(['[', ']', '(', ')'], "")
        .replace(['.', '-'], "_");
    let safe_ident = if clean.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("route_{clean}")
    } else if clean.is_empty() {
        "param".to_string()
    } else {
        clean
    };
    syn::parse_str::<syn::Ident>(&safe_ident)
        .unwrap_or_else(|_| syn::Ident::new("param", proc_macro2::Span::call_site()))
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
                    || file_name == "middleware.rs"
                    || file_name == "loading.rs"
                    || file_name == "loading.yav"
                    || file_name == "loading.rsx"
                    || file_name == "error.rs"
                    || file_name == "error.yav"
                    || file_name == "error.rsx"
                    || file_name == "not_found.rs"
                    || file_name == "not_found.yav"
                    || file_name == "not_found.rsx"
                    || file_name == "sitemap.rs"
                    || file_name == "sitemap.yav"
                    || file_name == "sitemap.rsx"
                    || file_name == "robots.rs"
                    || file_name == "mod.rs"
                {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    let pattern = file_path_to_pattern(rel);
                    let clean_rel = rel.to_string_lossy().replace('\\', "/");
                    if let Some(base_pattern) = optional_catch_all_base_pattern(rel) {
                        acc.push((base_pattern, clean_rel.clone()));
                    }
                    acc.push((pattern, clean_rel));
                }
            }
        }
    }
}

fn find_layout_modules(routes_dir: &Path, rel_path: &str) -> Vec<Vec<syn::Ident>> {
    let mut layouts = Vec::new();
    let rel = Path::new(rel_path);
    let mut current = rel.parent();

    while let Some(dir) = current {
        let dir_str = dir.to_string_lossy().replace('\\', "/");
        if !dir_str.is_empty() {
            let layout_rs = routes_dir.join(dir).join("_layout.rs");
            let alt_layout_rs = routes_dir.join(dir).join("layout.rs");
            if layout_rs.exists() || alt_layout_rs.exists() {
                let layout_file_name = if layout_rs.exists() {
                    "_layout"
                } else {
                    "layout"
                };
                let mut segments: Vec<syn::Ident> =
                    dir_str.split('/').map(sanitize_module_ident).collect();
                segments.push(syn::Ident::new(
                    layout_file_name,
                    proc_macro2::Span::call_site(),
                ));
                layouts.push(segments);
            }
        }
        current = dir.parent();
    }

    // Check root layout in routes_dir
    let root_layout_rs = routes_dir.join("_layout.rs");
    let alt_root_layout_rs = routes_dir.join("layout.rs");
    if root_layout_rs.exists() || alt_root_layout_rs.exists() {
        let root_name = if root_layout_rs.exists() {
            "_layout"
        } else {
            "layout"
        };
        layouts.push(vec![syn::Ident::new(
            root_name,
            proc_macro2::Span::call_site(),
        )]);
    }

    layouts
}

fn find_middleware_modules(routes_dir: &Path, rel_path: &str) -> Vec<Vec<syn::Ident>> {
    let mut middlewares = Vec::new();
    let rel = Path::new(rel_path);
    let mut current = rel.parent();

    while let Some(dir) = current {
        let dir_str = dir.to_string_lossy().replace('\\', "/");
        if !dir_str.is_empty() {
            let mw_rs = routes_dir.join(dir).join("_middleware.rs");
            let alt_mw_rs = routes_dir.join(dir).join("middleware.rs");
            if mw_rs.exists() || alt_mw_rs.exists() {
                let mw_file_name = if mw_rs.exists() {
                    "_middleware"
                } else {
                    "middleware"
                };
                let mut segments: Vec<syn::Ident> =
                    dir_str.split('/').map(sanitize_module_ident).collect();
                segments.push(syn::Ident::new(
                    mw_file_name,
                    proc_macro2::Span::call_site(),
                ));
                middlewares.push(segments);
            }
        }
        current = dir.parent();
    }

    let root_mw_rs = routes_dir.join("_middleware.rs");
    let alt_root_mw_rs = routes_dir.join("middleware.rs");
    if root_mw_rs.exists() || alt_root_mw_rs.exists() {
        let root_name = if root_mw_rs.exists() {
            "_middleware"
        } else {
            "middleware"
        };
        middlewares.push(vec![syn::Ident::new(
            root_name,
            proc_macro2::Span::call_site(),
        )]);
    }

    middlewares
}

fn is_axum_request_param(arg: &syn::FnArg) -> bool {
    if let syn::FnArg::Typed(pat_type) = arg {
        if let syn::Type::Path(type_path) = &*pat_type.ty {
            if let Some(last) = type_path.path.segments.last() {
                return last.ident == "Request"
                    && matches!(last.arguments, syn::PathArguments::None);
            }
        }
    }
    false
}

fn optional_catch_all_base_pattern(rel: &Path) -> Option<String> {
    let file_name = rel.file_name().and_then(|n| n.to_str())?;
    let clean = file_name
        .strip_suffix(".rs")
        .or_else(|| file_name.strip_suffix(".yav"))
        .or_else(|| file_name.strip_suffix(".rsx"))
        .unwrap_or(file_name);
    let is_opt = (clean.starts_with("[[...") && clean.ends_with("]]") && clean.len() > 7)
        || (clean.starts_with("___") && clean.ends_with("___") && clean.len() > 6);
    if !is_opt {
        return None;
    }
    let parent = rel.parent()?;
    if parent.as_os_str().is_empty() {
        Some("/".to_string())
    } else {
        Some(file_path_to_pattern(parent))
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
        // Ignore parallel route slots (@analytics, @modal)
        if clean.starts_with('@') && clean.len() > 1 {
            continue;
        }
        // Handle intercepting route prefixes (.) (..) (..)(..) (...)
        let mut clean = clean;
        if let Some(rest) = clean.strip_prefix("(...)") {
            segments.clear();
            clean = rest;
        } else if let Some(rest) = clean.strip_prefix("(..)(..)") {
            segments.pop();
            segments.pop();
            clean = rest;
        } else if let Some(rest) = clean.strip_prefix("(..)") {
            segments.pop();
            clean = rest;
        } else if let Some(rest) = clean.strip_prefix("(.)") {
            clean = rest;
        }
        if clean.is_empty() {
            continue;
        }
        // Route group: (marketing) -> transparent
        if clean.starts_with('(') && clean.ends_with(')') && clean.len() >= 2 {
            continue;
        }
        // Optional catch-all [[...slug]] or ___slug___ -> *slug
        if clean.starts_with("[[...") && clean.ends_with("]]") && clean.len() > 7 {
            let slug = &clean[5..clean.len() - 2];
            segments.push(format!("*{slug}"));
        } else if clean.starts_with("___") && clean.ends_with("___") && clean.len() > 6 {
            let slug = &clean[3..clean.len() - 3];
            segments.push(format!("*{slug}"));
        } else if clean.starts_with('[') && clean.ends_with(']') && clean.len() >= 2 {
            // [id] -> :id or [...slug] -> *slug
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
        // Optional catch-all [[...slug]] and ___slug___
        assert_eq!(
            file_path_to_pattern(Path::new("shop/[[...slug]].rs")),
            "/shop/*slug"
        );
        assert_eq!(
            optional_catch_all_base_pattern(Path::new("shop/[[...slug]].rs")),
            Some("/shop".to_string())
        );
        assert_eq!(
            file_path_to_pattern(Path::new("docs/___slug___.rs")),
            "/docs/*slug"
        );
        assert_eq!(
            optional_catch_all_base_pattern(Path::new("___slug___.rs")),
            Some("/".to_string())
        );
        // Parallel route slots (@analytics) and intercepting routes ((.)photo)
        assert_eq!(
            file_path_to_pattern(Path::new("dashboard/@analytics/page.rs")),
            "/dashboard"
        );
        assert_eq!(
            file_path_to_pattern(Path::new("feed/(.)photo/[id].rs")),
            "/feed/photo/:id"
        );
    }

    #[test]
    fn test_auto_routes_registers_server_functions_and_optional_catch_all() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_routes_macro_srv_{}", std::process::id()));
        let _ = fs::create_dir_all(temp_dir.join("shop"));
        fs::write(
            temp_dir.join("shop").join("[[...slug]].rs"),
            r#"
            pub async fn get() -> String { "shop".into() }
            #[server("/_actions/checkout")]
            pub async fn checkout() -> String { "ok".into() }
            "#,
        )
        .unwrap();

        let input_path = syn::LitStr::new(
            temp_dir.to_str().unwrap().replace('\\', "/").as_str(),
            proc_macro2::Span::call_site(),
        );
        let app_expr: syn::Expr = syn::parse_str("app").unwrap();
        let input = AutoRoutesInput {
            app_expr,
            path: Some(input_path),
        };

        let generated = auto_routes_impl(input).unwrap().to_string();
        // Registers both /shop and /shop/*slug for optional catch-all
        assert!(generated.contains("\"/shop\""));
        assert!(generated.contains("\"/shop/*slug\""));
        // Auto-registers #[server] function via checkout_url() and checkout_handler()
        assert!(generated.contains("checkout_url"));
        assert!(generated.contains("checkout_handler"));

        let _ = fs::remove_dir_all(&temp_dir);
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

    #[test]
    fn test_auto_routes_impl_with_not_found() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_routes_macro_nf_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        fs::write(
            temp_dir.join("not_found.rs"),
            "pub async fn not_found() -> String { \"404\".into() }",
        )
        .unwrap();

        let input_path = syn::LitStr::new(
            temp_dir.to_str().unwrap().replace('\\', "/").as_str(),
            proc_macro2::Span::call_site(),
        );
        let app_expr: syn::Expr = syn::parse_str("app").unwrap();
        let input = AutoRoutesInput {
            app_expr,
            path: Some(input_path),
        };

        let generated = auto_routes_impl(input).unwrap().to_string();
        assert!(generated.contains("not_found_handler"));
        assert!(generated.contains("routes :: not_found :: not_found"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_auto_routes_impl_with_sitemap_and_robots() {
        let temp_dir =
            std::env::temp_dir().join(format!("yav_routes_macro_sr_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        fs::write(
            temp_dir.join("sitemap.rs"),
            "pub async fn sitemap() -> String { \"<xml></xml>\".into() }",
        )
        .unwrap();
        fs::write(
            temp_dir.join("robots.rs"),
            "pub async fn robots() -> String { \"User-agent: *\".into() }",
        )
        .unwrap();

        let input_path = syn::LitStr::new(
            temp_dir.to_str().unwrap().replace('\\', "/").as_str(),
            proc_macro2::Span::call_site(),
        );
        let app_expr: syn::Expr = syn::parse_str("app").unwrap();
        let input = AutoRoutesInput {
            app_expr,
            path: Some(input_path),
        };

        let generated = auto_routes_impl(input).unwrap().to_string();
        assert!(generated.contains("\"/sitemap.xml\""));
        assert!(generated.contains("routes :: sitemap :: sitemap"));
        assert!(generated.contains("\"/robots.txt\""));
        assert!(generated.contains("routes :: robots :: robots"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
