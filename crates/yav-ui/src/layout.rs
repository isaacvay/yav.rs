//! Layout components: Container, Stack, Grid, and Separator.
//!
//! Provides structural primitives for structuring page layout, spacing, and flex/grid alignment.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// Maximum width breakpoint for a [`Container`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerSize {
    Sm,
    Md,
    #[default]
    Lg,
    Xl,
    Full,
}

/// A responsive page container that centers content and constrains maximum width.
///
/// # Examples
/// ```
/// use yav_ui::{Container, ContainerSize};
/// use yav_core::render_to_string;
///
/// let container = Container::new()
///     .size(ContainerSize::Lg)
///     .child("Main content")
///     .render();
///
/// let html = render_to_string(&container);
/// assert!(html.contains("max-w-screen-lg"));
/// assert!(html.contains("mx-auto"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    size: ContainerSize,
    centered: bool,
    padding: bool,
    class: String,
    children: Vec<Node>,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            size: ContainerSize::Lg,
            centered: true,
            padding: true,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl Container {
    /// Create a new default container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum width constraint size.
    pub fn size(mut self, size: ContainerSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether the container is horizontally centered (`mx-auto`).
    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    /// Set whether horizontal responsive padding is applied.
    pub fn padding(mut self, padding: bool) -> Self {
        self.padding = padding;
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

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let size_class = match self.size {
            ContainerSize::Sm => "max-w-screen-sm",
            ContainerSize::Md => "max-w-screen-md",
            ContainerSize::Lg => "max-w-screen-lg",
            ContainerSize::Xl => "max-w-screen-xl",
            ContainerSize::Full => "max-w-full",
        };

        let center_class = if self.centered { "mx-auto" } else { "" };
        let pad_class = if self.padding {
            "px-4 sm:px-6 lg:px-8"
        } else {
            ""
        };
        let full_class =
            merge_classes(&["w-full", size_class, center_class, pad_class, &self.class]);

        let mut el = Element::new("div").attr("class", full_class);
        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Container`].
pub fn container() -> Container {
    Container::new()
}

impl IntoNode for Container {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Container> for Node {
    fn from(c: Container) -> Self {
        c.render()
    }
}

impl Component for Container {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Direction for a [`Stack`] flex container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// Cross-axis alignment for a [`Stack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackAlign {
    Start,
    Center,
    End,
    Baseline,
    Stretch,
}

/// Main-axis distribution for a [`Stack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackJustify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// A flexbox layout component for stacking children vertically or horizontally with spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    direction: StackDirection,
    spacing: u8,
    align: Option<StackAlign>,
    justify: Option<StackJustify>,
    wrap: bool,
    class: String,
    children: Vec<Node>,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            direction: StackDirection::Vertical,
            spacing: 4,
            align: None,
            justify: None,
            wrap: false,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl Stack {
    /// Create a new vertical stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set stack direction to vertical (`flex flex-col`).
    pub fn vertical(mut self) -> Self {
        self.direction = StackDirection::Vertical;
        self
    }

    /// Set stack direction to horizontal (`flex flex-row`).
    pub fn horizontal(mut self) -> Self {
        self.direction = StackDirection::Horizontal;
        self
    }

    /// Set the direction explicitly.
    pub fn direction(mut self, dir: StackDirection) -> Self {
        self.direction = dir;
        self
    }

    /// Set spacing between children using Tailwind gap scale (e.g. 1 to 12).
    pub fn spacing(mut self, spacing: u8) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set cross-axis alignment (`items-start`, `items-center`, etc.).
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// Set main-axis justification (`justify-start`, `justify-between`, etc.).
    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = Some(justify);
        self
    }

    /// Enable flex wrapping (`flex-wrap`).
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
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

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let dir_class = match self.direction {
            StackDirection::Vertical => "flex flex-col",
            StackDirection::Horizontal => "flex flex-row",
        };

        let gap_string = format!("gap-{}", self.spacing);

        let align_class = match self.align {
            Some(StackAlign::Start) => "items-start",
            Some(StackAlign::Center) => "items-center",
            Some(StackAlign::End) => "items-end",
            Some(StackAlign::Baseline) => "items-baseline",
            Some(StackAlign::Stretch) => "items-stretch",
            None => "",
        };

        let justify_class = match self.justify {
            Some(StackJustify::Start) => "justify-start",
            Some(StackJustify::Center) => "justify-center",
            Some(StackJustify::End) => "justify-end",
            Some(StackJustify::Between) => "justify-between",
            Some(StackJustify::Around) => "justify-around",
            Some(StackJustify::Evenly) => "justify-evenly",
            None => "",
        };

        let wrap_class = if self.wrap { "flex-wrap" } else { "" };

        let full_class = merge_classes(&[
            dir_class,
            &gap_string,
            align_class,
            justify_class,
            wrap_class,
            &self.class,
        ]);

        let mut el = Element::new("div").attr("class", full_class);
        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new vertical [`Stack`].
pub fn stack() -> Stack {
    Stack::new()
}

/// Convenience function to construct a new horizontal [`Stack`].
pub fn hstack() -> Stack {
    Stack::new().horizontal()
}

/// Convenience function to construct a new vertical [`Stack`].
pub fn vstack() -> Stack {
    Stack::new().vertical()
}

impl IntoNode for Stack {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Stack> for Node {
    fn from(s: Stack) -> Self {
        s.render()
    }
}

impl Component for Stack {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A CSS grid layout container.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    cols: u8,
    cols_sm: Option<u8>,
    cols_md: Option<u8>,
    cols_lg: Option<u8>,
    cols_xl: Option<u8>,
    gap: u8,
    class: String,
    children: Vec<Node>,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            cols: 1,
            cols_sm: None,
            cols_md: None,
            cols_lg: None,
            cols_xl: None,
            gap: 4,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl Grid {
    /// Create a new grid with 1 column by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base number of columns (1 to 12).
    pub fn cols(mut self, cols: u8) -> Self {
        self.cols = cols.clamp(1, 12);
        self
    }

    /// Convenience helper to set responsive columns for small, medium, and large breakpoints.
    pub fn responsive_cols(mut self, sm: u8, md: u8, lg: u8) -> Self {
        self.cols_sm = Some(sm.clamp(1, 12));
        self.cols_md = Some(md.clamp(1, 12));
        self.cols_lg = Some(lg.clamp(1, 12));
        self
    }

    /// Set columns for `sm` breakpoint.
    pub fn cols_sm(mut self, cols: u8) -> Self {
        self.cols_sm = Some(cols.clamp(1, 12));
        self
    }

    /// Set columns for `md` breakpoint.
    pub fn cols_md(mut self, cols: u8) -> Self {
        self.cols_md = Some(cols.clamp(1, 12));
        self
    }

    /// Set columns for `lg` breakpoint.
    pub fn cols_lg(mut self, cols: u8) -> Self {
        self.cols_lg = Some(cols.clamp(1, 12));
        self
    }

    /// Set columns for `xl` breakpoint.
    pub fn cols_xl(mut self, cols: u8) -> Self {
        self.cols_xl = Some(cols.clamp(1, 12));
        self
    }

    /// Set the grid gap.
    pub fn gap(mut self, gap: u8) -> Self {
        self.gap = gap;
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

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_cols = format!("grid-cols-{}", self.cols);
        let gap_str = format!("gap-{}", self.gap);

        let sm_str = self
            .cols_sm
            .map(|c| format!("sm:grid-cols-{}", c))
            .unwrap_or_default();
        let md_str = self
            .cols_md
            .map(|c| format!("md:grid-cols-{}", c))
            .unwrap_or_default();
        let lg_str = self
            .cols_lg
            .map(|c| format!("lg:grid-cols-{}", c))
            .unwrap_or_default();
        let xl_str = self
            .cols_xl
            .map(|c| format!("xl:grid-cols-{}", c))
            .unwrap_or_default();

        let full_class = merge_classes(&[
            "grid",
            &base_cols,
            &gap_str,
            &sm_str,
            &md_str,
            &lg_str,
            &xl_str,
            &self.class,
        ]);

        let mut el = Element::new("div").attr("class", full_class);
        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Grid`].
pub fn grid() -> Grid {
    Grid::new()
}

impl IntoNode for Grid {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Grid> for Node {
    fn from(g: Grid) -> Self {
        g.render()
    }
}

impl Component for Grid {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Divider orientation for a [`Separator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A visual or semantic separator between content elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Separator {
    orientation: Orientation,
    decorative: bool,
    class: String,
}

impl Default for Separator {
    fn default() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            decorative: true,
            class: String::new(),
        }
    }
}

impl Separator {
    /// Create a new horizontal separator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set orientation to horizontal.
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    /// Set orientation to vertical.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set whether the separator is purely decorative (`aria-hidden="true"` vs `role="separator"`).
    pub fn decorative(mut self, decorative: bool) -> Self {
        self.decorative = decorative;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        match self.orientation {
            Orientation::Horizontal => {
                let base_class = "border-t border-gray-200 dark:border-gray-700 w-full my-4";
                let full_class = merge_classes(&[base_class, &self.class]);
                let mut el = Element::new("hr").attr("class", full_class);
                if self.decorative {
                    el = el.attr("aria-hidden", "true");
                } else {
                    el = el.attr("role", "separator");
                }
                Node::Element(el)
            }
            Orientation::Vertical => {
                let base_class =
                    "border-l border-gray-200 dark:border-gray-700 h-full inline-block mx-2 self-stretch";
                let full_class = merge_classes(&[base_class, &self.class]);
                let mut el = Element::new("div").attr("class", full_class);
                if self.decorative {
                    el = el.attr("aria-hidden", "true");
                } else {
                    el = el
                        .attr("role", "separator")
                        .attr("aria-orientation", "vertical");
                }
                Node::Element(el)
            }
        }
    }
}

/// Convenience function to construct a new [`Separator`].
pub fn separator() -> Separator {
    Separator::new()
}

impl IntoNode for Separator {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Separator> for Node {
    fn from(s: Separator) -> Self {
        s.render()
    }
}

impl Component for Separator {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn container_sizes_and_classes() {
        let sm = Container::new().size(ContainerSize::Sm).render();
        let lg = Container::new().size(ContainerSize::Lg).render();

        assert!(render_to_string(&sm).contains("max-w-screen-sm"));
        assert!(render_to_string(&lg).contains("max-w-screen-lg"));
        assert!(render_to_string(&lg).contains("mx-auto"));
    }

    #[test]
    fn stack_horizontal_and_vertical() {
        let v = Stack::new()
            .vertical()
            .spacing(6)
            .child(Node::text("item1"))
            .render();
        let h = Stack::new()
            .horizontal()
            .align(StackAlign::Center)
            .justify(StackJustify::Between)
            .render();

        let v_html = render_to_string(&v);
        let h_html = render_to_string(&h);

        assert!(v_html.contains("flex flex-col"));
        assert!(v_html.contains("gap-6"));
        assert!(v_html.contains("item1"));

        assert!(h_html.contains("flex flex-row"));
        assert!(h_html.contains("items-center"));
        assert!(h_html.contains("justify-between"));
    }

    #[test]
    fn grid_columns() {
        let node = Grid::new()
            .cols(2)
            .responsive_cols(2, 3, 4)
            .gap(6)
            .child(Node::text("cell"))
            .render();
        let html = render_to_string(&node);

        assert!(html.contains("grid"));
        assert!(html.contains("grid-cols-2"));
        assert!(html.contains("sm:grid-cols-2"));
        assert!(html.contains("md:grid-cols-3"));
        assert!(html.contains("lg:grid-cols-4"));
        assert!(html.contains("gap-6"));
    }

    #[test]
    fn separator_orientations() {
        let h = Separator::new().render();
        let v = Separator::new().vertical().decorative(false).render();

        let h_html = render_to_string(&h);
        let v_html = render_to_string(&v);

        assert!(h_html.starts_with("<hr"));
        assert!(h_html.contains("aria-hidden=\"true\""));

        assert!(v_html.starts_with("<div"));
        assert!(v_html.contains("role=\"separator\""));
        assert!(v_html.contains("aria-orientation=\"vertical\""));
    }
}
