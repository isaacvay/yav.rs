//! File-based component and template processing for `.rsx` and `.yav` files.
//!
//! Provides the backend implementation for:
//! - `include_rsx!("path/to/template.rsx")` / `include_yav!("path/to/template.yav")`
//! - `yav_component!("path/to/Component.yav")`

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::path::PathBuf;
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result,
};

/// Input to `include_rsx!("...")` and `include_yav!("...")`.
pub struct IncludeFileInput {
    pub path: LitStr,
}

impl Parse for IncludeFileInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(IncludeFileInput {
            path: input.parse()?,
        })
    }
}

/// Input to `yav_component!(...)`.
/// Supports:
/// - `yav_component!("path/to/MyComponent.yav")` -> struct name inferred as `MyComponent`
/// - `yav_component!(CustomName, "path/to/file.yav")` -> struct named `CustomName`
pub struct YavComponentInput {
    pub name: Option<Ident>,
    pub path: LitStr,
}

impl Parse for YavComponentInput {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Ident) && input.peek2(syn::Token![,]) {
            let name: Ident = input.parse()?;
            let _: syn::Token![,] = input.parse()?;
            let path: LitStr = input.parse()?;
            Ok(YavComponentInput {
                name: Some(name),
                path,
            })
        } else {
            let path: LitStr = input.parse()?;
            Ok(YavComponentInput { name: None, path })
        }
    }
}

/// Resolves a file path relative to `CARGO_MANIFEST_DIR` or current working directory.
pub fn resolve_path(lit: &LitStr) -> Result<PathBuf> {
    let rel = lit.value();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let candidate = PathBuf::from(&manifest_dir).join(&rel);
    if candidate.exists() {
        return Ok(candidate);
    }

    let direct = PathBuf::from(&rel);
    if direct.exists() {
        return Ok(direct);
    }

    Err(syn::Error::new(
        lit.span(),
        format!(
            "Template/Component file '{}' not found (checked '{}' and '{}')",
            rel,
            candidate.display(),
            direct.display()
        ),
    ))
}

/// Splits a file content into frontmatter (between `---` and `---`) and template body.
pub fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let trimmed = content.trim_start();
    if let Some(after_first) = trimmed.strip_prefix("---") {
        let first_newline = after_first.find('\n').unwrap_or(0);
        let body_start = if first_newline > 0 {
            first_newline + 1
        } else {
            0
        };
        let remaining = &after_first[body_start..];

        let mut curr_idx = 0;
        for line in remaining.lines() {
            if line.trim() == "---" {
                let fm = remaining[..curr_idx].trim();
                let rest = &remaining[curr_idx + line.len()..];
                let template = rest.trim_start_matches(['\r', '\n']).trim();
                return (Some(fm), template);
            }
            curr_idx += line.len();
            if remaining[curr_idx..].starts_with("\r\n") {
                curr_idx += 2;
            } else if remaining[curr_idx..].starts_with('\n') {
                curr_idx += 1;
            }
        }
    }
    (None, content.trim())
}

/// Implementation of `include_rsx!` and `include_yav!`.
pub fn include_template_impl(input: IncludeFileInput) -> Result<TokenStream> {
    let path = resolve_path(&input.path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| {
        syn::Error::new(
            input.path.span(),
            format!("Failed to read file '{}': {}", path.display(), e),
        )
    })?;

    let abs_str = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .to_string_lossy()
        .to_string();

    let (_, template) = split_frontmatter(&content);
    let tokens: TokenStream = template.parse().map_err(|e: proc_macro2::LexError| {
        syn::Error::new(
            input.path.span(),
            format!("Lex error in template file '{}': {}", path.display(), e),
        )
    })?;

    let rsx_node = crate::parser::parse_rsx(tokens)?;
    let generated = crate::codegen::generate(rsx_node);

    Ok(quote! {
        {
            // File dependency tracking for live reload & cargo cache invalidation
            const _: &[u8] = include_bytes!(#abs_str);
            #generated
        }
    })
}

/// Implementation of `yav_component!`.
pub fn yav_component_impl(input: YavComponentInput) -> Result<TokenStream> {
    let path = resolve_path(&input.path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| {
        syn::Error::new(
            input.path.span(),
            format!("Failed to read component file '{}': {}", path.display(), e),
        )
    })?;

    let abs_str = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .to_string_lossy()
        .to_string();

    let struct_ident = match input.name {
        Some(ident) => ident,
        None => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Component");
            format_ident!("{}", stem)
        }
    };

    let (frontmatter, template) = split_frontmatter(&content);

    // Frontmatter parsing & transpilation
    let (preamble, props_type) = if let Some(fm) = frontmatter {
        let transpiled_fm = transpile_frontmatter(fm);
        let fm_tokens: TokenStream =
            transpiled_fm.parse().map_err(|e: proc_macro2::LexError| {
                syn::Error::new(
                    input.path.span(),
                    format!(
                        "Syntax error in frontmatter of '{}': {}\nTranspiled code:\n{}",
                        path.display(),
                        e,
                        transpiled_fm
                    ),
                )
            })?;

        let has_props = transpiled_fm.contains("struct Props");
        if has_props {
            (fm_tokens, quote! { Props })
        } else {
            (
                quote! {
                    #fm_tokens

                    #[derive(Clone, Debug, Default, PartialEq)]
                    pub struct Props;
                },
                quote! { Props },
            )
        }
    } else {
        (
            quote! {
                #[derive(Clone, Debug, Default, PartialEq)]
                pub struct Props;
            },
            quote! { Props },
        )
    };

    // Template parsing
    let template_tokens: TokenStream = template.parse().map_err(|e: proc_macro2::LexError| {
        syn::Error::new(
            input.path.span(),
            format!(
                "Syntax error in template markup of '{}': {}",
                path.display(),
                e
            ),
        )
    })?;

    let rsx_node = crate::parser::parse_rsx(template_tokens)?;
    let rendered = crate::codegen::generate(rsx_node);

    let props_ident = quote::format_ident!("{}Props", struct_ident);

    Ok(quote! {
        #preamble

        pub type #props_ident = #props_type;

        #[derive(Clone, Debug, Default)]
        pub struct #struct_ident {
            pub props: #props_type,
        }

        #[allow(non_snake_case)]
        pub fn #struct_ident(props: #props_ident) -> ::yav::yav_core::Node {
            #struct_ident { props }.render()
        }

        impl #struct_ident {
            pub fn new(props: impl Into<#props_type>) -> Self {
                Self { props: props.into() }
            }

            pub fn render(&self) -> ::yav::yav_core::Node {
                const _: &[u8] = include_bytes!(#abs_str);
                #[allow(unused_variables)]
                let props = &self.props;
                #rendered
            }
        }

        impl ::yav::yav_core::Component for #struct_ident {
            fn render(&self) -> ::yav::yav_core::Node {
                self.render()
            }
        }

        impl ::yav::yav_core::IntoNode for #struct_ident {
            fn into_node(self) -> ::yav::yav_core::Node {
                self.render()
            }
        }

        impl ::yav::yav_core::IntoNode for &#struct_ident {
            fn into_node(self) -> ::yav::yav_core::Node {
                self.render()
            }
        }
    })
}

/// Transpiles frontmatter, allowing web-style `import { ... } from "..."`
/// and `props { ... }` blocks to be automatically converted to standard Rust.
pub fn transpile_frontmatter(fm: &str) -> String {
    let mut out = String::new();
    let mut in_props_block = false;
    let mut props_fields: Vec<(String, String, Option<String>)> = Vec::new();
    let mut import_buf: Option<String> = None;

    for line in fm.lines() {
        let trimmed = line.trim();

        // Handle multi-line import accumulation
        if let Some(ref mut buf) = import_buf {
            buf.push(' ');
            buf.push_str(trimmed);
            if trimmed.contains("from")
                && (trimmed.ends_with(';') || trimmed.ends_with('"') || trimmed.ends_with('\''))
            {
                let complete_import = import_buf.take().unwrap();
                if let Some(rust_use) = convert_import_statement(&complete_import) {
                    out.push_str(&rust_use);
                    out.push('\n');
                } else {
                    out.push_str(&complete_import);
                    out.push('\n');
                }
            }
            continue;
        }

        // Check if entering props { ... } block
        if trimmed.starts_with("props") && trimmed.contains('{') {
            in_props_block = true;
            continue;
        }

        if in_props_block {
            if trimmed.starts_with('}') {
                in_props_block = false;
                // Generate the Props struct and Default impl
                out.push_str("\n#[derive(Clone, Debug, PartialEq)]\npub struct Props {\n");
                for (name, ty, _) in &props_fields {
                    out.push_str(&format!("    pub {}: {},\n", name, ty));
                }
                out.push_str(
                    "}\n\nimpl Default for Props {\n    fn default() -> Self {\n        Self {\n",
                );
                for (name, ty, default_val) in &props_fields {
                    match default_val {
                        Some(val) => {
                            let v = val.trim();
                            if (ty.trim() == "String" || ty.trim().ends_with("::String"))
                                && v.starts_with('"')
                                && v.ends_with('"')
                            {
                                out.push_str(&format!(
                                    "            {}: {}.to_string(),\n",
                                    name, v
                                ));
                            } else {
                                out.push_str(&format!("            {}: {},\n", name, v));
                            }
                        }
                        None => out.push_str(&format!(
                            "            {}: ::std::default::Default::default(),\n",
                            name
                        )),
                    }
                }
                out.push_str("        }\n    }\n}\n");
                continue;
            }

            // Parse field in props block: `username: String` or `is_admin: bool = false`
            if let Some((field_def, default_val)) = trimmed.split_once('=') {
                if let Some((name, ty)) = field_def.split_once(':') {
                    let name = name.trim().trim_end_matches(',');
                    let ty = ty.trim().trim_end_matches(',');
                    let default_val = default_val.trim().trim_end_matches(',');
                    props_fields.push((
                        name.to_string(),
                        ty.to_string(),
                        Some(default_val.to_string()),
                    ));
                }
            } else if let Some((name, ty)) = trimmed.split_once(':') {
                let name = name.trim().trim_end_matches(',');
                let ty = ty.trim().trim_end_matches(',');
                props_fields.push((name.to_string(), ty.to_string(), None));
            }
            continue;
        }

        // Check for web style: import { A, B } from "path"
        if trimmed.starts_with("import") {
            if trimmed.contains("from") {
                if let Some(rust_use) = convert_import_statement(trimmed) {
                    out.push_str(&rust_use);
                    out.push('\n');
                    continue;
                }
            } else {
                import_buf = Some(trimmed.to_string());
                continue;
            }
        }

        // Standard Rust line passed through as-is
        out.push_str(line);
        out.push('\n');
    }

    if let Some(dangling) = import_buf {
        out.push_str(&dangling);
        out.push('\n');
    }

    out
}

/// Converts a web-style ESM import statement to Rust `use` syntax.
///
/// Examples:
/// - `import { Button, Card } from "yav/ui"` -> `use ::yav::ui::{Button, Card};`
/// - `import { render_icon } from "yav/icons"` -> `use ::yav::icons::{render_icon};`
/// - `import { Node, Component } from "yav"` -> `use ::yav::prelude::{Node, Component};`
fn convert_import_statement(line: &str) -> Option<String> {
    let after_import = line.strip_prefix("import")?.trim();
    let (symbols_part, source_part) = after_import.split_once("from")?;
    let symbols = symbols_part.trim();
    let source = source_part
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == ';');

    let rust_source = match source {
        "yav" => "::yav::prelude",
        "yav/ui" => "::yav::ui",
        "yav/icons" => "::yav::icons",
        "yav/server" => "::yav::server",
        "yav/router" => "::yav::router",
        s if s.ends_with(".yav") || s.ends_with(".rsx") || s.ends_with(".rs") => {
            let path = std::path::Path::new(s);
            let stem = path.file_stem()?.to_str()?;
            return Some(format!("use super::{stem}::{symbols};"));
        }
        other => other,
    };

    Some(format!("use {rust_source}::{symbols};"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = "---\nuse yav::prelude::*;\n---\n<div>\"Hello\"</div>";
        let (fm, tpl) = split_frontmatter(content);
        assert_eq!(fm, Some("use yav::prelude::*;"));
        assert_eq!(tpl, "<div>\"Hello\"</div>");
    }

    #[test]
    fn test_transpile_frontmatter_esm_imports() {
        let fm = r#"
import { Button, Card } from "yav/ui"
import { Icon } from "yav/icons"

use std::collections::HashMap;
"#;
        let transpiled = transpile_frontmatter(fm);
        assert!(transpiled.contains("use ::yav::ui::{ Button, Card };"));
        assert!(transpiled.contains("use ::yav::icons::{ Icon };"));
        assert!(transpiled.contains("use std::collections::HashMap;"));
    }

    #[test]
    fn test_transpile_frontmatter_props_block() {
        let fm = r#"
props {
    title: String,
    count: u32 = 0,
    is_active: bool = false
}
"#;
        let transpiled = transpile_frontmatter(fm);
        assert!(transpiled.contains("pub struct Props {"));
        assert!(transpiled.contains("pub title: String,"));
        assert!(transpiled.contains("pub count: u32,"));
        assert!(transpiled.contains("pub is_active: bool,"));
        assert!(transpiled.contains("impl Default for Props {"));
    }

    #[test]
    fn test_transpile_frontmatter_multiline_imports() {
        let fm = r#"
import {
    Button,
    Card,
} from "yav/ui";
"#;
        let transpiled = transpile_frontmatter(fm);
        assert!(transpiled.contains("use ::yav::ui::{ Button, Card, };"));
    }
}
