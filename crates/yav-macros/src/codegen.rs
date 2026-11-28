//! Code generation — transforms the RSX AST into Rust code that builds
//! `yav_core::Node` trees at runtime.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parser::{RsxAttrValue, RsxAttribute, RsxElement, RsxNode};

/// Resolves the canonical path to `yav_core` depending on whether the caller
/// is `yav-core` itself, a crate depending on `yav-core` directly, or the `yav` facade crate.
pub(crate) fn yav_core_root() -> TokenStream {
    if let Ok(pkg) = std::env::var("CARGO_PKG_NAME") {
        if pkg == "yav-core" || pkg == "yav_core" {
            return quote! { crate };
        }
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(manifest_path) {
            let has_yav_facade = content.lines().any(|l| {
                let t = l.trim();
                (t.starts_with("yav ") || t.starts_with("yav="))
                    && !t.starts_with("yav-")
                    && !t.starts_with("yav_")
            });
            let has_yav_core = content.contains("yav-core") || content.contains("yav_core");
            if has_yav_core && !has_yav_facade {
                return quote! { ::yav_core };
            }
        }
    }
    quote! { ::yav::yav_core }
}

/// Generate Rust code from an RSX AST node.
pub fn generate(node: RsxNode) -> TokenStream {
    gen_node(&node, true)
}

fn gen_node(node: &RsxNode, is_root: bool) -> TokenStream {
    let core = yav_core_root();
    if !is_root {
        if let Some(static_html) = try_fold_static_node(node) {
            return quote! {
                #core::Node::Static(#static_html)
            };
        }
    }

    match node {
        RsxNode::Element(el) => gen_element(el),
        RsxNode::Text(text, _span) => {
            let escaped = escape_html_compile_time(text);
            quote! {
                #core::Node::Static(#escaped)
            }
        }
        RsxNode::Expr(expr, _span) => {
            quote! {
                #core::IntoNode::into_node(#expr)
            }
        }
        RsxNode::Fragment(nodes) => {
            let children: Vec<_> = nodes.iter().map(|n| gen_node(n, false)).collect();
            quote! {
                #core::Node::Fragment(vec![#(#children),*])
            }
        }
        RsxNode::Raw(raw, _span) => {
            quote! {
                #core::Node::Static(#raw)
            }
        }
        RsxNode::RawExpr(expr, _span) => {
            quote! {
                #core::Node::raw((#expr).to_string())
            }
        }
        RsxNode::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let then_code = gen_node(then_branch, false);
            if let Some(else_node) = else_branch {
                let else_code = gen_node(else_node, false);
                quote! {
                    if #cond {
                        #then_code
                    } else {
                        #else_code
                    }
                }
            } else {
                quote! {
                    if #cond {
                        #then_code
                    } else {
                        #core::Node::Empty
                    }
                }
            }
        }
        RsxNode::For { header, body } => {
            let body_code = gen_node(body, false);
            quote! {
                #core::Node::Fragment({
                    let mut __yav_items = ::std::vec::Vec::new();
                    for #header {
                        __yav_items.push(#body_code);
                    }
                    __yav_items
                })
            }
        }
    }
}

fn gen_element(el: &RsxElement) -> TokenStream {
    let core = yav_core_root();
    if el.is_component {
        return gen_component(el);
    }

    let tag = &el.tag;

    // Check if there are any `class:name` conditional class directives
    let has_class_directives = el
        .attributes
        .iter()
        .any(|a| a.name.starts_with("class:") && a.name.len() > 6);

    let mut attrs: Vec<TokenStream> = if has_class_directives {
        gen_attributes_with_class_directives(&el.attributes)
    } else {
        el.attributes.iter().map(gen_attribute).collect()
    };

    // If `yav:initial` is present and no `style` attribute exists, inject anti-FOUC initial inline style
    let has_style_attr = el.attributes.iter().any(|a| a.name == "style");
    if !has_style_attr {
        if let Some(init_attr) = el.attributes.iter().find(|a| a.name == "yav:initial") {
            if let RsxAttrValue::Str(ref s, _) = init_attr.value {
                if let Some(style_str) = compile_yav_initial_style(s) {
                    attrs.push(quote! {
                        #core::Attribute {
                            name: ::std::borrow::Cow::Borrowed("style"),
                            value: #core::AttributeValue::Static(#style_str),
                        }
                    });
                }
            }
        }
    }

    // Generate children (allowing static subtree folding for children unless parent is <Head> template)
    let is_head_template =
        tag == "template" && el.attributes.iter().any(|a| a.name == "data-yav-head");
    let children: Vec<TokenStream> = el
        .children
        .iter()
        .map(|c| gen_node(c, is_head_template))
        .collect();

    let self_closing = el.self_closing || is_void_element(tag);

    quote! {
        #core::Node::Element(#core::Element {
            tag: ::std::borrow::Cow::Borrowed(#tag),
            attributes: vec![#(#attrs),*],
            children: vec![#(#children),*],
            self_closing: #self_closing,
        })
    }
}

fn gen_attributes_with_class_directives(attributes: &[RsxAttribute]) -> Vec<TokenStream> {
    let core = yav_core_root();
    let mut out = Vec::new();
    let mut class_push_stmts = Vec::new();

    for attr in attributes {
        if attr.name == "class" {
            match &attr.value {
                RsxAttrValue::Str(s, _) => {
                    if !s.is_empty() {
                        class_push_stmts.push(quote! {
                            if !__yav_classes.is_empty() {
                                __yav_classes.push(' ');
                            }
                            __yav_classes.push_str(#s);
                        });
                    }
                }
                RsxAttrValue::Expr(expr, _) => {
                    class_push_stmts.push(quote! {
                        match #core::IntoAttributeValue::into_attribute_value(#expr) {
                            #core::AttributeValue::Static(s) => {
                                if !s.is_empty() {
                                    if !__yav_classes.is_empty() {
                                        __yav_classes.push(' ');
                                    }
                                    __yav_classes.push_str(s);
                                }
                            }
                            #core::AttributeValue::String(s) => {
                                if !s.is_empty() {
                                    if !__yav_classes.is_empty() {
                                        __yav_classes.push(' ');
                                    }
                                    __yav_classes.push_str(&s);
                                }
                            }
                            _ => {}
                        }
                    });
                }
                RsxAttrValue::Bool(_) => {}
            }
        } else if let Some(cls_name) = attr.name.strip_prefix("class:") {
            if !cls_name.is_empty() {
                match &attr.value {
                    RsxAttrValue::Expr(expr, _) => {
                        class_push_stmts.push(quote! {
                            if #expr {
                                if !__yav_classes.is_empty() {
                                    __yav_classes.push(' ');
                                }
                                __yav_classes.push_str(#cls_name);
                            }
                        });
                    }
                    RsxAttrValue::Bool(_) => {
                        class_push_stmts.push(quote! {
                            if !__yav_classes.is_empty() {
                                __yav_classes.push(' ');
                            }
                            __yav_classes.push_str(#cls_name);
                        });
                    }
                    RsxAttrValue::Str(s, _) => {
                        if !s.is_empty() && s != "false" && s != "0" {
                            class_push_stmts.push(quote! {
                                if !__yav_classes.is_empty() {
                                    __yav_classes.push(' ');
                                }
                                __yav_classes.push_str(#cls_name);
                            });
                        }
                    }
                }
            }
        } else {
            out.push(gen_attribute(attr));
        }
    }

    out.insert(
        0,
        quote! {
            #core::Attribute {
                name: ::std::borrow::Cow::Borrowed("class"),
                value: #core::AttributeValue::String({
                    let mut __yav_classes = ::std::string::String::new();
                    #(#class_push_stmts)*
                    __yav_classes
                }),
            }
        },
    );

    out
}

fn gen_component(el: &RsxElement) -> TokenStream {
    let core = yav_core_root();
    let tag = &el.tag;
    let tag_path: syn::Path = match syn::parse_str(tag) {
        Ok(path) => path,
        Err(_) => {
            let ident = syn::Ident::new(tag, el.tag_span);
            syn::parse_quote!(#ident)
        }
    };

    let props_path = get_props_path(&tag_path);

    let prop_fields: Vec<TokenStream> = el
        .attributes
        .iter()
        .map(|attr| {
            let field_ident = match syn::parse_str::<syn::Ident>(&attr.name) {
                Ok(mut id) => {
                    id.set_span(attr.name_span);
                    id
                }
                Err(_) => syn::Ident::new_raw(&attr.name, attr.name_span),
            };
            match &attr.value {
                RsxAttrValue::Str(s, _) => {
                    quote! { #field_ident: #core::IntoPropValue::into_prop_value(#s) }
                }
                RsxAttrValue::Expr(expr, _) => quote! { #field_ident: #expr },
                RsxAttrValue::Bool(_) => quote! { #field_ident: true },
            }
        })
        .collect();

    let is_head_component = tag == "Head" || tag.ends_with("::Head");
    let children_field = if !el.children.is_empty() {
        let child_tokens: Vec<TokenStream> = el
            .children
            .iter()
            .map(|c| gen_node(c, is_head_component))
            .collect();
        quote! {
            children: vec![#(#child_tokens),*].into(),
        }
    } else {
        quote! {}
    };

    quote! {
        #core::IntoNode::into_node(
            #tag_path({
                #[allow(clippy::needless_update)]
                #props_path {
                    #(#prop_fields,)*
                    #children_field
                    ..Default::default()
                }
            })
        )
    }
}

fn get_props_path(tag_path: &syn::Path) -> syn::Path {
    let mut props_path = tag_path.clone();
    if let Some(last) = props_path.segments.last_mut() {
        let name = format!("{}Props", last.ident);
        last.ident = syn::Ident::new(&name, last.ident.span());
    }
    props_path
}

fn gen_attribute(attr: &RsxAttribute) -> TokenStream {
    let core = yav_core_root();
    let name = &attr.name;
    match &attr.value {
        RsxAttrValue::Str(val, _) => {
            quote! {
                #core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: #core::AttributeValue::Static(#val),
                }
            }
        }
        RsxAttrValue::Expr(expr, _) => {
            quote! {
                #core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: #core::IntoAttributeValue::into_attribute_value(#expr),
                }
            }
        }
        RsxAttrValue::Bool(_) => {
            quote! {
                #core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: #core::AttributeValue::Bool(true),
                }
            }
        }
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_url_attr(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "href"
            | "src"
            | "action"
            | "formaction"
            | "poster"
            | "data"
            | "cite"
            | "xlink:href"
            | "ping"
            | "background"
    )
}

fn is_safe_static_url(val: &str) -> bool {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return false;
    }
    let normalized: String = trimmed
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if normalized.starts_with("javascript:")
        || normalized.starts_with("vbscript:")
        || normalized.starts_with("data:")
    {
        return false;
    }
    true
}

fn escape_html_compile_time(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

fn compile_yav_initial_style(shorthand: &str) -> Option<String> {
    let mut opacity = None;
    let mut x = None;
    let mut y = None;
    let mut scale = None;
    let mut rotate = None;

    for pair in shorthand.split(',') {
        let mut kv = pair.split(':');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            let key = k.trim();
            if let Ok(val) = v.trim().parse::<f64>() {
                match key {
                    "opacity" => opacity = Some(val),
                    "x" => x = Some(val),
                    "y" => y = Some(val),
                    "scale" => scale = Some(val),
                    "rotate" => rotate = Some(val),
                    _ => {}
                }
            }
        }
    }

    let mut parts = Vec::new();
    if let Some(op) = opacity {
        parts.push(format!("opacity: {op};"));
    }
    let mut transforms = Vec::new();
    if x.is_some() || y.is_some() {
        transforms.push(format!(
            "translate3d({}px, {}px, 0)",
            x.unwrap_or(0.0),
            y.unwrap_or(0.0)
        ));
    }
    if let Some(sc) = scale {
        transforms.push(format!("scale({sc})"));
    }
    if let Some(rot) = rotate {
        transforms.push(format!("rotate({rot}deg)"));
    }
    if !transforms.is_empty() {
        parts.push(format!("transform: {};", transforms.join(" ")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn try_fold_static_node(node: &RsxNode) -> Option<String> {
    match node {
        RsxNode::Text(text, _) => Some(escape_html_compile_time(text)),
        RsxNode::Raw(raw, _) => Some(raw.clone()),
        RsxNode::Fragment(children) => {
            let mut out = String::new();
            for child in children {
                out.push_str(&try_fold_static_node(child)?);
            }
            Some(out)
        }
        RsxNode::Element(el) => {
            if el.is_component {
                return None;
            }
            if matches!(
                el.tag.to_ascii_lowercase().as_str(),
                "template" | "title" | "meta" | "link" | "script" | "style"
            ) {
                return None;
            }
            if !is_valid_static_tag(&el.tag) {
                return None;
            }
            if el.attributes.iter().any(|a| {
                a.name.starts_with("class:")
                    || a.name == "yav:initial"
                    || a.name.eq_ignore_ascii_case("style")
                    || a.name.eq_ignore_ascii_case("srcdoc")
                    || a.name.eq_ignore_ascii_case("srcset")
                    || !is_valid_static_attr(&a.name)
            }) {
                return None;
            }

            let mut out = String::new();
            out.push('<');
            out.push_str(&el.tag);

            for attr in &el.attributes {
                match &attr.value {
                    RsxAttrValue::Str(val, _) => {
                        if is_url_attr(&attr.name) && !is_safe_static_url(val) {
                            return None;
                        }
                        out.push(' ');
                        out.push_str(&attr.name);
                        out.push_str("=\"");
                        out.push_str(&escape_html_compile_time(val));
                        out.push('"');
                    }
                    RsxAttrValue::Bool(_) => {
                        out.push(' ');
                        out.push_str(&attr.name);
                    }
                    RsxAttrValue::Expr(_, _) => return None,
                }
            }

            if is_void_element(&el.tag) {
                out.push_str(" />");
                return Some(out);
            }

            out.push('>');
            for child in &el.children {
                out.push_str(&try_fold_static_node(child)?);
            }
            out.push_str("</");
            out.push_str(&el.tag);
            out.push('>');
            Some(out)
        }
        RsxNode::Expr(_, _) | RsxNode::RawExpr(_, _) | RsxNode::If { .. } | RsxNode::For { .. } => {
            None
        }
    }
}

fn is_valid_static_tag(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
}

fn is_valid_static_attr(name: &str) -> bool {
    !name.is_empty()
        && !name.chars().any(|c| {
            c.is_ascii_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '>' | '/' | '=')
        })
}
