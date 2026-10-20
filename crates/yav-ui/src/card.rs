//! Card component and related container subcomponents.
//!
//! Cards group related information and actions into a distinct visual container.
//! Includes header, title, description, content, and footer subcomponents.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// A card container component.
///
/// # Examples
/// ```
/// use yav_ui::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
/// use yav_core::render_to_string;
///
/// let card = Card::new()
///     .header(
///         CardHeader::new()
///             .title(CardTitle::new("Project Overview"))
///             .description(CardDescription::new("Summary of current progress"))
///     )
///     .child(CardContent::new().child("Main details here"))
///     .footer(CardFooter::new().child("Last updated today"))
///     .render();
///
/// let html = render_to_string(&card);
/// assert!(html.contains("Project Overview"));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Card {
    header: Option<Node>,
    footer: Option<Node>,
    children: Vec<Node>,
    class: String,
}

impl Card {
    /// Create a new, empty card.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an optional header node for the card.
    pub fn header(mut self, header: impl IntoNode) -> Self {
        self.header = Some(header.into_node());
        self
    }

    /// Set an optional footer node for the card.
    pub fn footer(mut self, footer: impl IntoNode) -> Self {
        self.footer = Some(footer.into_node());
        self
    }

    /// Add a child node to the card body.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes to the card body.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Add custom CSS classes to the card container.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the card into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "rounded-lg border border-gray-200 bg-white shadow-sm dark:border-gray-700 dark:bg-gray-800";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new("div").attr("class", full_class);

        if let Some(header) = self.header {
            element = element.child(header);
        }

        for child in self.children {
            element = element.child(child);
        }

        if let Some(footer) = self.footer {
            element = element.child(footer);
        }

        Node::Element(element)
    }
}

/// Convenience function to construct a new [`Card`].
pub fn card() -> Card {
    Card::new()
}

impl IntoNode for Card {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Card> for Node {
    fn from(c: Card) -> Self {
        c.render()
    }
}

impl Component for Card {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Header section within a [`Card`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardHeader {
    title: Option<Node>,
    description: Option<Node>,
    children: Vec<Node>,
    class: String,
}

impl CardHeader {
    /// Create a new card header.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title of the card header.
    pub fn title(mut self, title: impl IntoNode) -> Self {
        self.title = Some(title.into_node());
        self
    }

    /// Set the description of the card header.
    pub fn description(mut self, desc: impl IntoNode) -> Self {
        self.description = Some(desc.into_node());
        self
    }

    /// Add a child node to the header.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple children to the header.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Add custom CSS classes to the card header.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the card header into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "flex flex-col space-y-1.5 p-6";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new("div").attr("class", full_class);

        if let Some(title) = self.title {
            element = element.child(title);
        }

        if let Some(desc) = self.description {
            element = element.child(desc);
        }

        for child in self.children {
            element = element.child(child);
        }

        Node::Element(element)
    }
}

/// Convenience function to construct a new [`CardHeader`].
pub fn card_header() -> CardHeader {
    CardHeader::new()
}

impl IntoNode for CardHeader {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CardHeader> for Node {
    fn from(h: CardHeader) -> Self {
        h.render()
    }
}

impl Component for CardHeader {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Title element for a [`CardHeader`].
#[derive(Debug, Clone, PartialEq)]
pub struct CardTitle {
    text: String,
    level: u8,
    class: String,
}

impl CardTitle {
    /// Create a new card title with heading level 3 (`<h3>`) by default.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: 3,
            class: String::new(),
        }
    }

    /// Set the heading level (`1` for `<h1>`, `2` for `<h2>`, etc.).
    pub fn level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 6);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the card title into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "text-xl font-semibold leading-none tracking-tight text-gray-900 dark:text-gray-100";
        let full_class = merge_classes(&[base_class, &self.class]);
        let tag = format!("h{}", self.level);

        Node::Element(
            Element::new(tag)
                .attr("class", full_class)
                .child(Node::text(self.text)),
        )
    }
}

/// Convenience function to construct a new [`CardTitle`].
pub fn card_title(text: impl Into<String>) -> CardTitle {
    CardTitle::new(text)
}

impl IntoNode for CardTitle {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CardTitle> for Node {
    fn from(t: CardTitle) -> Self {
        t.render()
    }
}

impl Component for CardTitle {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Description text for a [`CardHeader`].
#[derive(Debug, Clone, PartialEq)]
pub struct CardDescription {
    text: String,
    class: String,
}

impl CardDescription {
    /// Create a new card description.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            class: String::new(),
        }
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the description into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "text-sm text-gray-500 dark:text-gray-400";
        let full_class = merge_classes(&[base_class, &self.class]);

        Node::Element(
            Element::new("p")
                .attr("class", full_class)
                .child(Node::text(self.text)),
        )
    }
}

/// Convenience function to construct a new [`CardDescription`].
pub fn card_description(text: impl Into<String>) -> CardDescription {
    CardDescription::new(text)
}

impl IntoNode for CardDescription {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CardDescription> for Node {
    fn from(d: CardDescription) -> Self {
        d.render()
    }
}

impl Component for CardDescription {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Main content container for a [`Card`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardContent {
    children: Vec<Node>,
    class: String,
}

impl CardContent {
    /// Create a new card content container.
    pub fn new() -> Self {
        Self::default()
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

    /// Render the card content container into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "p-6 pt-0";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new("div").attr("class", full_class);
        for child in self.children {
            element = element.child(child);
        }

        Node::Element(element)
    }
}

/// Convenience function to construct a new [`CardContent`].
pub fn card_content() -> CardContent {
    CardContent::new()
}

impl IntoNode for CardContent {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CardContent> for Node {
    fn from(c: CardContent) -> Self {
        c.render()
    }
}

impl Component for CardContent {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Footer container for a [`Card`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardFooter {
    children: Vec<Node>,
    class: String,
}

impl CardFooter {
    /// Create a new card footer.
    pub fn new() -> Self {
        Self::default()
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

    /// Render the card footer into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "flex items-center p-6 pt-0 border-t border-gray-100 dark:border-gray-700";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new("div").attr("class", full_class);
        for child in self.children {
            element = element.child(child);
        }

        Node::Element(element)
    }
}

/// Convenience function to construct a new [`CardFooter`].
pub fn card_footer() -> CardFooter {
    CardFooter::new()
}

impl IntoNode for CardFooter {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CardFooter> for Node {
    fn from(f: CardFooter) -> Self {
        f.render()
    }
}

impl Component for CardFooter {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── Functional Component Adapters for RSX / JSX ─────────────────────────────

/// Properties for constructing a `Card` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardProps {
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Card`, enabling `<Card>...</Card>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Card(props: CardProps) -> Node {
    let mut c = Card::new();
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    for child in props.children {
        c = c.child(child);
    }
    c.render()
}

/// Properties for constructing a `CardHeader` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardHeaderProps {
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `CardHeader`, enabling `<CardHeader>...</CardHeader>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CardHeader(props: CardHeaderProps) -> Node {
    let mut h = CardHeader::new();
    if !props.class.is_empty() {
        h = h.class(props.class);
    }
    for child in props.children {
        h = h.child(child);
    }
    h.render()
}

/// Properties for constructing a `CardTitle` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardTitleProps {
    pub text: Option<String>,
    pub level: Option<u8>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `CardTitle`, enabling `<CardTitle>...</CardTitle>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CardTitle(props: CardTitleProps) -> Node {
    let mut t = if let Some(txt) = props.text {
        CardTitle::new(txt)
    } else {
        CardTitle::new("")
    };
    if let Some(lvl) = props.level {
        t = t.level(lvl);
    }
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    let mut node = t.render();
    if !props.children.is_empty() {
        if let Node::Element(ref mut el) = node {
            for child in props.children {
                el.children.push(child);
            }
        }
    }
    node
}

/// Properties for constructing a `CardDescription` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardDescriptionProps {
    pub text: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `CardDescription`, enabling `<CardDescription>...</CardDescription>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CardDescription(props: CardDescriptionProps) -> Node {
    let mut d = if let Some(txt) = props.text {
        CardDescription::new(txt)
    } else {
        CardDescription::new("")
    };
    if !props.class.is_empty() {
        d = d.class(props.class);
    }
    let mut node = d.render();
    if !props.children.is_empty() {
        if let Node::Element(ref mut el) = node {
            for child in props.children {
                el.children.push(child);
            }
        }
    }
    node
}

/// Properties for constructing a `CardContent` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardContentProps {
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `CardContent`, enabling `<CardContent>...</CardContent>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CardContent(props: CardContentProps) -> Node {
    let mut c = CardContent::new();
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    for child in props.children {
        c = c.child(child);
    }
    c.render()
}

/// Properties for constructing a `CardFooter` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardFooterProps {
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `CardFooter`, enabling `<CardFooter>...</CardFooter>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CardFooter(props: CardFooterProps) -> Node {
    let mut f = CardFooter::new();
    if !props.class.is_empty() {
        f = f.class(props.class);
    }
    for child in props.children {
        f = f.child(child);
    }
    f.render()
}

/// The direction of a metric trend indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatTrendDirection {
    Up,
    Down,
    Neutral,
}

/// A metric trend delta with direction, percentage value, and comparison period.
#[derive(Debug, Clone, PartialEq)]
pub struct StatTrend {
    pub value: String,
    pub label: Option<String>,
    pub direction: StatTrendDirection,
}

/// A KPI and metric statistic card for dashboards.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatCard {
    pub title: String,
    pub value: String,
    pub description: Option<String>,
    pub trend: Option<StatTrend>,
    pub icon: Option<Node>,
    pub class: String,
}

impl StatCard {
    /// Create a new stat card with a title and display value.
    pub fn new(title: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            ..Default::default()
        }
    }

    /// Set an optional description or footnote.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set an upward/positive trend.
    pub fn trend_up(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.trend = Some(StatTrend {
            value: value.into(),
            label: Some(label.into()),
            direction: StatTrendDirection::Up,
        });
        self
    }

    /// Set a downward/negative trend.
    pub fn trend_down(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.trend = Some(StatTrend {
            value: value.into(),
            label: Some(label.into()),
            direction: StatTrendDirection::Down,
        });
        self
    }

    /// Set a neutral trend.
    pub fn trend_neutral(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.trend = Some(StatTrend {
            value: value.into(),
            label: Some(label.into()),
            direction: StatTrendDirection::Neutral,
        });
        self
    }

    /// Set an optional icon node displayed in the top corner.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the stat card into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "rounded-xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-700 dark:bg-gray-800 transition-all hover:shadow-md";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut card_el = Element::new("div").attr("class", full_class);

        // Top Row: title + icon
        let mut top_row = Element::new("div").attr("class", "flex items-center justify-between");
        let title_el = Element::new("p")
            .attr(
                "class",
                "text-sm font-medium text-gray-500 dark:text-gray-400 truncate",
            )
            .child(Node::text(self.title));
        top_row = top_row.child(Node::Element(title_el));

        if let Some(icon) = self.icon {
            let icon_wrapper = Element::new("div")
                .attr(
                    "class",
                    "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary-50 text-primary-600 dark:bg-primary-950/60 dark:text-primary-400",
                )
                .child(icon);
            top_row = top_row.child(Node::Element(icon_wrapper));
        }
        card_el = card_el.child(Node::Element(top_row));

        // Value
        let val_el = Element::new("p")
            .attr(
                "class",
                "mt-2 text-3xl font-bold tracking-tight text-gray-900 dark:text-white font-mono",
            )
            .child(Node::text(self.value));
        card_el = card_el.child(Node::Element(val_el));

        // Bottom: Trend or Description
        if let Some(trend) = self.trend {
            let (badge_bg, badge_text, arrow_svg) = match trend.direction {
                StatTrendDirection::Up => (
                    "bg-green-50 dark:bg-green-950/50",
                    "text-green-700 dark:text-green-400",
                    r#"<svg class="h-3.5 w-3.5 self-center shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M5.293 9.707a1 1 0 010-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 01-1.414 1.414L11 7.414V15a1 1 0 11-2 0V7.414L6.707 9.707a1 1 0 01-1.414 0z" clip-rule="evenodd"/></svg>"#,
                ),
                StatTrendDirection::Down => (
                    "bg-red-50 dark:bg-red-950/50",
                    "text-red-700 dark:text-red-400",
                    r#"<svg class="h-3.5 w-3.5 self-center shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M14.707 10.293a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 111.414-1.414L9 12.586V5a1 1 0 012 0v7.586l2.293-2.293a1 1 0 011.414 0z" clip-rule="evenodd"/></svg>"#,
                ),
                StatTrendDirection::Neutral => (
                    "bg-gray-100 dark:bg-gray-800",
                    "text-gray-700 dark:text-gray-300",
                    r#"<svg class="h-3.5 w-3.5 self-center shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M3 10a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clip-rule="evenodd"/></svg>"#,
                ),
            };

            let mut trend_row = Element::new("div").attr("class", "mt-3 flex items-center text-xs");
            let badge = Element::new("span")
                .attr(
                    "class",
                    format!("inline-flex items-center gap-0.5 rounded px-1.5 py-0.5 font-medium {badge_bg} {badge_text}"),
                )
                .child(Node::raw(arrow_svg))
                .child(Node::text(trend.value));
            trend_row = trend_row.child(Node::Element(badge));

            if let Some(lbl) = trend.label {
                let lbl_el = Element::new("span")
                    .attr("class", "ml-2 text-gray-500 dark:text-gray-400")
                    .child(Node::text(lbl));
                trend_row = trend_row.child(Node::Element(lbl_el));
            }

            card_el = card_el.child(Node::Element(trend_row));
        } else if let Some(desc) = self.description {
            let desc_el = Element::new("p")
                .attr("class", "mt-2 text-xs text-gray-500 dark:text-gray-400")
                .child(Node::text(desc));
            card_el = card_el.child(Node::Element(desc_el));
        }

        Node::Element(card_el)
    }
}

impl IntoNode for StatCard {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<StatCard> for Node {
    fn from(s: StatCard) -> Self {
        s.render()
    }
}

impl Component for StatCard {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to create a new [`StatCard`].
pub fn stat_card(title: impl Into<String>, value: impl Into<String>) -> StatCard {
    StatCard::new(title, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn card_renders_basic() {
        let node = Card::new().render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<div"));
        assert!(html.contains("rounded-lg border border-gray-200"));
        assert!(html.ends_with("</div>"));
    }

    #[test]
    fn card_renders_with_children() {
        let node = Card::new()
            .child(Node::text("First child"))
            .child(Node::text("Second child"))
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("First child"));
        assert!(html.contains("Second child"));
    }

    #[test]
    fn card_renders_header_content_footer() {
        let node = Card::new()
            .header(
                CardHeader::new()
                    .title(CardTitle::new("My Card"))
                    .description(CardDescription::new("A description of the card")),
            )
            .child(CardContent::new().child(Node::text("Body content")))
            .footer(CardFooter::new().child(Node::text("Action button")))
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("<h3"));
        assert!(html.contains("My Card</h3>"));
        assert!(html.contains("<p class=\"text-sm text-gray-500"));
        assert!(html.contains("A description of the card"));
        assert!(html.contains("Body content"));
        assert!(html.contains("Action button"));
    }

    #[test]
    fn card_title_heading_levels() {
        let h2 = CardTitle::new("Section Title").level(2).render();
        let html2 = render_to_string(&h2);
        assert!(html2.starts_with("<h2"));
        assert!(html2.ends_with("</h2>"));

        let h4 = CardTitle::new("Sub Title").level(4).render();
        let html4 = render_to_string(&h4);
        assert!(html4.starts_with("<h4"));
        assert!(html4.ends_with("</h4>"));
    }

    #[test]
    fn card_with_custom_classes() {
        let node = Card::new().class("custom-card shadow-xl").render();
        let html = render_to_string(&node);
        assert!(html.contains("custom-card shadow-xl"));
    }

    #[test]
    fn stat_card_renders_with_trend() {
        let node = stat_card("Total Revenue", "$1,280,000")
            .trend_up("+14.2%", "vs last month")
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("Total Revenue"));
        assert!(html.contains("$1,280,000"));
        assert!(html.contains("+14.2%"));
        assert!(html.contains("vs last month"));
        assert!(html.contains("text-green-700"));
    }
}
