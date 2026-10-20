//! SVG icon rendering for Yav virtual DOM nodes.
//!
//! Provides [`render_icon`] and the [`Icon`] component to render Classic,
//! Minimal, and System SVG icons directly into [`yav_core::Node`] trees.

use crate::catalog::{IconData, IconFamily, IconStyle};
use std::fmt;
use yav_core::{Component, Element, IntoNode, Node};

/// Standard size presets for rendered icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconSize {
    /// Small size: 16x16 pixels.
    Sm,
    /// Medium size: 20x20 pixels (default).
    #[default]
    Md,
    /// Large size: 24x24 pixels.
    Lg,
    /// Extra Large size: 32x32 pixels.
    Xl,
    /// Custom size in pixels.
    Custom(u32),
}

impl IconSize {
    /// Returns the pixel dimension (width and height) for this size.
    pub const fn pixels(self) -> u32 {
        match self {
            Self::Sm => 16,
            Self::Md => 20,
            Self::Lg => 24,
            Self::Xl => 32,
            Self::Custom(px) => px,
        }
    }
}

impl From<IconSize> for u32 {
    fn from(size: IconSize) -> Self {
        size.pixels()
    }
}

impl fmt::Display for IconSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pixels())
    }
}

/// Fallback SVG inner content used when an unknown icon is requested.
///
/// Renders a question mark inside a circle (Lucide `help-circle` style).
pub const FALLBACK_SVG: &str = r#"<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" x2="12.01" y1="17" y2="17"/>"#;

/// Render an icon into a [`yav_core::Node`].
///
/// # Arguments
/// * `name` — Canonical name or query with optional prefix (e.g. `"home"`, `"classic:home"`, `"system:bold:user"`).
/// * `size` — The icon size preset or custom pixel dimension.
/// * `class` — Optional CSS classes to append to the `<svg>` element.
/// * `aria_label` — Optional accessibility label. If provided, sets `aria-label`
///   and `role="img"`. If omitted or empty, sets `aria-hidden="true"`.
///
/// If `name` is not found in the catalog, a visible fallback icon (a question mark
/// in a circle) is rendered and a warning is logged to stderr.
///
/// # Examples
/// ```
/// use yav_icons::{render_icon, IconSize};
///
/// let home_node = render_icon("home", IconSize::Md, Some("text-blue-500"), Some("Home"));
/// let html = yav_core::render_to_string(&home_node);
/// assert!(html.starts_with("<svg"));
/// assert!(html.contains(r#"viewBox="0 0 24 24""#));
/// ```
pub fn render_icon(
    name: &str,
    size: IconSize,
    class: Option<&str>,
    aria_label: Option<&str>,
) -> yav_core::Node {
    let icon_data = crate::catalog::get_icon(name);
    render_icon_internal(icon_data, name, size, class, aria_label)
}

/// Internal rendering helper for both `render_icon` and `Icon`.
fn render_icon_internal(
    icon_data: Option<&IconData>,
    fallback_name: &str,
    size: IconSize,
    class: Option<&str>,
    aria_label: Option<&str>,
) -> yav_core::Node {
    let (svg_content, view_box, stroke_width, is_fill) = match icon_data {
        Some(icon) => (
            icon.svg_content,
            icon.view_box,
            icon.stroke_width,
            icon.is_fill,
        ),
        None => {
            eprintln!("[yav-icons] Warning: Unknown icon '{}'", fallback_name);
            (FALLBACK_SVG, "0 0 24 24", "2", false)
        }
    };

    let px = size.pixels().to_string();
    let mut element = Element::new("svg")
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("width", &px)
        .attr("height", &px)
        .attr("viewBox", view_box);

    if is_fill {
        element = element.attr("fill", "currentColor").attr("stroke", "none");
    } else {
        element = element.attr("fill", "none").attr("stroke", "currentColor");
        if !stroke_width.is_empty() {
            element = element
                .attr("stroke-width", stroke_width)
                .attr("stroke-linecap", "round")
                .attr("stroke-linejoin", "round");
        }
    }

    // Accessibility attributes
    match aria_label {
        Some(label) if !label.trim().is_empty() => {
            element = element.attr("aria-label", label.trim()).attr("role", "img");
        }
        _ => {
            element = element.attr("aria-hidden", "true");
        }
    }

    // Additional CSS classes
    if let Some(cls) = class {
        let trimmed = cls.trim();
        if !trimmed.is_empty() {
            element = element.attr("class", trimmed);
        }
    }

    element = element.child(Node::raw(svg_content));
    Node::Element(element)
}

/// A fluent builder component for rendering icons.
///
/// Implements [`yav_core::Component`] and [`yav_core::IntoNode`].
///
/// # Examples
/// ```
/// use yav_icons::{Icon, IconFamily, IconStyle, IconSize};
/// use yav_core::Component;
///
/// let icon = Icon::new("search")
///     .size(IconSize::Lg)
///     .class("icon-search")
///     .aria_label("Search site");
///
/// let node = icon.render();
/// let html = yav_core::render_to_string(&node);
/// assert!(html.contains(r#"width="24""#));
/// assert!(html.contains(r#"class="icon-search""#));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon<'a> {
    name: &'a str,
    family: Option<IconFamily>,
    style: Option<IconStyle>,
    size: IconSize,
    class: Option<&'a str>,
    aria_label: Option<&'a str>,
}

impl<'a> Icon<'a> {
    /// Create a new icon builder with default classic regular style.
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            family: None,
            style: None,
            size: IconSize::default(),
            class: None,
            aria_label: None,
        }
    }

    /// Create an icon builder explicitly targeting the Classic (Lucide) family.
    pub fn classic(name: &'a str) -> Self {
        Self::new(name).family(IconFamily::Classic)
    }

    /// Create an icon builder explicitly targeting the Minimal (MynaUI) family.
    pub fn minimal(name: &'a str) -> Self {
        Self::new(name).family(IconFamily::Minimal)
    }

    /// Create an icon builder explicitly targeting the System (Phosphor) family.
    pub fn system(name: &'a str) -> Self {
        Self::new(name).family(IconFamily::System)
    }

    /// Set the icon family.
    pub fn family(mut self, family: IconFamily) -> Self {
        self.family = Some(family);
        self
    }

    /// Set the icon style/weight.
    pub fn style(mut self, style: IconStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Convenience: Switch to Regular style.
    pub fn regular(self) -> Self {
        self.style(IconStyle::Regular)
    }

    /// Convenience: Switch to Solid style.
    pub fn solid(self) -> Self {
        self.style(IconStyle::Solid)
    }

    /// Convenience: Switch to Solid / Fill style.
    pub fn fill(self) -> Self {
        self.style(IconStyle::Solid)
    }

    /// Convenience: Switch to Bold style.
    pub fn bold(self) -> Self {
        self.style(IconStyle::Bold)
    }

    /// Convenience: Switch to Light style.
    pub fn light(self) -> Self {
        self.style(IconStyle::Light)
    }

    /// Convenience: Switch to Thin style.
    pub fn thin(self) -> Self {
        self.style(IconStyle::Thin)
    }

    /// Convenience: Switch to Duotone style.
    pub fn duotone(self) -> Self {
        self.style(IconStyle::Duotone)
    }

    /// Set the icon size.
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Set an optional CSS class.
    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    /// Set an accessibility label.
    pub fn aria_label(mut self, label: &'a str) -> Self {
        self.aria_label = Some(label);
        self
    }

    /// Render the icon directly to a [`yav_core::Node`].
    pub fn render_node(&self) -> yav_core::Node {
        let icon_data = match (self.family, self.style) {
            (Some(f), Some(s)) => crate::catalog::get_icon_by(self.name, Some(f), Some(s)),
            (Some(f), None) => crate::catalog::get_icon_by(self.name, Some(f), None),
            (None, Some(s)) => crate::catalog::get_icon_by(self.name, None, Some(s)),
            (None, None) => crate::catalog::get_icon(self.name),
        };
        render_icon_internal(icon_data, self.name, self.size, self.class, self.aria_label)
    }
}

impl<'a> Component for Icon<'a> {
    fn render(&self) -> yav_core::Node {
        self.render_node()
    }
}

impl<'a> IntoNode for Icon<'a> {
    fn into_node(self) -> yav_core::Node {
        self.render_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn render_known_icon_produces_valid_svg() {
        let node = render_icon("home", IconSize::Md, None, None);
        let html = render_to_string(&node);

        assert!(html.starts_with("<svg"));
        assert!(html.ends_with("</svg>"));
        assert!(html.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
        assert!(html.contains(r#"width="20""#));
        assert!(html.contains(r#"height="20""#));
        assert!(html.contains(r#"viewBox="0 0 24 24""#));
        assert!(html.contains(r#"fill="none""#));
        assert!(html.contains(r#"stroke="currentColor""#));
        assert!(html.contains(r#"stroke-width="2""#));
        assert!(html.contains(r#"stroke-linecap="round""#));
        assert!(html.contains(r#"stroke-linejoin="round""#));
        assert!(html.contains(r#"aria-hidden="true""#));
    }

    #[test]
    fn render_minimal_solid_icon_produces_fill_svg() {
        let node = render_icon("minimal:solid:activity", IconSize::Md, None, None);
        let html = render_to_string(&node);

        assert!(html.contains(r#"fill="currentColor""#));
        assert!(html.contains(r#"stroke="none""#));
        assert!(html.contains(r#"viewBox="0 0 24 24""#));
    }

    #[test]
    fn render_system_icon_produces_256_viewbox() {
        let node = render_icon("system:acorn", IconSize::Lg, None, None);
        let html = render_to_string(&node);

        assert!(html.contains(r#"viewBox="0 0 256 256""#));
        assert!(html.contains(r#"stroke-width="16""#));
    }

    #[test]
    fn render_system_bold_icon_produces_bold_stroke() {
        let node = render_icon("system:bold:acorn", IconSize::Lg, None, None);
        let html = render_to_string(&node);

        assert!(html.contains(r#"viewBox="0 0 256 256""#));
        assert!(html.contains(r#"stroke-width="24""#));
    }

    #[test]
    fn render_with_custom_css_class() {
        let node = render_icon(
            "search",
            IconSize::Sm,
            Some("text-red-500 custom-icon"),
            None,
        );
        let html = render_to_string(&node);

        assert!(html.contains(r#"class="text-red-500 custom-icon""#));
    }

    #[test]
    fn render_without_class_omits_class_attribute() {
        let node = render_icon("search", IconSize::Sm, None, None);
        let html = render_to_string(&node);

        assert!(!html.contains("class="));
    }

    #[test]
    fn render_with_aria_label() {
        let node = render_icon("user", IconSize::Md, None, Some("User profile"));
        let html = render_to_string(&node);

        assert!(html.contains(r#"aria-label="User profile""#));
        assert!(html.contains(r#"role="img""#));
        assert!(!html.contains(r#"aria-hidden="true""#));
    }

    #[test]
    fn render_without_aria_label_is_hidden() {
        let node = render_icon("user", IconSize::Md, None, None);
        let html = render_to_string(&node);

        assert!(html.contains(r#"aria-hidden="true""#));
        assert!(!html.contains("aria-label="));
        assert!(!html.contains(r#"role="img""#));
    }

    #[test]
    fn render_unknown_icon_renders_fallback() {
        let node = render_icon("nonexistent-test-icon-123", IconSize::Md, None, None);
        let html = render_to_string(&node);

        assert!(html.starts_with("<svg"));
        assert!(html.contains(FALLBACK_SVG));
    }

    #[test]
    fn icon_sizes_render_correct_dimensions() {
        for (size, px) in [
            (IconSize::Sm, "16"),
            (IconSize::Md, "20"),
            (IconSize::Lg, "24"),
            (IconSize::Xl, "32"),
            (IconSize::Custom(48), "48"),
        ] {
            let node = render_icon("home", size, None, None);
            let html = render_to_string(&node);
            assert!(html.contains(&format!(r#"width="{}""#, px)));
            assert!(html.contains(&format!(r#"height="{}""#, px)));
        }
    }

    #[test]
    fn icon_component_renders_identically() {
        let manual = render_icon("mail", IconSize::Lg, Some("email-link"), Some("Send email"));
        let component = Icon::new("mail")
            .size(IconSize::Lg)
            .class("email-link")
            .aria_label("Send email");

        assert_eq!(
            render_to_string(&manual),
            render_to_string(&component.render())
        );
    }

    #[test]
    fn icon_into_node_conversion() {
        let icon = Icon::new("settings").size(IconSize::Sm);
        let node: yav_core::Node = icon.into_node();
        let html = render_to_string(&node);

        assert!(html.contains(r#"width="16""#));
        assert!(html.contains(r#"height="16""#));
    }
}
