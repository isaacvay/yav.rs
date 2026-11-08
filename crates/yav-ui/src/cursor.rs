//! Native cursor tracking, radial spotlight cards, 3D tilt plates, and magnetic wrappers.
//!
//! Powered by Yav's SSR tree-shaken cursor runtime (`YAV_CURSOR_RUNTIME_SCRIPT`),
//! requiring zero user-written JavaScript.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// A container whose border and surface emit a radial spotlight tracking the cursor (`--yav-mouse-x`, `--yav-mouse-y`).
#[derive(Debug, Clone, PartialEq)]
pub struct CursorSpotlight {
    tag: String,
    radius_px: u32,
    color: String,
    class: String,
    children: Vec<Node>,
}

impl Default for CursorSpotlight {
    fn default() -> Self {
        Self {
            tag: "div".to_string(),
            radius_px: 420,
            color: "rgba(224, 89, 47, 0.14)".to_string(),
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl CursorSpotlight {
    /// Create a new [`CursorSpotlight`] container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the wrapper HTML tag (default `"div"`).
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Set the spotlight radial glow radius in pixels (default `420`).
    pub fn radius_px(mut self, px: u32) -> Self {
        self.radius_px = px;
        self
    }

    /// Set the spotlight glow CSS color (default `"rgba(224, 89, 47, 0.14)"`).
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Add a child node.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Render into a [`Node`].
    pub fn render(self) -> Node {
        let full_class = merge_classes(&["relative overflow-hidden", &self.class]);
        let glow_style = format!(
            "opacity: var(--yav-spotlight-opacity, 0); background: radial-gradient({}px circle at var(--yav-mouse-x, 50%) var(--yav-mouse-y, 50%), {}, transparent 75%);",
            self.radius_px, self.color
        );

        let overlay = Element::new("div")
            .attr(
                "class",
                "pointer-events-none absolute inset-0 transition-opacity duration-300",
            )
            .attr("style", glow_style)
            .attr("aria-hidden", "true");

        let mut el = Element::new(self.tag)
            .attr("class", full_class)
            .attr("yav:cursor", "spotlight")
            .child(Node::Element(overlay));

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

impl IntoNode for CursorSpotlight {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CursorSpotlight> for Node {
    fn from(c: CursorSpotlight) -> Self {
        c.render()
    }
}

impl Component for CursorSpotlight {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience builder for [`CursorSpotlight`].
pub fn cursor_spotlight() -> CursorSpotlight {
    CursorSpotlight::new()
}

/// Props for `<CursorSpotlight />` in `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorSpotlightProps {
    pub tag: String,
    pub radius_px: u32,
    pub color: String,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for CursorSpotlightProps {
    fn default() -> Self {
        Self {
            tag: "div".to_string(),
            radius_px: 420,
            color: "rgba(224, 89, 47, 0.14)".to_string(),
            class: String::new(),
            children: Vec::new(),
        }
    }
}

/// Functional component for `<CursorSpotlight />` in `rsx!`.
#[allow(non_snake_case)]
pub fn CursorSpotlight(props: CursorSpotlightProps) -> Node {
    let mut s = CursorSpotlight::new()
        .tag(if props.tag.is_empty() {
            "div"
        } else {
            &props.tag
        })
        .radius_px(if props.radius_px == 0 {
            420
        } else {
            props.radius_px
        })
        .color(if props.color.is_empty() {
            "rgba(224, 89, 47, 0.14)"
        } else {
            &props.color
        })
        .children(props.children);
    if !props.class.is_empty() {
        s = s.class(props.class);
    }
    s.render()
}

/// A 3D perspective plate that tilts smoothly toward the cursor and optionally displays a spotlight glare.
#[derive(Debug, Clone, PartialEq)]
pub struct TiltPlate {
    max_deg: f64,
    perspective_px: u32,
    spotlight: bool,
    class: String,
    children: Vec<Node>,
}

impl Default for TiltPlate {
    fn default() -> Self {
        Self {
            max_deg: 8.0,
            perspective_px: 800,
            spotlight: true,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl TiltPlate {
    /// Create a new 3D [`TiltPlate`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum tilt angle in degrees (default `8.0`).
    pub fn max_deg(mut self, deg: f64) -> Self {
        self.max_deg = deg;
        self
    }

    /// 3D perspective distance in pixels (default `800`).
    pub fn perspective_px(mut self, px: u32) -> Self {
        self.perspective_px = px;
        self
    }

    /// Enable or disable the radial cursor spotlight glare (default `true`).
    pub fn spotlight(mut self, enabled: bool) -> Self {
        self.spotlight = enabled;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Add a child node.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Render into a [`Node`].
    pub fn render(self) -> Node {
        let mode = if self.spotlight {
            "tilt spotlight"
        } else {
            "tilt"
        };
        let full_class = merge_classes(&[
            "relative overflow-hidden will-change-transform",
            &self.class,
        ]);
        let mut el = Element::new("div")
            .attr("class", full_class)
            .attr("yav:cursor", mode)
            .attr("yav:tilt-max", self.max_deg.to_string())
            .attr("yav:tilt-perspective", format!("{}px", self.perspective_px));

        if self.spotlight {
            let glare = Element::new("div")
                .attr("class", "pointer-events-none absolute inset-0 transition-opacity duration-300")
                .attr(
                    "style",
                    "opacity: var(--yav-spotlight-opacity, 0); background: radial-gradient(380px circle at var(--yav-mouse-x, 50%) var(--yav-mouse-y, 50%), rgba(224, 89, 47, 0.12), transparent 70%);",
                )
                .attr("aria-hidden", "true");
            el = el.child(Node::Element(glare));
        }

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

impl IntoNode for TiltPlate {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<TiltPlate> for Node {
    fn from(t: TiltPlate) -> Self {
        t.render()
    }
}

impl Component for TiltPlate {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience builder for [`TiltPlate`].
pub fn tilt_plate() -> TiltPlate {
    TiltPlate::new()
}

/// Props for `<TiltPlate />` in `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct TiltPlateProps {
    pub max_deg: f64,
    pub perspective_px: u32,
    pub spotlight: bool,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for TiltPlateProps {
    fn default() -> Self {
        Self {
            max_deg: 8.0,
            perspective_px: 800,
            spotlight: true,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

/// Functional component for `<TiltPlate />` in `rsx!`.
#[allow(non_snake_case)]
pub fn TiltPlate(props: TiltPlateProps) -> Node {
    let mut t = TiltPlate::new()
        .max_deg(props.max_deg)
        .perspective_px(if props.perspective_px == 0 {
            800
        } else {
            props.perspective_px
        })
        .spotlight(props.spotlight)
        .children(props.children);
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    t.render()
}

/// A magnetic wrapper that physically pulls its child button/icon toward the cursor and springs back on leave.
#[derive(Debug, Clone, PartialEq)]
pub struct Magnetic {
    strength: f64,
    class: String,
    children: Vec<Node>,
}

impl Default for Magnetic {
    fn default() -> Self {
        Self {
            strength: 0.32,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl Magnetic {
    /// Create a new [`Magnetic`] wrapper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set magnetic attraction strength (`0.1` subtle to `0.6` strong; default `0.32`).
    pub fn strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Add a child node.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Render into a [`Node`].
    pub fn render(self) -> Node {
        let full_class = merge_classes(&["inline-block will-change-transform", &self.class]);
        let mut el = Element::new("div")
            .attr("class", full_class)
            .attr("yav:cursor", "magnetic")
            .attr("yav:magnetic-strength", self.strength.to_string());

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

impl IntoNode for Magnetic {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Magnetic> for Node {
    fn from(m: Magnetic) -> Self {
        m.render()
    }
}

impl Component for Magnetic {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience builder for [`Magnetic`].
pub fn magnetic() -> Magnetic {
    Magnetic::new()
}

/// Props for `<Magnetic />` in `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct MagneticProps {
    pub strength: f64,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for MagneticProps {
    fn default() -> Self {
        Self {
            strength: 0.32,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

/// Functional component for `<Magnetic />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Magnetic(props: MagneticProps) -> Node {
    let mut m = Magnetic::new()
        .strength(props.strength)
        .children(props.children);
    if !props.class.is_empty() {
        m = m.class(props.class);
    }
    m.render()
}

/// A smooth spring-interpolated custom cursor follower element.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorFollower {
    smoothing: f64,
    class: String,
}

impl Default for CursorFollower {
    fn default() -> Self {
        Self {
            smoothing: 0.22,
            class: "fixed top-0 left-0 w-5 h-5 -ml-2.5 -mt-2.5 rounded-full border border-[#e0592f]/60 pointer-events-none z-50 hidden md:block will-change-transform".to_string(),
        }
    }
}

impl CursorFollower {
    /// Create a new [`CursorFollower`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set interpolation smoothing factor (`0.05` slow lag to `0.5` snappy; default `0.22`).
    pub fn smoothing(mut self, smoothing: f64) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// Set custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a [`Node`].
    pub fn render(self) -> Node {
        Node::Element(
            Element::new("div")
                .attr("class", self.class)
                .attr("yav:cursor", "follower")
                .attr("yav:cursor-smoothing", self.smoothing.to_string())
                .attr("aria-hidden", "true"),
        )
    }
}

impl IntoNode for CursorFollower {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CursorFollower> for Node {
    fn from(f: CursorFollower) -> Self {
        f.render()
    }
}

impl Component for CursorFollower {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience builder for [`CursorFollower`].
pub fn cursor_follower() -> CursorFollower {
    CursorFollower::new()
}

/// Props for `<CursorFollower />` in `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorFollowerProps {
    pub smoothing: f64,
    pub class: String,
}

impl Default for CursorFollowerProps {
    fn default() -> Self {
        Self {
            smoothing: 0.22,
            class: String::new(),
        }
    }
}

/// Functional component for `<CursorFollower />` in `rsx!`.
#[allow(non_snake_case)]
pub fn CursorFollower(props: CursorFollowerProps) -> Node {
    let mut f = CursorFollower::new().smoothing(props.smoothing);
    if !props.class.is_empty() {
        f = f.class(props.class);
    }
    f.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn cursor_spotlight_renders_attributes_and_overlay() {
        let node = cursor_spotlight()
            .radius_px(500)
            .class("plate p-6")
            .child(Node::text("Interactive Card"))
            .render();
        let html = render_to_string(&node);
        assert!(html.contains(r#"yav:cursor="spotlight""#));
        assert!(html.contains("500px circle at var(--yav-mouse-x, 50%) var(--yav-mouse-y, 50%)"));
        assert!(html.contains("Interactive Card"));
    }

    #[test]
    fn tilt_plate_and_magnetic_render() {
        let tilt = tilt_plate().max_deg(10.0).perspective_px(900).render();
        let html = render_to_string(&tilt);
        assert!(html.contains(r#"yav:cursor="tilt spotlight""#));
        assert!(html.contains(r#"yav:tilt-max="10""#));
        assert!(html.contains(r#"yav:tilt-perspective="900px""#));

        let mag = magnetic().strength(0.4).child(Node::text("Btn")).render();
        let mag_html = render_to_string(&mag);
        assert!(mag_html.contains(r#"yav:cursor="magnetic""#));
        assert!(mag_html.contains(r#"yav:magnetic-strength="0.4""#));
    }
}
