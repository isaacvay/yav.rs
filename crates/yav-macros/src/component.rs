//! Implementation of the `#[component]` attribute macro.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse2, FnArg, ItemFn, Pat, PatType, Result};

pub fn component_impl(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let input_fn = parse2::<ItemFn>(item)?;

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let fn_name = &sig.ident;
    let generics = &sig.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let block = &input_fn.block;
    let props_ident = format_ident!("{}Props", fn_name);

    // If there is only 1 argument named `props` (or `_props`):
    // The user already defined their props struct manually.
    if sig.inputs.len() == 1 {
        if let Some(FnArg::Typed(PatType { pat, .. })) = sig.inputs.first() {
            if let Pat::Ident(pat_ident) = pat.as_ref() {
                if pat_ident.ident == "props" || pat_ident.ident == "_props" {
                    return Ok(quote! {
                        #[allow(non_snake_case)]
                        #input_fn
                    });
                }
            }
        }
    }

    let core = crate::codegen::yav_core_root();

    // If there are 0 arguments:
    if sig.inputs.is_empty() {
        return Ok(quote! {
            #[derive(Clone, Debug, Default)]
            #vis struct #props_ident {}

            #(#attrs)*
            #[allow(non_snake_case)]
            #vis fn #fn_name(_props: #props_ident) -> #core::Node {
                #block
            }
        });
    }

    // Multiple arguments or 1 argument that is not `props`:
    // E.g.: `fn UserBadge(name: String, is_admin: bool) -> Node`
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();

    for arg in &sig.inputs {
        match arg {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(
                    r.span(),
                    "#[component] cannot be used on methods with a `self` receiver",
                ));
            }
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(pat_ident) = pat.as_ref() {
                    field_names.push(&pat_ident.ident);
                    field_types.push(ty.as_ref());
                } else {
                    return Err(syn::Error::new(
                        pat.span(),
                        "#[component] arguments must be simple identifiers",
                    ));
                }
            }
        }
    }

    let expanded = quote! {
        #[derive(Default)]
        #vis struct #props_ident #generics #where_clause {
            #(pub #field_names: #field_types,)*
        }

        #(#attrs)*
        #[allow(non_snake_case)]
        #vis fn #fn_name #impl_generics (props: #props_ident #ty_generics) -> #core::Node #where_clause {
            let #props_ident { #(#field_names),* } = props;
            #block
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn error_message(item: TokenStream) -> String {
        component_impl(TokenStream::new(), item)
            .expect_err("expected #[component] input to be rejected")
            .to_string()
    }

    #[test]
    fn zero_arg_component_gets_generated_props() {
        let expanded = component_impl(
            TokenStream::new(),
            quote! { fn Hello() -> Node { Node::empty() } },
        )
        .expect("zero-argument component should expand");
        let text = expanded.to_string();
        assert!(text.contains("HelloProps"), "{text}");
        assert!(text.contains("_props"), "{text}");
    }

    #[test]
    fn multiple_simple_args_become_props_fields() {
        let expanded = component_impl(
            TokenStream::new(),
            quote! { fn Badge(name: String, admin: bool) -> Node { Node::empty() } },
        )
        .expect("simple arguments should expand");
        let text = expanded.to_string();
        assert!(text.contains("pub name"), "{text}");
        assert!(text.contains("pub admin"), "{text}");
        assert!(text.contains("BadgeProps"), "{text}");
    }

    #[test]
    fn explicit_props_argument_is_left_untouched() {
        let expanded = component_impl(
            TokenStream::new(),
            quote! { fn Card(props: CardProps) -> Node { props.render() } },
        )
        .expect("explicit props struct should be preserved");
        assert!(!expanded.to_string().contains("CardPropsProps"));
    }

    #[test]
    fn method_with_self_receiver_is_rejected() {
        let msg = error_message(quote! { fn Bad(&self) -> Node { Node::empty() } });
        assert!(msg.contains("self"), "{msg}");
    }

    #[test]
    fn destructured_argument_is_rejected() {
        let msg =
            error_message(quote! { fn Bad((a, b): (String, String)) -> Node { Node::empty() } });
        assert!(msg.contains("simple identifiers"), "{msg}");
    }

    #[test]
    fn non_function_item_is_rejected() {
        let msg = error_message(quote! { struct Nope; });
        assert!(!msg.is_empty());
    }
}
