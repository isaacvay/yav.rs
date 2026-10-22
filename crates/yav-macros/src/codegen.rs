//! Code generation — transforms the RSX AST into Rust code that builds
//! `yav_core::Node` trees at runtime.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parser::{RsxAttrValue, RsxAttribute, RsxElement, RsxNode};

/// Generate Rust code from an RSX AST node.
pub fn generate(node: RsxNode) -> TokenStream {
    gen_node(&node)
}

fn gen_node(node: &RsxNode) -> TokenStream {
    match node {
        RsxNode::Element(el) => gen_element(el),
        RsxNode::Text(text, _span) => {
            quote! {
                ::yav::yav_core::Node::Text(#text.to_string())
            }
        }
        RsxNode::Expr(expr, _span) => {
            quote! {
                ::yav::yav_core::IntoNode::into_node(#expr)
            }
        }
        RsxNode::Fragment(nodes) => {
            let children: Vec<_> = nodes.iter().map(gen_node).collect();
            quote! {
                ::yav::yav_core::Node::Fragment(vec![#(#children),*])
            }
        }
    }
}

fn gen_element(el: &RsxElement) -> TokenStream {
    if el.is_component {
        return gen_component(el);
    }

    let tag = &el.tag;

    // Generate attributes
    let attrs: Vec<TokenStream> = el.attributes.iter().map(gen_attribute).collect();

    // Generate children
    let children: Vec<TokenStream> = el.children.iter().map(gen_node).collect();

    let self_closing = el.self_closing || is_void_element(tag);

    quote! {
        ::yav::yav_core::Node::Element(::yav::yav_core::Element {
            tag: ::std::borrow::Cow::Borrowed(#tag),
            attributes: vec![#(#attrs),*],
            children: vec![#(#children),*],
            self_closing: #self_closing,
        })
    }
}

fn gen_component(el: &RsxElement) -> TokenStream {
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
                    quote! { #field_ident: ::yav::yav_core::IntoPropValue::into_prop_value(#s) }
                }
                RsxAttrValue::Expr(expr, _) => quote! { #field_ident: #expr },
                RsxAttrValue::Bool(_) => quote! { #field_ident: true },
            }
        })
        .collect();

    let children_field = if !el.children.is_empty() {
        let child_tokens: Vec<TokenStream> = el.children.iter().map(gen_node).collect();
        quote! {
            children: vec![#(#child_tokens),*].into(),
        }
    } else {
        quote! {}
    };

    quote! {
        ::yav::yav_core::IntoNode::into_node(
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
    let name = &attr.name;
    match &attr.value {
        RsxAttrValue::Str(val, _) => {
            quote! {
                ::yav::yav_core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: ::yav::yav_core::AttributeValue::Static(#val),
                }
            }
        }
        RsxAttrValue::Expr(expr, _) => {
            quote! {
                ::yav::yav_core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: ::yav::yav_core::IntoAttributeValue::into_attribute_value(#expr),
                }
            }
        }
        RsxAttrValue::Bool(_) => {
            quote! {
                ::yav::yav_core::Attribute {
                    name: ::std::borrow::Cow::Borrowed(#name),
                    value: ::yav::yav_core::AttributeValue::Bool(true),
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
