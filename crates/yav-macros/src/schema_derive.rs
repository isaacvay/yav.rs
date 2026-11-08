//! `#[derive(ApiSchema)]` procedural macro for OpenAPI 3.1 JSON Schema & TypeScript codegen.
//!
//! Generates compile-time schema inspection methods on structs (`schema_name()`,
//! `openapi_schema_json()`, and `ts_interface()`), providing built-in equivalents to
//! `utoipa::ToSchema` and `ts-rs::TS`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Result, Type};

fn analyze_type(ty: &Type) -> (String, String, bool) {
    // Returns (openapi_type_json, ts_type_str, is_optional)
    if let Type::Reference(tr) = ty {
        return analyze_type(&tr.elem);
    }
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let ident = seg.ident.to_string();
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (oa, ts, _) = analyze_type(inner);
                        return (oa, ts, true);
                    }
                }
                return (
                    r#"{"type":"string"}"#.to_string(),
                    "string".to_string(),
                    true,
                );
            }
            if ident == "Box" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return analyze_type(inner);
                    }
                }
            }
            if ident == "HashMap" || ident == "BTreeMap" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    let mut type_args = args.args.iter().filter_map(|a| match a {
                        GenericArgument::Type(t) => Some(t),
                        _ => None,
                    });
                    let _key_ty = type_args.next();
                    if let Some(val_ty) = type_args.next() {
                        let (oa, ts, _) = analyze_type(val_ty);
                        return (
                            format!(r#"{{"type":"object","additionalProperties":{oa}}}"#),
                            format!("Record<string, {ts}>"),
                            false,
                        );
                    }
                }
                return (
                    r#"{"type":"object","additionalProperties":true}"#.to_string(),
                    "Record<string, unknown>".to_string(),
                    false,
                );
            }
            if ident == "Value" {
                return (
                    r#"{"type":"object"}"#.to_string(),
                    "Record<string, any>".to_string(),
                    false,
                );
            }
            if ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (oa, ts, _) = analyze_type(inner);
                        return (
                            format!(r#"{{"type":"array","items":{oa}}}"#),
                            format!("{ts}[]"),
                            false,
                        );
                    }
                }
                return (
                    r#"{"type":"array","items":{"type":"string"}}"#.to_string(),
                    "unknown[]".to_string(),
                    false,
                );
            }
            match ident.as_str() {
                "String" | "str" | "Cow" => {
                    return (
                        r#"{"type":"string"}"#.to_string(),
                        "string".to_string(),
                        false,
                    )
                }
                "bool" => {
                    return (
                        r#"{"type":"boolean"}"#.to_string(),
                        "boolean".to_string(),
                        false,
                    )
                }
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize" => {
                    return (
                        r#"{"type":"integer"}"#.to_string(),
                        "number".to_string(),
                        false,
                    )
                }
                "f32" | "f64" => {
                    return (
                        r#"{"type":"number"}"#.to_string(),
                        "number".to_string(),
                        false,
                    )
                }
                other => {
                    return (
                        format!("{{\"$ref\":\"#/components/schemas/{other}\"}}"),
                        other.to_string(),
                        false,
                    )
                }
            }
        }
    }
    (
        r#"{"type":"string"}"#.to_string(),
        "unknown".to_string(),
        false,
    )
}

pub fn derive_api_schema_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_ident = &input.ident;
    let struct_name = struct_ident.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(ds) => match &ds.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "ApiSchema can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "ApiSchema can only be derived for structs",
            ))
        }
    };

    let mut prop_entries = Vec::new();
    let mut required_fields = Vec::new();
    let mut ts_lines = Vec::new();

    for field in fields {
        let f_name = field.ident.as_ref().unwrap().to_string();
        let (oa_json, ts_ty, is_opt) = analyze_type(&field.ty);
        prop_entries.push(format!(r#""{f_name}":{oa_json}"#));
        if !is_opt {
            required_fields.push(format!(r#""{f_name}""#));
        }
        let opt_mark = if is_opt { "?" } else { "" };
        ts_lines.push(format!("  {f_name}{opt_mark}: {ts_ty};"));
    }

    let openapi_schema_str = format!(
        r#"{{"type":"object","properties":{{{}}},"required":[{}]}}"#,
        prop_entries.join(","),
        required_fields.join(",")
    );

    let ts_interface_str = format!(
        "export interface {struct_name} {{\n{}\n}}",
        ts_lines.join("\n")
    );

    Ok(quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            /// Returns the canonical schema name (`"`#struct_name`"`).
            pub const fn schema_name() -> &'static str {
                #struct_name
            }

            /// Returns the OpenAPI 3.1 JSON Schema definition for this struct.
            pub const fn openapi_schema_json() -> &'static str {
                #openapi_schema_str
            }

            /// Returns the TypeScript `export interface` definition for this struct.
            pub const fn ts_interface() -> &'static str {
                #ts_interface_str
            }
        }
    })
}
