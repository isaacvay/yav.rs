//! # YAV Core (v0.1.4)
//!
//! Foundation types and rendering engine for the Yav web framework.
//!
//! This crate provides:
//! - [`Node`] / [`Element`] — Virtual DOM tree representation with zero-allocation [`Node::Static`] compile-time folding
//! - [`Attribute`] / [`AttributeValue`] — Typed HTML attribute handling, class directives, and boolean attributes
//! - [`render_to_string`] / [`document_shell_with_lang`] — Server-Side Rendering (SSR) pipeline with automatic locale injection and SSR tree-shaking of interactive client runtimes
//! - [`Head`] — `<Head>` metadata hoisting for zero-clone document header aggregation
//! - [`escape_html`] / [`escape_attribute`] / [`escape_url`] — Context-aware, XSS-safe HTML and attribute escaping
//! - [`I18n`] — Internationalization catalog with pluralization and XSS-safe JSON escaping
//! - [`Component`] / [`IntoNode`] traits — Interfaces for composable functional and struct components
//! - [`provide_context`] / [`use_context`] — Scoped async request state propagation

mod component;
pub mod context;
pub mod escape;
pub mod i18n;
mod node;
mod render;

pub use component::*;
pub use context::*;
pub use escape::*;
pub use i18n::*;
pub use node::*;
pub use render::*;

/// Convenience prelude for common imports
pub mod prelude {
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::escape::*;
    pub use crate::i18n::*;
    pub use crate::node::*;
    pub use crate::render::*;
}
