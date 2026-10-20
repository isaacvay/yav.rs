//! YAV Macros — Procedural macros for the Yav web framework.
//!
//! Provides:
//! - `rsx!` — JSX-like HTML authoring macro
//! - `#[component]` — attribute macro for component functions

extern crate proc_macro;

mod codegen;
mod component;
mod file_component;
mod parser;
mod routes_macro;

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
