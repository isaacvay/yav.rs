//! # YAV Core
//!
//! Foundation types and rendering engine for the Yav web framework.
//!
//! This crate provides:
//! - [`Node`] / [`Element`] — the virtual DOM tree representation
//! - [`Attribute`] / [`AttributeValue`] — typed HTML attribute handling
//! - [`render_to_string`] — renders a node tree to an HTML string
//! - [`escape_html`] / [`escape_attribute`] — context-aware HTML escaping
//! - [`Component`] trait — interface for renderable components
//! - [`IntoNode`] trait — ergonomic node conversion

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
