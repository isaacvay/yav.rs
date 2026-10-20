//! Feedback components: Alert, Badge, Loading spinner, and Skeleton placeholder.
//!
//! Provides user feedback, status notices, loading indicators, and loading skeletons.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;
use crate::{Size, Variant};

/// Type alias for alert variant styling.
pub type AlertVariant = Variant;

/// Type alias for badge variant styling.
pub type BadgeVariant = Variant;

/// An alert box for communicating system status, warnings, or confirmations.
#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    title: Option<String>,
    message: Option<String>,
    children: Vec<Node>,
    variant: AlertVariant,
    icon: Option<Node>,
    dismissible: bool,
    class: String,
}

impl Default for Alert {
    fn default() -> Self {
        Self {
            title: None,
            message: None,
            children: Vec::new(),
            variant: Variant::Primary,
            icon: None,
            dismissible: false,
            class: String::new(),
        }
    }
}

impl Alert {
    /// Create a new alert.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor for an informational alert.
    pub fn info() -> Self {
        Self::new().variant(Variant::Primary)
    }

    /// Convenience constructor for a success alert.
    pub fn success() -> Self {
        Self::new().variant(Variant::Success)
    }

    /// Convenience constructor for a warning alert.
    pub fn warning() -> Self {
        Self::new().variant(Variant::Warning)
    }

    /// Convenience constructor for a danger / error alert.
    pub fn danger() -> Self {
        Self::new().variant(Variant::Danger)
    }

    /// Set an alert title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set a text message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the visual variant.
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set an optional icon node to display alongside the alert message.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Whether to show a dismiss / close button.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
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

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let variant_class = match self.variant {
            Variant::Primary => "rounded-md bg-blue-50 p-4 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
            Variant::Success => "rounded-md bg-green-50 p-4 text-green-800 dark:bg-green-900 dark:text-green-200",
            Variant::Warning => "rounded-md bg-yellow-50 p-4 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
            Variant::Danger => "rounded-md bg-red-50 p-4 text-red-800 dark:bg-red-900 dark:text-red-200",
            Variant::Secondary | Variant::Outline | Variant::Ghost => {
                "rounded-md bg-gray-50 p-4 text-gray-800 dark:bg-gray-800 dark:text-gray-200"
            }
        };

        let full_class = merge_classes(&[variant_class, &self.class]);

        let mut alert_el = Element::new("div")
            .attr("role", "alert")
            .attr("class", full_class);

        let mut inner_flex = Element::new("div").attr("class", "flex items-start gap-3");

        if let Some(icon) = self.icon {
            let icon_wrapper = Element::new("div")
                .attr("class", "flex-shrink-0")
                .child(icon);
            inner_flex = inner_flex.child(Node::Element(icon_wrapper));
        }

        let mut content = Element::new("div").attr("class", "flex-1");

        if let Some(title) = self.title {
            let title_el = Element::new("h4")
                .attr("class", "text-sm font-semibold mb-1")
                .child(Node::text(title));
            content = content.child(Node::Element(title_el));
        }

        if let Some(message) = self.message {
            let msg_el = Element::new("div")
                .attr("class", "text-sm")
                .child(Node::text(message));
            content = content.child(Node::Element(msg_el));
        }

        for child in self.children {
            content = content.child(child);
        }

        inner_flex = inner_flex.child(Node::Element(content));

        if self.dismissible {
            let close_btn = Element::new("button")
                .attr("type", "button")
                .attr("class", "-mx-1.5 -my-1.5 ml-auto inline-flex h-8 w-8 rounded-md p-1.5 hover:bg-black/10 focus:outline-none")
                .attr("aria-label", "Close")
                .child(Node::text("×"));
            inner_flex = inner_flex.child(Node::Element(close_btn));
        }

        alert_el = alert_el.child(Node::Element(inner_flex));
        Node::Element(alert_el)
    }
}

/// Convenience function to construct a new [`Alert`].
pub fn alert() -> Alert {
    Alert::new()
}

impl IntoNode for Alert {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Alert> for Node {
    fn from(a: Alert) -> Self {
        a.render()
    }
}

impl Component for Alert {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A compact status or tag indicator badge.
#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    label: String,
    variant: BadgeVariant,
    size: Size,
    pill: bool,
    dot: bool,
    class: String,
}

impl Badge {
    /// Create a new badge with label text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: Variant::Primary,
            size: Size::Md,
            pill: true,
            dot: false,
            class: String::new(),
        }
    }

    /// Set visual variant.
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set whether the badge has fully rounded pill corners (`rounded-full` vs `rounded-md`).
    pub fn pill(mut self, pill: bool) -> Self {
        self.pill = pill;
        self
    }

    /// Show a small colored status dot before the label.
    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "inline-flex items-center font-medium gap-1.5";

        let shape_class = if self.pill {
            "rounded-full"
        } else {
            "rounded-md"
        };

        let size_class = match self.size {
            Size::Sm => "px-2 py-0.5 text-xs",
            Size::Md => "px-2.5 py-0.5 text-xs",
            Size::Lg => "px-3 py-1 text-sm",
            Size::Xl => "px-3.5 py-1.5 text-sm",
        };

        let (variant_class, dot_color) = match self.variant {
            Variant::Primary => (
                "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300",
                "bg-blue-500",
            ),
            Variant::Secondary => (
                "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300",
                "bg-gray-500",
            ),
            Variant::Outline => (
                "border border-gray-300 text-gray-700 dark:border-gray-600 dark:text-gray-300",
                "bg-gray-400",
            ),
            Variant::Ghost => ("text-gray-600 dark:text-gray-400", "bg-gray-400"),
            Variant::Danger => (
                "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
                "bg-red-500",
            ),
            Variant::Success => (
                "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
                "bg-green-500",
            ),
            Variant::Warning => (
                "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300",
                "bg-yellow-500",
            ),
        };

        let full_class = merge_classes(&[
            base_class,
            shape_class,
            size_class,
            variant_class,
            &self.class,
        ]);

        let mut el = Element::new("span").attr("class", full_class);

        if self.dot {
            let dot_class = format!("h-1.5 w-1.5 rounded-full {}", dot_color);
            let dot_el = Element::new("span")
                .attr("class", dot_class)
                .attr("aria-hidden", "true");
            el = el.child(Node::Element(dot_el));
        }

        el = el.child(Node::text(self.label));
        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Badge`].
pub fn badge(label: impl Into<String>) -> Badge {
    Badge::new(label)
}

impl IntoNode for Badge {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Badge> for Node {
    fn from(b: Badge) -> Self {
        b.render()
    }
}

impl Component for Badge {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An animated spinner loading indicator.
#[derive(Debug, Clone, PartialEq)]
pub struct Loading {
    size: Size,
    label: String,
    class: String,
}

impl Default for Loading {
    fn default() -> Self {
        Self {
            size: Size::Md,
            label: "Loading...".to_string(),
            class: String::new(),
        }
    }
}

impl Loading {
    /// Create a new loading indicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set accessibility label for screen readers.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let size_dim = match self.size {
            Size::Sm => "h-4 w-4",
            Size::Md => "h-6 w-6",
            Size::Lg => "h-8 w-8",
            Size::Xl => "h-12 w-12",
        };

        let svg_class = merge_classes(&[
            "animate-spin text-blue-600 dark:text-blue-400",
            size_dim,
            &self.class,
        ]);

        let circle = Element::new("circle")
            .attr("class", "opacity-25")
            .attr("cx", "12")
            .attr("cy", "12")
            .attr("r", "10")
            .attr("stroke", "currentColor")
            .attr("stroke-width", "4");

        let path = Element::new("path")
            .attr("class", "opacity-75")
            .attr("fill", "currentColor")
            .attr(
                "d",
                "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z",
            );

        let svg = Element::new("svg")
            .attr("class", svg_class)
            .attr("xmlns", "http://www.w3.org/2000/svg")
            .attr("fill", "none")
            .attr("viewBox", "0 0 24 24")
            .child(Node::Element(circle))
            .child(Node::Element(path));

        let sr_only = Element::new("span")
            .attr("class", "sr-only")
            .child(Node::text(self.label));

        let wrapper = Element::new("div")
            .attr("role", "status")
            .attr("class", "inline-flex items-center gap-2")
            .child(Node::Element(svg))
            .child(Node::Element(sr_only));

        Node::Element(wrapper)
    }
}

/// Convenience function to construct a new [`Loading`].
pub fn loading() -> Loading {
    Loading::new()
}

impl IntoNode for Loading {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Loading> for Node {
    fn from(l: Loading) -> Self {
        l.render()
    }
}

impl Component for Loading {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Placeholder skeleton component for loading state content.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Skeleton {
    width: Option<String>,
    height: Option<String>,
    circle: bool,
    class: String,
}

impl Skeleton {
    /// Create a new skeleton.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom width class (e.g. `"w-full"`, `"w-32"`).
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set custom height class (e.g. `"h-4"`, `"h-12"`).
    pub fn height(mut self, height: impl Into<String>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Format skeleton as a circular placeholder (e.g. for avatars).
    pub fn circle(mut self, circle: bool) -> Self {
        self.circle = circle;
        self
    }

    /// Convenience for a text line skeleton placeholder.
    pub fn text() -> Self {
        Self::new().height("h-4").width("w-full")
    }

    /// Convenience for an avatar skeleton placeholder.
    pub fn avatar(size_dim: impl Into<String>) -> Self {
        let dim = size_dim.into();
        Self::new().height(&dim).width(&dim).circle(true)
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "animate-pulse bg-gray-200 dark:bg-gray-700";
        let shape_class = if self.circle {
            "rounded-full"
        } else {
            "rounded-md"
        };
        let w = self.width.as_deref().unwrap_or("w-full");
        let h = self.height.as_deref().unwrap_or("h-4");

        let full_class = merge_classes(&[base_class, shape_class, w, h, &self.class]);

        let el = Element::new("div")
            .attr("aria-hidden", "true")
            .attr("class", full_class);

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Skeleton`].
pub fn skeleton() -> Skeleton {
    Skeleton::new()
}

impl IntoNode for Skeleton {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Skeleton> for Node {
    fn from(s: Skeleton) -> Self {
        s.render()
    }
}

impl Component for Skeleton {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── Functional Component Adapters for RSX / JSX ─────────────────────────────

/// Properties for constructing an `Alert` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlertProps {
    pub title: Option<String>,
    pub message: Option<String>,
    pub variant: Option<String>,
    pub dismissible: bool,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Alert`, enabling `<Alert ...>...</Alert>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Alert(props: AlertProps) -> Node {
    let mut a = Alert::new();
    if let Some(t) = props.title {
        a = a.title(t);
    }
    if let Some(m) = props.message {
        a = a.message(m);
    }
    if let Some(v) = props.variant {
        match v.to_lowercase().as_str() {
            "primary" | "info" => a = a.variant(Variant::Primary),
            "secondary" => a = a.variant(Variant::Secondary),
            "danger" | "error" => a = a.variant(Variant::Danger),
            "success" => a = a.variant(Variant::Success),
            "warning" => a = a.variant(Variant::Warning),
            "ghost" => a = a.variant(Variant::Ghost),
            "outline" => a = a.variant(Variant::Outline),
            _ => {}
        }
    }
    if props.dismissible {
        a = a.dismissible(true);
    }
    if !props.class.is_empty() {
        a = a.class(props.class);
    }
    for child in props.children {
        a = a.child(child);
    }
    a.render()
}

/// Properties for constructing a `Badge` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BadgeProps {
    pub label: Option<String>,
    pub variant: Option<String>,
    pub size: Option<String>,
    pub pill: bool,
    pub dot: bool,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Badge`, enabling `<Badge ...>...</Badge>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Badge(props: BadgeProps) -> Node {
    let mut b = if let Some(label) = props.label {
        Badge::new(label)
    } else {
        Badge::new("")
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
    if props.pill {
        b = b.pill(true);
    }
    if props.dot {
        b = b.dot(true);
    }
    if !props.class.is_empty() {
        b = b.class(props.class);
    }
    let mut node = b.render();
    if !props.children.is_empty() {
        if let Node::Element(ref mut el) = node {
            for child in props.children {
                el.children.push(child);
            }
        }
    }
    node
}

/// A progress bar indicator displaying completion percentage.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBar {
    value: f64,
    max: f64,
    label: Option<String>,
    show_value: bool,
    variant: Variant,
    size: Size,
    animated: bool,
    class: String,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            value: 0.0,
            max: 100.0,
            label: None,
            show_value: false,
            variant: Variant::Primary,
            size: Size::Md,
            animated: false,
            class: String::new(),
        }
    }
}

impl ProgressBar {
    /// Create a new progress bar with a value between 0.0 and 100.0.
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 100.0),
            ..Default::default()
        }
    }

    /// Set the maximum bound (default 100.0).
    pub fn max(mut self, max: f64) -> Self {
        self.max = if max <= 0.0 { 100.0 } else { max };
        self
    }

    /// Set an optional descriptive label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show or hide the percentage text display.
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Set the color variant.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the bar thickness size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Enable pulse or indeterminate animation.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the progress bar into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let percent = ((self.value / self.max) * 100.0).clamp(0.0, 100.0);
        let height_class = match self.size {
            Size::Sm => "h-1.5",
            Size::Md => "h-2.5",
            Size::Lg => "h-4",
            Size::Xl => "h-6",
        };

        let bar_color = match self.variant {
            Variant::Primary => "bg-primary-600 dark:bg-primary-500",
            Variant::Secondary => "bg-gray-600 dark:bg-gray-400",
            Variant::Success => "bg-green-600 dark:bg-green-500",
            Variant::Warning => "bg-yellow-500 dark:bg-yellow-400",
            Variant::Danger => "bg-red-600 dark:bg-red-500",
            Variant::Outline | Variant::Ghost => "bg-primary-500",
        };

        let anim_class = if self.animated { "animate-pulse" } else { "" };

        let mut wrapper =
            Element::new("div").attr("class", merge_classes(&["w-full space-y-1.5", &self.class]));

        // Header: label + percentage
        if self.label.is_some() || self.show_value {
            let mut header = Element::new("div").attr(
                "class",
                "flex items-center justify-between text-xs font-medium",
            );
            if let Some(lbl) = self.label {
                let l_el = Element::new("span")
                    .attr("class", "text-gray-700 dark:text-gray-300")
                    .child(Node::text(lbl));
                header = header.child(Node::Element(l_el));
            }
            if self.show_value {
                let v_el = Element::new("span")
                    .attr("class", "text-gray-500 dark:text-gray-400 font-mono")
                    .child(Node::text(format!("{percent:.0}%")));
                header = header.child(Node::Element(v_el));
            }
            wrapper = wrapper.child(Node::Element(header));
        }

        // Track
        let track_class = format!(
            "w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700 {height_class}"
        );
        let mut track = Element::new("div")
            .attr("class", track_class)
            .attr("role", "progressbar")
            .attr("aria-valuenow", format!("{percent:.0}"))
            .attr("aria-valuemin", "0")
            .attr("aria-valuemax", "100");

        // Fill bar
        let bar_class = format!(
            "h-full rounded-full transition-all duration-500 ease-out {bar_color} {anim_class}"
        );
        let bar = Element::new("div")
            .attr("class", bar_class)
            .attr("style", format!("width: {percent:.1}%;"));

        track = track.child(Node::Element(bar));
        wrapper = wrapper.child(Node::Element(track));

        Node::Element(wrapper)
    }
}

impl IntoNode for ProgressBar {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<ProgressBar> for Node {
    fn from(p: ProgressBar) -> Self {
        p.render()
    }
}

impl Component for ProgressBar {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`ProgressBar`].
pub fn progress_bar(value: f64) -> ProgressBar {
    ProgressBar::new(value)
}

/// Empty state container for blank screens, zero results, or onboarding placeholders.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmptyState {
    title: String,
    description: Option<String>,
    icon: Option<Node>,
    action: Option<Node>,
    secondary_action: Option<Node>,
    class: String,
}

impl EmptyState {
    /// Create a new empty state with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set a descriptive message.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set an illustration or icon node.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Set a primary call-to-action button or link.
    pub fn action(mut self, action: impl IntoNode) -> Self {
        self.action = Some(action.into_node());
        self
    }

    /// Set an optional secondary action.
    pub fn secondary_action(mut self, action: impl IntoNode) -> Self {
        self.secondary_action = Some(action.into_node());
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the empty state into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "text-center py-12 px-6 rounded-2xl border border-dashed border-gray-300 dark:border-gray-700 bg-white/50 dark:bg-gray-800/40";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut root = Element::new("div").attr("class", full_class);

        // Icon
        if let Some(icon) = self.icon {
            let icon_circle = Element::new("div")
                .attr(
                    "class",
                    "mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-gray-100 dark:bg-gray-700/60 text-gray-400 dark:text-gray-300 mb-4",
                )
                .child(icon);
            root = root.child(Node::Element(icon_circle));
        }

        // Title
        let title_el = Element::new("h3")
            .attr(
                "class",
                "text-base font-semibold text-gray-900 dark:text-white",
            )
            .child(Node::text(self.title));
        root = root.child(Node::Element(title_el));

        // Description
        if let Some(desc) = self.description {
            let desc_el = Element::new("p")
                .attr(
                    "class",
                    "mt-1.5 text-sm text-gray-500 dark:text-gray-400 max-w-sm mx-auto",
                )
                .child(Node::text(desc));
            root = root.child(Node::Element(desc_el));
        }

        // Actions
        if self.action.is_some() || self.secondary_action.is_some() {
            let mut actions_el = Element::new("div").attr(
                "class",
                "mt-6 flex flex-wrap items-center justify-center gap-3",
            );
            if let Some(act) = self.action {
                actions_el = actions_el.child(act);
            }
            if let Some(sec) = self.secondary_action {
                actions_el = actions_el.child(sec);
            }
            root = root.child(Node::Element(actions_el));
        }

        Node::Element(root)
    }
}

impl IntoNode for EmptyState {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<EmptyState> for Node {
    fn from(e: EmptyState) -> Self {
        e.render()
    }
}

impl Component for EmptyState {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct an [`EmptyState`].
pub fn empty_state(title: impl Into<String>) -> EmptyState {
    EmptyState::new(title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn alert_renders_with_role_alert() {
        let node = Alert::info()
            .title("Note")
            .message("System will update tonight.")
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<div"));
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("Note"));
        assert!(html.contains("System will update tonight."));
    }

    #[test]
    fn alert_variants_produce_different_classes() {
        let success = Alert::success().message("ok").render();
        let danger = Alert::danger().message("err").render();

        let s_html = render_to_string(&success);
        let d_html = render_to_string(&danger);

        assert!(s_html.contains("bg-green-50"));
        assert!(d_html.contains("bg-red-50"));
    }

    #[test]
    fn badge_renders_classes_and_label() {
        let node = Badge::new("Active")
            .variant(Variant::Success)
            .size(Size::Sm)
            .dot(true)
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<span"));
        assert!(html.contains("Active"));
        assert!(html.contains("bg-green-100"));
        assert!(html.contains("rounded-full"));
        assert!(html.contains("bg-green-500")); // dot
    }

    #[test]
    fn loading_renders_svg_and_sr_only() {
        let node = Loading::new().label("Please wait...").render();
        let html = render_to_string(&node);
        assert!(html.contains("role=\"status\""));
        assert!(html.contains("<svg"));
        assert!(html.contains("animate-spin"));
        assert!(html.contains("<span class=\"sr-only\">Please wait...</span>"));
    }

    #[test]
    fn skeleton_renders_pulse_and_dimensions() {
        let node = Skeleton::new()
            .width("w-48")
            .height("h-8")
            .circle(false)
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("animate-pulse"));
        assert!(html.contains("w-48"));
        assert!(html.contains("h-8"));
        assert!(html.contains("aria-hidden=\"true\""));
    }

    #[test]
    fn progress_bar_renders_aria_and_width() {
        let node = progress_bar(65.0)
            .label("Uploading")
            .show_value(true)
            .variant(Variant::Success)
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("role=\"progressbar\""));
        assert!(html.contains("aria-valuenow=\"65\""));
        assert!(html.contains("width: 65.0%"));
        assert!(html.contains("Uploading"));
        assert!(html.contains("65%"));
    }

    #[test]
    fn empty_state_renders_title_and_description() {
        let node = empty_state("No vehicles found")
            .description("Try adjusting your search filters")
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("No vehicles found"));
        assert!(html.contains("Try adjusting your search filters"));
    }
}
