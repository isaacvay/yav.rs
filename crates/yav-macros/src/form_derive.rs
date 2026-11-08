//! `#[derive(FromFormData)]` procedural macro for automatic typed form extraction & validation.
//!
//! Eliminates manual `FromFormData` boilerplate while supporting declarative `#[form(...)]` rules:
//! - `#[form(required = "Custom required message")]` or `#[form(required)]`
//! - `#[form(email = "Invalid email format")]` or `#[form(email)]`
//! - `#[form(min_length = 8, message = "Must be at least 8 chars")]`
//! - `#[form(max_length = 120, message = "Too long")]`
//! - `#[form(min = 1, max = 100, message = "Value out of range")]`
//! - `#[form(matches = "password", message = "Passwords do not match")]`

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Meta, Result, Type};

#[derive(Default)]
struct FieldFormRules {
    required: Option<String>,
    email: Option<String>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    min_val: Option<f64>,
    max_val: Option<f64>,
    matches_field: Option<String>,
    custom_message: Option<String>,
}

fn parse_field_rules(attrs: &[syn::Attribute], field_name: &str) -> Result<FieldFormRules> {
    let mut rules = FieldFormRules::default();

    for attr in attrs {
        if !attr.path().is_ident("form") {
            continue;
        }
        let nested = attr.parse_args_with(
            syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
        )?;
        for meta in nested {
            match meta {
                Meta::Path(path) => {
                    if path.is_ident("required") {
                        rules.required = Some(format!("{field_name} is required"));
                    } else if path.is_ident("email") {
                        rules.email = Some(format!("{field_name} must be a valid email address"));
                    }
                }
                Meta::NameValue(nv) => {
                    let ident = nv
                        .path
                        .get_ident()
                        .map(|i| i.to_string())
                        .unwrap_or_default();
                    match (&ident[..], &nv.value) {
                        ("required", Expr::Lit(el)) => {
                            if let Lit::Str(s) = &el.lit {
                                rules.required = Some(s.value());
                            } else if let Lit::Bool(b) = &el.lit {
                                if b.value {
                                    rules.required = Some(format!("{field_name} is required"));
                                }
                            }
                        }
                        ("email", Expr::Lit(el)) => {
                            if let Lit::Str(s) = &el.lit {
                                rules.email = Some(s.value());
                            } else if let Lit::Bool(b) = &el.lit {
                                if b.value {
                                    rules.email =
                                        Some(format!("{field_name} must be a valid email address"));
                                }
                            }
                        }
                        ("min_length", Expr::Lit(el)) => {
                            if let Lit::Int(int_lit) = &el.lit {
                                rules.min_length = Some(int_lit.base10_parse::<usize>()?);
                            }
                        }
                        ("max_length", Expr::Lit(el)) => {
                            if let Lit::Int(int_lit) = &el.lit {
                                rules.max_length = Some(int_lit.base10_parse::<usize>()?);
                            }
                        }
                        ("min", Expr::Lit(el)) => match &el.lit {
                            Lit::Int(i) => rules.min_val = Some(i.base10_parse::<f64>()?),
                            Lit::Float(f) => rules.min_val = Some(f.base10_parse::<f64>()?),
                            _ => {}
                        },
                        ("max", Expr::Lit(el)) => match &el.lit {
                            Lit::Int(i) => rules.max_val = Some(i.base10_parse::<f64>()?),
                            Lit::Float(f) => rules.max_val = Some(f.base10_parse::<f64>()?),
                            _ => {}
                        },
                        ("matches", Expr::Lit(el)) => {
                            if let Lit::Str(s) = &el.lit {
                                rules.matches_field = Some(s.value());
                            }
                        }
                        ("message", Expr::Lit(el)) => {
                            if let Lit::Str(s) = &el.lit {
                                rules.custom_message = Some(s.value());
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(rules)
}

fn extract_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn type_ident_name(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}

pub fn derive_from_form_data_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(ds) => match &ds.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "FromFormData can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "FromFormData can only be derived for structs",
            ))
        }
    };

    let mut field_validations = Vec::new();
    let mut field_assignments = Vec::new();

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let rules = parse_field_rules(&field.attrs, &field_name)?;
        let ty = &field.ty;

        let mut checks = Vec::new();

        if let Some(ref req_msg) = rules.required {
            checks.push(quote! {
                __errors.require(#field_name, __raw_val, #req_msg);
            });
        }

        if let Some(ref email_msg) = rules.email {
            checks.push(quote! {
                if !__raw_val.trim().is_empty() {
                    __errors.email(#field_name, __raw_val, #email_msg);
                }
            });
        }

        if let Some(min_len) = rules.min_length {
            let msg = rules
                .custom_message
                .clone()
                .unwrap_or_else(|| format!("{field_name} must be at least {min_len} characters"));
            checks.push(quote! {
                if !__raw_val.trim().is_empty() {
                    __errors.min_length(#field_name, __raw_val, #min_len, #msg);
                }
            });
        }

        if let Some(max_len) = rules.max_length {
            let msg = rules
                .custom_message
                .clone()
                .unwrap_or_else(|| format!("{field_name} must be at most {max_len} characters"));
            checks.push(quote! {
                if __raw_val.trim().chars().count() > #max_len {
                    __errors.add(#field_name, #msg);
                }
            });
        }

        if let Some(ref other_field) = rules.matches_field {
            let msg = rules
                .custom_message
                .clone()
                .unwrap_or_else(|| format!("{field_name} must match {other_field}"));
            checks.push(quote! {
                let __other_val = form.get_or_empty(#other_field);
                if __raw_val != __other_val {
                    __errors.add(#field_name, #msg);
                }
            });
        }

        if let Some(inner_ty) = extract_option_inner(ty) {
            let inner_name = type_ident_name(inner_ty);
            let parse_expr = if inner_name == "String" {
                quote! {
                    if __raw_val.trim().is_empty() {
                        ::std::option::Option::None
                    } else {
                        ::std::option::Option::Some(__raw_val.to_string())
                    }
                }
            } else if inner_name == "bool" {
                quote! {
                    if __raw_val.trim().is_empty() {
                        ::std::option::Option::None
                    } else {
                        ::std::option::Option::Some(matches!(
                            __raw_val.trim().to_ascii_lowercase().as_str(),
                            "true" | "1" | "on" | "yes"
                        ))
                    }
                }
            } else {
                let parse_err_msg = format!("{field_name} must be a valid {inner_name}");
                let min_check = if let Some(min_v) = rules.min_val {
                    let min_msg = rules
                        .custom_message
                        .clone()
                        .unwrap_or_else(|| format!("{field_name} must be at least {min_v}"));
                    quote! {
                        if (v as f64) < #min_v {
                            __errors.add(#field_name, #min_msg);
                        }
                    }
                } else {
                    quote! {}
                };
                let max_check = if let Some(max_v) = rules.max_val {
                    let max_msg = rules
                        .custom_message
                        .clone()
                        .unwrap_or_else(|| format!("{field_name} must be at most {max_v}"));
                    quote! {
                        if (v as f64) > #max_v {
                            __errors.add(#field_name, #max_msg);
                        }
                    }
                } else {
                    quote! {}
                };
                quote! {
                    if __raw_val.trim().is_empty() {
                        ::std::option::Option::None
                    } else {
                        match __raw_val.trim().parse::<#inner_ty>() {
                            ::std::result::Result::Ok(v) => {
                                #min_check
                                #max_check
                                ::std::option::Option::Some(v)
                            }
                            ::std::result::Result::Err(_) => {
                                __errors.add(#field_name, #parse_err_msg);
                                ::std::option::Option::None
                            }
                        }
                    }
                }
            };

            field_validations.push(quote! {
                let __raw_val = form.get_or_empty(#field_name);
                #(#checks)*
                let #field_ident: #ty = #parse_expr;
            });
        } else {
            let ty_name = type_ident_name(ty);
            if ty_name == "String" {
                field_validations.push(quote! {
                    let __raw_val = form.get_or_empty(#field_name);
                    #(#checks)*
                    let #field_ident: ::std::string::String = __raw_val.to_string();
                });
            } else if ty_name == "bool" {
                field_validations.push(quote! {
                    let #field_ident: bool = form.get_bool(#field_name);
                });
            } else {
                // Numeric or FromStr type
                let parse_err_msg = rules
                    .custom_message
                    .clone()
                    .unwrap_or_else(|| format!("{field_name} must be a valid number"));
                let min_check = if let Some(min_v) = rules.min_val {
                    let min_msg = rules
                        .custom_message
                        .clone()
                        .unwrap_or_else(|| format!("{field_name} must be at least {min_v}"));
                    quote! {
                        if (__parsed as f64) < #min_v {
                            __errors.add(#field_name, #min_msg);
                        }
                    }
                } else {
                    quote! {}
                };
                let max_check = if let Some(max_v) = rules.max_val {
                    let max_msg = rules
                        .custom_message
                        .clone()
                        .unwrap_or_else(|| format!("{field_name} must be at most {max_v}"));
                    quote! {
                        if (__parsed as f64) > #max_v {
                            __errors.add(#field_name, #max_msg);
                        }
                    }
                } else {
                    quote! {}
                };

                field_validations.push(quote! {
                    let __raw_val = form.get_or_empty(#field_name);
                    #(#checks)*
                    let #field_ident: #ty = match __raw_val.trim().parse::<#ty>() {
                        ::std::result::Result::Ok(__parsed) => {
                            #min_check
                            #max_check
                            __parsed
                        }
                        ::std::result::Result::Err(_) => {
                            __errors.add(#field_name, #parse_err_msg);
                            ::std::default::Default::default()
                        }
                    };
                });
            }
        }

        field_assignments.push(quote! { #field_ident });
    }

    Ok(quote! {
        impl #impl_generics ::yav::server::FromFormData for #struct_ident #ty_generics #where_clause {
            fn from_form(form: &::yav::server::FormData) -> ::std::result::Result<Self, ::yav::server::FormErrors> {
                let mut __errors = ::yav::server::FormErrors::new();
                #(#field_validations)*
                if __errors.has_errors() {
                    ::std::result::Result::Err(__errors)
                } else {
                    ::std::result::Result::Ok(Self {
                        #(#field_assignments),*
                    })
                }
            }
        }
    })
}
