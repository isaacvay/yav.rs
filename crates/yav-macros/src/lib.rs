//! YAV Macros (v0.1.4) — Procedural macros for the Yav web framework.
//!
//! Provides:
//! - [`rsx!`] — JSX-like declarative UI authoring with native `if`/`else`, `for` iteration, class directives, and static subtree folding
//! - [`#[component]`](macro@component) — Attribute macro for ergonomic functional and struct components
//! - [`auto_routes!`] — Compile-time route tree discovery from `src/routes/`
//! - [`t!`] / [`t_strict!`] — Translation interpolation macros with XSS-safe JSON escaping
//! - [`typed_routes!`] — Strongly-typed route constants generation
//! - [`include_rsx!`] / [`include_yav!`] / [`include_routes!`] — External file component embedding

extern crate proc_macro;

mod codegen;
mod component;
mod file_component;
mod form_derive;
mod parser;
mod routes_macro;
mod schema_derive;
mod server_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Write JSX-like HTML markup in Rust.
///
/// # Grammar (simplified)
///
/// ```text
/// rsx_node   := element | expr_block | string_literal
/// element    := '<' tag attributes ('/' '>' | '>' children '<' '/' tag '>')
/// attributes := (name '=' string_literal | name '=' expr_block | name)*
/// children   := rsx_node*
/// expr_block := '{' rust_expr '}'
/// tag        := identifier  (lowercase = HTML, PascalCase = component)
/// ```
///
/// # Example
/// ```ignore
/// use yav::prelude::*;
///
/// let page = rsx! {
///     <div class="container">
///         <h1>"Hello, world!"</h1>
///         <p>"Built with Yav."</p>
///     </div>
/// };
/// ```
#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.into();
    match parser::parse_rsx(input2) {
        Ok(node) => codegen::generate(node).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Include and compile an external `.rsx` template file.
///
/// # Example
/// ```ignore
/// let card = include_rsx!("templates/card.rsx");
/// ```
#[proc_macro]
pub fn include_rsx(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as file_component::IncludeFileInput);
    match file_component::include_template_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Include and compile an external `.yav` template file.
///
/// # Example
/// ```ignore
/// let card = include_yav!("templates/card.yav");
/// ```
#[proc_macro]
pub fn include_yav(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as file_component::IncludeFileInput);
    match file_component::include_template_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Define a component from an external `.yav` or `.rsx` file.
///
/// The file can have optional frontmatter between `---` markers containing
/// imports and `pub struct Props { ... }`, followed by the RSX markup.
///
/// # Example
/// ```ignore
/// yav_component!("components/UserCard.yav");
///
/// let card = UserCard::new(Props { name: "Alice".into() });
/// ```
#[proc_macro]
pub fn yav_component(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as file_component::YavComponentInput);
    match file_component::yav_component_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Mark a function as a Yav component.
///
/// The function should return `yav_core::Node` and can accept props as arguments.
///
/// # Example
/// ```ignore
/// #[component]
/// pub fn Greeting(name: String) -> Node {
///     rsx! { <p>{format!("Hello, {}!", name)}</p> }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match component::component_impl(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Scan a directory of route files at compile-time and return route tuples.
///
/// Returns a slice of `(&'static str, &'static str)` representing `(url_pattern, file_relative_path)`.
///
/// # Example
/// ```ignore
/// let routes: &[(&str, &str)] = include_routes!("src/routes");
/// ```
#[proc_macro]
pub fn include_routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as routes_macro::IncludeRoutesInput);
    match routes_macro::include_routes_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Automatically register discovered route handlers from a directory into a `YavApp` instance.
///
/// Discovers all `.rs` files in the given route directory (default: `"src/routes"`), inspects their
/// public functions (`get`, `post`, `handler`, `page`, etc.), and chains the corresponding
/// `.route()` registrations onto the passed application builder.
///
/// # Example
/// ```ignore
/// use yav::prelude::*;
///
/// let app = yav::server::YavApp::new();
/// let app = yav::auto_routes!(app);
/// ```
#[proc_macro]
pub fn auto_routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as routes_macro::AutoRoutesInput);
    match routes_macro::auto_routes_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate type-safe route constant definitions from the routes directory.
///
/// # Example
/// ```ignore
/// yav::typed_routes!();
/// assert_eq!(Routes::ROOT, "/");
/// ```
#[proc_macro]
pub fn typed_routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as routes_macro::TypedRoutesInput);
    match routes_macro::typed_routes_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct TranslationMacroInput {
    key: syn::Expr,
    args: Vec<(syn::Expr, syn::Expr)>,
}

impl syn::parse::Parse for TranslationMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: syn::Expr = input.parse()?;
        let mut args = Vec::new();
        while input.peek(syn::Token![,]) {
            let _: syn::Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }
            let param_key: syn::Expr = input.parse()?;
            let _: syn::Token![=>] = input.parse()?;
            let param_val: syn::Expr = input.parse()?;
            args.push((param_key, param_val));
        }
        Ok(Self { key, args })
    }
}

fn check_compile_time_locale_key(key_lit: &syn::LitStr) -> Result<(), syn::Error> {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(d) => std::path::PathBuf::from(d),
        Err(_) => return Ok(()),
    };

    let mut json_files = Vec::new();
    for dir_name in ["locales", "i18n"] {
        let dir = manifest_dir.join(dir_name);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    json_files.push(path);
                }
            }
        }
    }

    if json_files.is_empty() {
        return Ok(());
    }

    let needle = format!("\"{}\"", key_lit.value());
    let found = json_files.iter().any(|path| {
        std::fs::read_to_string(path)
            .map(|content| content.contains(&needle))
            .unwrap_or(false)
    });

    if !found {
        return Err(syn::Error::new(
            key_lit.span(),
            format!(
                "Strict i18n compile-time check failed: key '{}' was not found in any locale JSON file under locales/ or i18n/",
                key_lit.value()
            ),
        ));
    }

    Ok(())
}

/// Translate an i18n key or return it as fallback if no catalog is attached.
///
/// Supports optional named parameter interpolation:
/// ```ignore
/// let title = t!("nav.overview");
/// let greeting = t!("welcome.user", "name" => username);
/// ```
#[proc_macro]
pub fn t(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as TranslationMacroInput);
    let key = &parsed.key;
    let pair_defs: Vec<proc_macro2::TokenStream> = parsed
        .args
        .iter()
        .map(|(k, v)| {
            quote::quote! {
                let __arg_str = (#v).to_string();
                __pairs.push((#k, __arg_str));
            }
        })
        .collect();

    let core = crate::codegen::yav_core_root();

    let tokens = quote::quote! {
        {
            let __yav_key = #key;
            let mut __pairs: ::std::vec::Vec<(&str, ::std::string::String)> = ::std::vec::Vec::new();
            #(#pair_defs)*
            let __borrowed_pairs: ::std::vec::Vec<(&str, &str)> = __pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
            if let Some(i18n) = #core::use_context::<#core::I18n>() {
                let locale = #core::use_context::<String>()
                    .unwrap_or_else(|| i18n.default_locale().to_string());
                i18n.translate(&locale, __yav_key, &__borrowed_pairs)
            } else {
                let mut __res = __yav_key.to_string();
                for (k, v) in &__borrowed_pairs {
                    let needle = format!("{{{}}}", k);
                    __res = __res.replace(&needle, v);
                }
                __res
            }
        }
    };
    tokens.into()
}

/// Translate an i18n key strictly: validates against `locales/*.json` at compile time if present,
/// and panics at runtime if the key is missing from the active locale catalog.
///
/// # Example
/// ```ignore
/// let title = t_strict!("nav.overview");
/// ```
#[proc_macro]
pub fn t_strict(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as TranslationMacroInput);
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(ref lit_str),
        ..
    }) = parsed.key
    {
        if let Err(err) = check_compile_time_locale_key(lit_str) {
            return err.to_compile_error().into();
        }
    }

    let key = &parsed.key;
    let pair_defs: Vec<proc_macro2::TokenStream> = parsed
        .args
        .iter()
        .map(|(k, v)| {
            quote::quote! {
                let __arg_str = (#v).to_string();
                __pairs.push((#k, __arg_str));
            }
        })
        .collect();

    let core = crate::codegen::yav_core_root();

    let tokens = quote::quote! {
        {
            let __yav_key = #key;
            let mut __pairs: ::std::vec::Vec<(&str, ::std::string::String)> = ::std::vec::Vec::new();
            #(#pair_defs)*
            let __borrowed_pairs: ::std::vec::Vec<(&str, &str)> = __pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
            if let Some(i18n) = #core::use_context::<#core::I18n>() {
                let locale = #core::use_context::<String>()
                    .unwrap_or_else(|| i18n.default_locale().to_string());
                if !i18n.has_key(&locale, __yav_key) {
                    panic!("Strict i18n lookup failed: key '{}' missing for locale '{}'", __yav_key, locale);
                }
                i18n.translate(&locale, __yav_key, &__borrowed_pairs)
            } else {
                panic!("Strict i18n lookup failed: no I18n context found when resolving key '{}'", __yav_key);
            }
        }
    };
    tokens.into()
}

/// Derive [`FromFormData`](::yav::server::FromFormData) for a struct with declarative field validation rules.
///
/// # Supported `#[form(...)]` Field Attributes
/// - `#[form(required = "Email is required")]` or `#[form(required)]`
/// - `#[form(email = "Invalid email format")]` or `#[form(email)]`
/// - `#[form(min_length = 8, message = "Minimum 8 characters")]`
/// - `#[form(max_length = 100)]`
/// - `#[form(min = 1, max = 120)]`
/// - `#[form(matches = "password", message = "Passwords must match")]`
#[proc_macro_derive(FromFormData, attributes(form))]
pub fn derive_from_form_data(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as syn::DeriveInput);
    match form_derive::derive_from_form_data_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive OpenAPI 3.1 JSON Schema and TypeScript interface definitions (`schema_name()`,
/// `openapi_schema_json()`, `ts_interface()`) for a struct.
#[proc_macro_derive(ApiSchema)]
pub fn derive_api_schema(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as syn::DeriveInput);
    match schema_derive::derive_api_schema_impl(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Transform an `async fn` into a type-safe Server Function / Server Action (RPC).
///
/// Generates `<fn_name>_url()`, `<fn_name>_handler()`, and `register_<fn_name>(app)`.
#[proc_macro_attribute]
pub fn server(attr: TokenStream, item: TokenStream) -> TokenStream {
    match server_macro::server_macro_impl(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
