//! Button component for user interaction and navigation.
//!
//! Provides a flexible, accessible button that can render either as a native
//! `<button>` element or as an `<a>` link when an `href` is supplied.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;
use crate::{ButtonVariant, Size, Variant};

/// A customizable button component.
///
/// Can be rendered as a semantic `<button>` or as an `<a>` link when configured
/// with an `href`. Supports standard variants, sizes, icons, and ARIA attributes.
///
/// # Examples
/// ```
/// use yav_ui::{Button, Variant, Size};
/// use yav_core::render_to_string;
///
/// let btn = Button::new("Click me")
///     .variant(Variant::Primary)
///     .size(Size::Md)
///     .render();
///
/// let html = render_to_string(&btn);
/// assert!(html.contains("<button"));
/// assert!(html.contains("Click me"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: Size,
    disabled: bool,
    class: String,
    btn_type: String,
    href: Option<String>,
    children: Vec<Node>,
    id: Option<String>,
    name: Option<String>,
    aria_label: Option<String>,
    aria_expanded: Option<bool>,
    aria_controls: Option<String>,
    icon_left: Option<Node>,
    icon_right: Option<Node>,
}

impl Button {
    /// Create a new button with a text label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: Variant::Primary,
            size: Size::Md,
            disabled: false,
            class: String::new(),
            btn_type: "button".to_string(),
            href: None,
            children: Vec::new(),
            id: None,
            name: None,
            aria_label: None,
            aria_expanded: None,
            aria_controls: None,
            icon_left: None,
            icon_right: None,
        }
    }

    /// Create an empty button without an initial label.
    pub fn empty() -> Self {
        Self::new("")
    }

    /// Set the visual variant of the button.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the size of the button.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set whether the button is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add custom CSS classes to the button.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set the button's `type` attribute (e.g. `"button"`, `"submit"`, `"reset"`).
    pub fn btn_type(mut self, btn_type: impl Into<String>) -> Self {
        self.btn_type = btn_type.into();
        self
    }

    /// Set an `href` URL, causing the button to render as an `<a>` element.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Add a child node to the button content.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes to the button content.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Set the DOM `id` attribute.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the form `name` attribute.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the `aria-label` attribute for screen readers.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the `aria-expanded` attribute.
    pub fn aria_expanded(mut self, expanded: bool) -> Self {
        self.aria_expanded = Some(expanded);
        self
    }

    /// Set the `aria-controls` attribute.
    pub fn aria_controls(mut self, controls: impl Into<String>) -> Self {
        self.aria_controls = Some(controls.into());
        self
    }

    /// Add an icon before the label.
    pub fn icon_left(mut self, icon: impl IntoNode) -> Self {
        self.icon_left = Some(icon.into_node());
        self
    }

    /// Add an icon after the label.
    pub fn icon_right(mut self, icon: impl IntoNode) -> Self {
        self.icon_right = Some(icon.into_node());
        self
    }

    /// Render the button into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_classes = "inline-flex items-center justify-center font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2";

        let variant_classes = match self.variant {
            Variant::Primary => {
                "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500"
            }
            Variant::Secondary => {
                "bg-gray-100 text-gray-900 hover:bg-gray-200 focus:ring-gray-500 dark:bg-gray-800 dark:text-gray-100 dark:hover:bg-gray-700"
            }
            Variant::Outline => {
                "border border-gray-300 bg-transparent text-gray-700 hover:bg-gray-50 focus:ring-blue-500 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
            }
            Variant::Ghost => {
                "bg-transparent text-gray-700 hover:bg-gray-100 focus:ring-gray-500 dark:text-gray-200 dark:hover:bg-gray-800"
            }
            Variant::Danger => {
                "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500"
            }
            Variant::Success => {
                "bg-green-600 text-white hover:bg-green-700 focus:ring-green-500"
            }
            Variant::Warning => {
                "bg-yellow-500 text-white hover:bg-yellow-600 focus:ring-yellow-500"
            }
        };

        let size_classes = match self.size {
            Size::Sm => "px-3 py-1.5 text-sm rounded-md gap-1.5",
            Size::Md => "px-4 py-2 text-sm rounded-md gap-2",
            Size::Lg => "px-6 py-3 text-base rounded-lg gap-2.5",
            Size::Xl => "px-8 py-4 text-lg rounded-xl gap-3",
        };

        let disabled_classes = if self.disabled {
            "opacity-50 pointer-events-none cursor-not-allowed"
        } else {
            ""
        };

        let full_class = merge_classes(&[
            base_classes,
            variant_classes,
            size_classes,
            disabled_classes,
            &self.class,
        ]);

        let mut element = if let Some(href) = &self.href {
            let mut el = Element::new("a").attr("role", "button");
            if self.disabled {
                el = el.attr("aria-disabled", "true").attr("tabindex", "-1");
            } else {
                el = el.attr("href", href);
            }
            el
        } else {
            let mut el = Element::new("button").attr("type", &self.btn_type);
            if self.disabled {
                el = el.bool_attr("disabled", true);
            }
            el
        };

        if !full_class.is_empty() {
            element = element.attr("class", full_class);
        }

        if let Some(id) = &self.id {
            element = element.attr("id", id);
        }

        if let Some(name) = &self.name {
            element = element.attr("name", name);
        }

        if let Some(aria_label) = &self.aria_label {
            element = element.attr("aria-label", aria_label);
        }

        if let Some(expanded) = self.aria_expanded {
            element = element.attr("aria-expanded", if expanded { "true" } else { "false" });
        }

        if let Some(controls) = &self.aria_controls {
            element = element.attr("aria-controls", controls);
        }

        // Add left icon
        if let Some(icon) = self.icon_left {
            element = element.child(icon);
        }

        // Add main label text
        if !self.label.is_empty() {
            element = element.child(Node::text(self.label));
        }

        // Add custom children
        for child in self.children {
            element = element.child(child);
        }

        // Add right icon
        if let Some(icon) = self.icon_right {
            element = element.child(icon);
        }

        Node::Element(element)
    }
}

/// Convenience function to construct a new [`Button`].
pub fn button(label: impl Into<String>) -> Button {
    Button::new(label)
}

/// Properties for constructing a `Button` via `rsx!` or JSX syntax.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ButtonProps {
    pub label: Option<String>,
    pub variant: Option<String>,
    pub size: Option<String>,
    pub disabled: bool,
    pub class: String,
    pub btn_type: Option<String>,
    pub href: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub aria_label: Option<String>,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Button`, enabling `<Button ...>...</Button>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Button(props: ButtonProps) -> Node {
    let mut b = if let Some(label) = props.label {
        Button::new(label)
    } else {
        Button::empty()
    };
    if let Some(v) = props.variant {
        match v.to_lowercase().as_str() {
            "primary" => b = b.variant(Variant::Primary),
            "secondary" => b = b.variant(Variant::Secondary),
            "danger" => b = b.variant(Variant::Danger),
            "success" => b = b.variant(Variant::Success),
            "warning" => b = b.variant(Variant::Warning),
            "ghost" => b = b.variant(Variant::Ghost),
            "outline" => b = b.variant(Variant::Outline),
            _ => {}
        }
    }
    if let Some(s) = props.size {
        match s.to_lowercase().as_str() {
            "sm" => b = b.size(Size::Sm),
            "md" => b = b.size(Size::Md),
            "lg" => b = b.size(Size::Lg),
            "xl" => b = b.size(Size::Xl),
            _ => {}
        }
    }
    if props.disabled {
        b = b.disabled(true);
    }
    if !props.class.is_empty() {
        b = b.class(props.class);
    }
    if let Some(bt) = props.btn_type {
        b = b.btn_type(bt);
    }
    if let Some(href) = props.href {
        b = b.href(href);
    }
    if let Some(id) = props.id {
        b = b.id(id);
    }
    if let Some(name) = props.name {
        b = b.name(name);
    }
    if let Some(al) = props.aria_label {
        b = b.aria_label(al);
    }
    for child in props.children {
        b = b.child(child);
    }
    b.render()
}

impl IntoNode for Button {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Button> for Node {
    fn from(btn: Button) -> Self {
        btn.render()
    }
}

impl Component for Button {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn button_renders_button_tag() {
        let node = Button::new("Click me").render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<button"));
        assert!(html.contains("type=\"button\""));
        assert!(html.contains("Click me"));
        assert!(html.ends_with("</button>"));
    }

    #[test]
    fn button_renders_link_when_href_set() {
        let node = Button::new("Home").href("/home").render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<a"));
        assert!(html.contains("href=\"/home\""));
        assert!(html.contains("role=\"button\""));
        assert!(html.contains("Home"));
        assert!(html.ends_with("</a>"));
    }

    #[test]
    fn button_disabled_state() {
        let btn_disabled = Button::new("Save").disabled(true).render();
        let html = render_to_string(&btn_disabled);
        assert!(html.contains("disabled"));
        assert!(html.contains("opacity-50"));

        let link_disabled = Button::new("Link").href("/path").disabled(true).render();
        let html_link = render_to_string(&link_disabled);
        assert!(html_link.contains("aria-disabled=\"true\""));
        assert!(html_link.contains("tabindex=\"-1\""));
    }

    #[test]
    fn button_variants_produce_different_classes() {
        let primary = Button::new("P").variant(Variant::Primary).render();
        let danger = Button::new("D").variant(Variant::Danger).render();
        let ghost = Button::new("G").variant(Variant::Ghost).render();

        let p_html = render_to_string(&primary);
        let d_html = render_to_string(&danger);
        let g_html = render_to_string(&ghost);

        assert!(p_html.contains("bg-blue-600"));
        assert!(d_html.contains("bg-red-600"));
        assert!(g_html.contains("bg-transparent"));
    }

    #[test]
    fn button_sizes_produce_different_classes() {
        let sm = Button::new("S").size(Size::Sm).render();
        let lg = Button::new("L").size(Size::Lg).render();

        assert!(render_to_string(&sm).contains("px-3 py-1.5"));
        assert!(render_to_string(&lg).contains("px-6 py-3"));
    }

    #[test]
    fn button_with_custom_class_and_aria() {
        let node = Button::new("Menu")
            .class("custom-btn shadow-md")
            .aria_label("Open main menu")
            .aria_expanded(true)
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("custom-btn shadow-md"));
        assert!(html.contains("aria-label=\"Open main menu\""));
        assert!(html.contains("aria-expanded=\"true\""));
    }

    #[test]
    fn button_with_icons() {
        let icon_l = Node::Element(Element::new("svg").attr("class", "icon-left"));
        let icon_r = Node::Element(Element::new("svg").attr("class", "icon-right"));

        let node = Button::new("Action")
            .icon_left(icon_l)
            .icon_right(icon_r)
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("icon-left"));
        assert!(html.contains("Action"));
        assert!(html.contains("icon-right"));
    }

    #[test]
    fn button_disabled_link_omits_href() {
        let active_link = Button::new("Link").href("/dashboard").render();
        let html_active = render_to_string(&active_link);
        assert!(html_active.contains(r#"href="/dashboard""#));

        let disabled_link = Button::new("Link")
            .href("/dashboard")
            .disabled(true)
            .render();
        let html_disabled = render_to_string(&disabled_link);
        assert!(!html_disabled.contains("href="));
        assert!(html_disabled.contains(r#"aria-disabled="true""#));
        assert!(html_disabled.contains(r#"tabindex="-1""#));
    }
}
