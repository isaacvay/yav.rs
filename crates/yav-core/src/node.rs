//! Virtual DOM node types for the Yav framework.
//!
//! The node tree is a lightweight representation of HTML that gets rendered
//! to strings for server-side rendering. Components produce [`Node`] trees
//! which the renderer serializes to valid HTML.

use std::borrow::Cow;
use std::fmt;

/// A node in the virtual DOM tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// An HTML element such as `<div>`, `<p>`, `<a>`, etc.
    Element(Element),
    /// Escaped text content. All text is HTML-escaped during rendering.
    Text(String),
    /// Raw HTML injected without escaping. **Use with extreme caution** —
    /// only for trusted content. Never use with user input.
    Raw(String),
    /// A pre-rendered, compile-time static HTML string requiring zero allocations and no escaping.
    Static(&'static str),
    /// A fragment grouping multiple nodes without a wrapper element.
    Fragment(Vec<Node>),
    /// An empty node that renders nothing. Useful for conditional rendering.
    Empty,
}

/// An HTML element with a tag name, attributes, and children.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// The HTML tag name (e.g., `"div"`, `"span"`, `"input"`).
    pub tag: Cow<'static, str>,
    /// The element's attributes.
    pub attributes: Vec<Attribute>,
    /// Child nodes.
    pub children: Vec<Node>,
    /// Whether this is a void/self-closing element (e.g., `<input />`, `<br />`).
    pub self_closing: bool,
}

/// An HTML attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The attribute name (e.g., `"class"`, `"href"`, `"data-id"`).
    pub name: Cow<'static, str>,
    /// The attribute value.
    pub value: AttributeValue,
}

/// The value of an HTML attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// A string value: `class="foo"` → `AttributeValue::String("foo")`
    String(String),
    /// A zero-allocation static string literal: `AttributeValue::Static("container")`
    Static(&'static str),
    /// A boolean attribute: `disabled` → `AttributeValue::Bool(true)`.
    /// When true, the attribute name is rendered without a value.
    /// When false, the attribute is omitted entirely.
    Bool(bool),
    /// No explicit value — the attribute name is rendered alone.
    None,
}

// ── Void (self-closing) elements per HTML spec ──────────────────────

/// HTML void elements that must not have children and are self-closing.
pub const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Returns `true` if the given tag is a void element per the HTML spec.
pub fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag)
}

// ── Builder helpers ─────────────────────────────────────────────────

impl Node {
    /// Create a text node. Content will be HTML-escaped during rendering.
    pub fn text(content: impl Into<String>) -> Self {
        Node::Text(content.into())
    }

    /// Create a raw HTML node. **Not escaped** — only use with trusted content.
    pub fn raw(html: impl Into<String>) -> Self {
        Node::Raw(html.into())
    }

    /// Create a static HTML node from a compile-time string literal. Zero allocation.
    pub fn static_str(content: &'static str) -> Self {
        Node::Static(content)
    }

    /// Create an empty node.
    pub fn empty() -> Self {
        Node::Empty
    }

    /// Create a fragment from a vector of nodes.
    pub fn fragment(children: Vec<Node>) -> Self {
        Node::Fragment(children)
    }

    /// Returns `true` if this node produces no visible output.
    pub fn is_empty(&self) -> bool {
        match self {
            Node::Empty => true,
            Node::Text(s) => s.is_empty(),
            Node::Raw(s) => s.is_empty(),
            Node::Static(s) => s.is_empty(),
            Node::Fragment(children) => children.iter().all(|c| c.is_empty()),
            Node::Element(_) => false,
        }
    }
}

impl Element {
    /// Create a new element with the given tag name. Accepts either static `&'static str` (zero allocation) or `String`.
    #[must_use]
    pub fn new(tag: impl Into<Cow<'static, str>>) -> Self {
        let tag = tag.into();
        let self_closing = is_void_element(&tag);
        Element {
            tag,
            attributes: Vec::new(),
            children: Vec::new(),
            self_closing,
        }
    }

    /// Add an attribute with a string value.
    #[must_use]
    pub fn attr(mut self, name: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        self.attributes.push(Attribute {
            name: name.into(),
            value: AttributeValue::String(value.into()),
        });
        self
    }

    /// Add an attribute with a compile-time static string literal (zero heap allocation).
    #[must_use]
    pub fn attr_static(mut self, name: impl Into<Cow<'static, str>>, value: &'static str) -> Self {
        self.attributes.push(Attribute {
            name: name.into(),
            value: AttributeValue::Static(value),
        });
        self
    }

    /// Add a boolean attribute.
    #[must_use]
    pub fn bool_attr(mut self, name: impl Into<Cow<'static, str>>, value: bool) -> Self {
        self.attributes.push(Attribute {
            name: name.into(),
            value: AttributeValue::Bool(value),
        });
        self
    }

    /// Add a child node.
    #[must_use]
    pub fn child(mut self, node: Node) -> Self {
        self.children.push(node);
        self
    }

    /// Add multiple children.
    #[must_use]
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }
}

impl Attribute {
    /// Create a string attribute.
    pub fn new(name: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        Attribute {
            name: name.into(),
            value: AttributeValue::String(value.into()),
        }
    }

    /// Create a compile-time static string attribute (zero heap allocation).
    pub fn static_str(name: impl Into<Cow<'static, str>>, value: &'static str) -> Self {
        Attribute {
            name: name.into(),
            value: AttributeValue::Static(value),
        }
    }

    /// Create a boolean attribute.
    pub fn boolean(name: impl Into<Cow<'static, str>>, value: bool) -> Self {
        Attribute {
            name: name.into(),
            value: AttributeValue::Bool(value),
        }
    }
}

// ── IntoNode conversion trait ───────────────────────────────────────

/// Trait for types that can be converted into a [`Node`].
pub trait IntoNode {
    fn into_node(self) -> Node;
}

impl IntoNode for Node {
    fn into_node(self) -> Node {
        self
    }
}

impl IntoNode for Element {
    fn into_node(self) -> Node {
        Node::Element(self)
    }
}

impl IntoNode for String {
    fn into_node(self) -> Node {
        Node::Text(self)
    }
}

impl IntoNode for &str {
    fn into_node(self) -> Node {
        Node::Text(self.to_string())
    }
}

impl<T: IntoNode> IntoNode for Option<T> {
    fn into_node(self) -> Node {
        match self {
            Some(v) => v.into_node(),
            None => Node::Empty,
        }
    }
}

impl IntoNode for Vec<Node> {
    fn into_node(self) -> Node {
        Node::Fragment(self)
    }
}

macro_rules! impl_into_node_for_display {
    ($($t:ty),*) => {
        $(
            impl IntoNode for $t {
                fn into_node(self) -> Node {
                    Node::Text(self.to_string())
                }
            }
        )*
    };
}

impl_into_node_for_display!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char
);

// ── IntoAttributeValue conversion trait ─────────────────────────────

/// Trait for types that can be converted into an [`AttributeValue`].
///
/// Used by the `rsx!` macro for dynamic attribute expressions (`attr={expr}`):
/// - `bool` converts to [`AttributeValue::Bool`] (`false` omits the attribute, `true` renders a boolean attribute)
/// - `Option<T>` renders the inner value when `Some(v)` or omits the attribute when `None`
/// - Strings and numbers convert to [`AttributeValue::String`]
pub trait IntoAttributeValue {
    fn into_attribute_value(self) -> AttributeValue;
}

impl IntoAttributeValue for AttributeValue {
    fn into_attribute_value(self) -> AttributeValue {
        self
    }
}

impl IntoAttributeValue for bool {
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Bool(self)
    }
}

impl IntoAttributeValue for String {
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::String(self)
    }
}

impl IntoAttributeValue for &String {
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::String(self.clone())
    }
}

impl IntoAttributeValue for &str {
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::String(self.to_string())
    }
}

impl IntoAttributeValue for Cow<'_, str> {
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::String(self.into_owned())
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_attribute_value(self) -> AttributeValue {
        match self {
            Some(v) => v.into_attribute_value(),
            None => AttributeValue::Bool(false),
        }
    }
}

macro_rules! impl_into_attr_value_for_display {
    ($($t:ty),*) => {
        $(
            impl IntoAttributeValue for $t {
                fn into_attribute_value(self) -> AttributeValue {
                    AttributeValue::String(self.to_string())
                }
            }
        )*
    };
}

impl_into_attr_value_for_display!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, char
);

// ── IntoPropValue conversion trait for component props in rsx! ──────

/// Trait for converting string literals in `rsx!` into component property types
/// (`String`, `Option<String>`, `&str`, `Option<&str>`, `Cow<str>`).
pub trait IntoPropValue<T> {
    fn into_prop_value(self) -> T;
}

impl IntoPropValue<String> for &str {
    fn into_prop_value(self) -> String {
        self.to_string()
    }
}

impl IntoPropValue<Option<String>> for &str {
    fn into_prop_value(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl<'a> IntoPropValue<&'a str> for &'a str {
    fn into_prop_value(self) -> &'a str {
        self
    }
}

impl<'a> IntoPropValue<Option<&'a str>> for &'a str {
    fn into_prop_value(self) -> Option<&'a str> {
        Some(self)
    }
}

impl<'a> IntoPropValue<Cow<'a, str>> for &'a str {
    fn into_prop_value(self) -> Cow<'a, str> {
        Cow::Borrowed(self)
    }
}

impl<'a> IntoPropValue<Option<Cow<'a, str>>> for &'a str {
    fn into_prop_value(self) -> Option<Cow<'a, str>> {
        Some(Cow::Borrowed(self))
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::render::render_to_string(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_element_detection() {
        assert!(is_void_element("br"));
        assert!(is_void_element("input"));
        assert!(is_void_element("img"));
        assert!(!is_void_element("div"));
        assert!(!is_void_element("span"));
    }

    #[test]
    fn element_builder() {
        let el = Element::new("div")
            .attr("class", "container")
            .child(Node::text("Hello"));
        assert_eq!(el.tag, "div");
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.children.len(), 1);
        assert!(!el.self_closing);
    }

    #[test]
    fn void_element_auto_self_closing() {
        let el = Element::new("input");
        assert!(el.self_closing);
    }

    #[test]
    fn into_node_string() {
        let node: Node = "hello".into_node();
        assert_eq!(node, Node::Text("hello".to_string()));
    }

    #[test]
    fn into_node_option_none() {
        let node: Node = Option::<String>::None.into_node();
        assert_eq!(node, Node::Empty);
    }
}
