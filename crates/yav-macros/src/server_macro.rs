//! `#[server]` procedural attribute macro for type-safe Server Functions / Server Actions.
//!
//! Inspired by Next.js `"use server"` and Leptos `#[server]`, this macro transforms an async
//! Rust function into both a directly callable server function and a mountable HTTP POST
//! endpoint with automatic [`FromFormData`] extraction and validation.
//!
//! # Example (illustrative, not compiled here)
//!
//! See the crate-level note in `lib.rs`: this crate's doctests cannot resolve
//! the `yav` facade. Compiled `#[server]` examples live in the `yav` docs.
//! ```rust,ignore
//! use yav::prelude::*;
//!
//! #[derive(FromFormData)]
//! pub struct CreateTaskForm {
//!     #[form(required = "Title cannot be empty")]
//!     pub title: String,
//! }
//!
//! #[server("/_actions/create-task")]
//! pub async fn create_task(input: CreateTaskForm) -> HtmlFragment {
//!     HtmlFragment::new(rsx! { <li>{ input.title }</li> })
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, LitStr, PatType, Result, Type};

fn last_type_ident(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}

pub fn server_macro_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let func: ItemFn = syn::parse2(item)?;
    let fn_ident = &func.sig.ident;
    let vis = &func.vis;

    let endpoint_path = if attr.is_empty() {
        format!("/_server/{fn_ident}")
    } else {
        let lit: LitStr = syn::parse2(attr)?;
        let val = lit.value();
        if val.starts_with('/') {
            val
        } else {
            format!("/{val}")
        }
    };

    let url_fn_ident = format_ident!("{}_url", fn_ident);
    let handler_fn_ident = format_ident!("{}_handler", fn_ident);
    let register_fn_ident = format_ident!("register_{}", fn_ident);

    let typed_inputs: Vec<&PatType> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(pt) => Some(pt),
            FnArg::Receiver(_) => None,
        })
        .collect();

    let handler_body = match typed_inputs.len() {
        0 => {
            quote! {
                ::yav::server::BoxedHandler::from_zero_arg(#fn_ident)
            }
        }
        1 => {
            let param_ty = &typed_inputs[0].ty;
            let ty_name = last_type_ident(param_ty);
            if ty_name == "Request" {
                quote! {
                    ::yav::server::BoxedHandler::new(#fn_ident)
                }
            } else if ty_name == "RouteContext" {
                quote! {
                    ::yav::server::BoxedHandler::from_ctx(#fn_ident)
                }
            } else if ty_name == "FormData" {
                quote! {
                    ::yav::server::BoxedHandler::from_extractor1(#fn_ident)
                }
            } else {
                quote! {
                    ::yav::server::BoxedHandler::from_extractor1(|__form: ::yav::server::FormData| async move {
                        match <#param_ty as ::yav::server::FromFormData>::from_form(&__form) {
                            ::std::result::Result::Ok(__parsed) => {
                                ::yav::server::IntoResponse::into_response(#fn_ident(__parsed).await)
                            }
                            ::std::result::Result::Err(__errs) => {
                                ::yav::server::IntoResponse::into_response(
                                    ::yav::server::ActionResponse::errors(__errs),
                                )
                            }
                        }
                    })
                }
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                &func.sig.inputs,
                "#[server] functions support 0 arguments or 1 typed argument (e.g. a struct deriving FromFormData, FormData, RouteContext, or Request)",
            ));
        }
    };

    Ok(quote! {
        #func

        /// Returns the HTTP endpoint URL path for the [`#fn_ident`] server function.
        #vis const fn #url_fn_ident() -> &'static str {
            #endpoint_path
        }

        /// Returns a boxed Axum/Yav handler for mounting the [`#fn_ident`] server function.
        #vis fn #handler_fn_ident() -> ::yav::server::BoxedHandler {
            #handler_body
        }

        /// Registers the [`#fn_ident`] server function as a `POST` route on a [`YavApp`](::yav::server::YavApp).
        #vis fn #register_fn_ident(app: ::yav::server::YavApp) -> ::yav::server::YavApp {
            app.post(#endpoint_path, #handler_fn_ident())
        }
    })
}
