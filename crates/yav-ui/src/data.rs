//! Data display components: Table, Pagination, and Avatar.
//!
//! Provides accessible tabular data structures, pagination controls, and user avatars.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;
use crate::Size;

/// Alignment of content inside a [`TableCell`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// A cell in a table (`<th>` or `<td>`).
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    content: Node,
    is_header: bool,
    align: CellAlign,
    class: String,
}

impl TableCell {
    /// Create a standard body data cell (`<td>`).
    pub fn new(content: impl IntoNode) -> Self {
        Self {
            content: content.into_node(),
            is_header: false,
            align: CellAlign::Left,
            class: String::new(),
        }
    }

    /// Create a header cell (`<th>`).
    pub fn header(content: impl IntoNode) -> Self {
        Self {
            content: content.into_node(),
            is_header: true,
            align: CellAlign::Left,
            class: String::new(),
        }
    }

    /// Set cell content text alignment.
    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let align_class = match self.align {
            CellAlign::Left => "text-left",
            CellAlign::Center => "text-center",
            CellAlign::Right => "text-right",
        };

        if self.is_header {
            let base_class =
                "px-6 py-3 text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400";
            let full_class = merge_classes(&[base_class, align_class, &self.class]);
            let el = Element::new("th")
                .attr("scope", "col")
                .attr("class", full_class)
                .child(self.content);
            Node::Element(el)
        } else {
            let base_class = "px-6 py-4 whitespace-nowrap text-gray-900 dark:text-gray-100";
            let full_class = merge_classes(&[base_class, align_class, &self.class]);
            let el = Element::new("td")
                .attr("class", full_class)
                .child(self.content);
            Node::Element(el)
        }
    }
}

/// Convenience function to construct a [`TableCell`].
pub fn table_cell(content: impl IntoNode) -> TableCell {
    TableCell::new(content)
}

/// Convenience function to construct a header [`TableCell`].
pub fn table_header_cell(content: impl IntoNode) -> TableCell {
    TableCell::header(content)
}

impl IntoNode for TableCell {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<TableCell> for Node {
    fn from(c: TableCell) -> Self {
        c.render()
    }
}

impl Component for TableCell {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A row inside a [`Table`] (`<tr>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableRow {
    cells: Vec<TableCell>,
    class: String,
}

impl TableRow {
    /// Create a new table row.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a cell to this row.
    pub fn cell(mut self, cell: TableCell) -> Self {
        self.cells.push(cell);
        self
    }

    /// Add multiple cells.
    pub fn cells(mut self, cells: impl IntoIterator<Item = TableCell>) -> Self {
        self.cells.extend(cells);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let mut el = Element::new("tr");
        if !self.class.is_empty() {
            el = el.attr("class", &self.class);
        }
        for cell in self.cells {
            el = el.child(cell.render());
        }
        Node::Element(el)
    }
}

/// Convenience function to construct a new [`TableRow`].
pub fn table_row() -> TableRow {
    TableRow::new()
}

impl IntoNode for TableRow {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<TableRow> for Node {
    fn from(r: TableRow) -> Self {
        r.render()
    }
}

impl Component for TableRow {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A responsive, accessible data table component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    headers: Vec<TableCell>,
    rows: Vec<TableRow>,
    striped: bool,
    hoverable: bool,
    compact: bool,
    bordered: bool,
    caption: Option<String>,
    class: String,
}

impl Table {
    /// Create a new table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a header cell.
    pub fn header(mut self, cell: TableCell) -> Self {
        self.headers.push(cell);
        self
    }

    /// Add multiple header titles by string.
    pub fn headers(mut self, headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for h in headers {
            self.headers.push(TableCell::header(Node::text(h.into())));
        }
        self
    }

    /// Add a row of data.
    pub fn row(mut self, row: TableRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Add multiple rows.
    pub fn rows(mut self, rows: impl IntoIterator<Item = TableRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Enable alternating zebra stripes on rows.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Enable highlight on mouse hover over rows.
    pub fn hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }

    /// Enable compact padding for denser data display.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Enable outer border around the table.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set an accessible table `<caption>`.
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "min-w-full divide-y divide-gray-200 dark:divide-gray-700 text-sm";
        let bordered_class = if self.bordered {
            "border border-gray-200 dark:border-gray-700"
        } else {
            ""
        };
        let full_class = merge_classes(&[base_class, bordered_class, &self.class]);

        let mut table_el = Element::new("table").attr("class", full_class);

        if let Some(caption) = self.caption {
            let cap_el = Element::new("caption")
                .attr(
                    "class",
                    "py-2 text-left text-sm text-gray-500 dark:text-gray-400 font-medium",
                )
                .child(Node::text(caption));
            table_el = table_el.child(Node::Element(cap_el));
        }

        // Header
        if !self.headers.is_empty() {
            let mut thead = Element::new("thead").attr("class", "bg-gray-50 dark:bg-gray-800");
            let mut tr = Element::new("tr");
            for header in self.headers {
                tr = tr.child(header.render());
            }
            thead = thead.child(Node::Element(tr));
            table_el = table_el.child(Node::Element(thead));
        }

        // Body
        let mut tbody = Element::new("tbody").attr(
            "class",
            "divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-900",
        );

        for (i, mut row) in self.rows.into_iter().enumerate() {
            let mut row_classes = Vec::new();
            if self.striped && i % 2 == 1 {
                row_classes.push("bg-gray-50 dark:bg-gray-800/50");
            }
            if self.hoverable {
                row_classes.push("hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors");
            }
            if !row.class.is_empty() {
                row_classes.push(&row.class);
            }
            row.class = merge_classes(&row_classes);
            tbody = tbody.child(row.render());
        }

        table_el = table_el.child(Node::Element(tbody));

        let container = Element::new("div")
            .attr("class", "overflow-x-auto w-full")
            .child(Node::Element(table_el));

        Node::Element(container)
    }
}

/// Convenience function to construct a new [`Table`].
pub fn table() -> Table {
    Table::new()
}

impl IntoNode for Table {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Table> for Node {
    fn from(t: Table) -> Self {
        t.render()
    }
}

impl Component for Table {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Accessible pagination bar component.
#[derive(Debug, Clone, PartialEq)]
pub struct Pagination {
    current_page: usize,
    total_pages: usize,
    base_url: String,
    show_edges: bool,
    class: String,
}

impl Pagination {
    /// Create a new pagination bar.
    ///
    /// # Arguments
    /// * `current_page` - 1-based current page index
    /// * `total_pages` - total number of pages
    /// * `base_url` - base URL prefix (e.g. `"/users?page="`)
    pub fn new(current_page: usize, total_pages: usize, base_url: impl Into<String>) -> Self {
        Self {
            current_page: current_page.max(1),
            total_pages: total_pages.max(1),
            base_url: base_url.into(),
            show_edges: false,
            class: String::new(),
        }
    }

    /// Whether to display First and Last edge jump buttons.
    pub fn show_edges(mut self, show_edges: bool) -> Self {
        self.show_edges = show_edges;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    fn page_url(&self, page: usize) -> String {
        format!("{}{}", self.base_url, page)
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let full_class = merge_classes(&["flex items-center justify-center", &self.class]);
        let mut nav = Element::new("nav")
            .attr("aria-label", "Pagination")
            .attr("class", full_class);

        let mut ul = Element::new("ul").attr(
            "class",
            "inline-flex items-center -space-x-px text-sm rounded-md shadow-sm",
        );

        // Previous button
        let prev_page = if self.current_page > 1 {
            self.current_page - 1
        } else {
            1
        };
        let prev_disabled = self.current_page <= 1;

        let prev_class = merge_classes(&[
            "px-3 py-2 leading-tight border border-gray-300 rounded-l-md transition-colors",
            if prev_disabled {
                "text-gray-400 bg-gray-50 cursor-not-allowed pointer-events-none dark:border-gray-700 dark:bg-gray-800 dark:text-gray-600"
            } else {
                "text-gray-500 bg-white hover:bg-gray-100 hover:text-gray-700 dark:bg-gray-800 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
            },
        ]);

        let mut prev_btn = Element::new("a")
            .attr("href", self.page_url(prev_page))
            .attr("class", prev_class)
            .child(Node::text("Previous"));

        if prev_disabled {
            prev_btn = prev_btn
                .attr("aria-disabled", "true")
                .attr("tabindex", "-1");
        }
        ul = ul.child(Node::Element(
            Element::new("li").child(Node::Element(prev_btn)),
        ));

        // Page numbers
        for page in 1..=self.total_pages {
            let is_current = page == self.current_page;
            let page_class = merge_classes(&[
                "px-3 py-2 leading-tight border transition-colors",
                if is_current {
                    "z-10 text-blue-600 bg-blue-50 border-blue-500 font-semibold dark:bg-blue-900/40 dark:border-blue-500 dark:text-blue-300"
                } else {
                    "text-gray-500 bg-white border-gray-300 hover:bg-gray-100 hover:text-gray-700 dark:bg-gray-800 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
                },
            ]);

            let mut a = Element::new("a")
                .attr("href", self.page_url(page))
                .attr("class", page_class)
                .child(Node::text(page.to_string()));

            if is_current {
                a = a.attr("aria-current", "page");
            }

            ul = ul.child(Node::Element(Element::new("li").child(Node::Element(a))));
        }

        // Next button
        let next_page = if self.current_page < self.total_pages {
            self.current_page + 1
        } else {
            self.total_pages
        };
        let next_disabled = self.current_page >= self.total_pages;

        let next_class = merge_classes(&[
            "px-3 py-2 leading-tight border border-gray-300 rounded-r-md transition-colors",
            if next_disabled {
                "text-gray-400 bg-gray-50 cursor-not-allowed pointer-events-none dark:border-gray-700 dark:bg-gray-800 dark:text-gray-600"
            } else {
                "text-gray-500 bg-white hover:bg-gray-100 hover:text-gray-700 dark:bg-gray-800 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
            },
        ]);

        let mut next_btn = Element::new("a")
            .attr("href", self.page_url(next_page))
            .attr("class", next_class)
            .child(Node::text("Next"));

        if next_disabled {
            next_btn = next_btn
                .attr("aria-disabled", "true")
                .attr("tabindex", "-1");
        }
        ul = ul.child(Node::Element(
            Element::new("li").child(Node::Element(next_btn)),
        ));

        nav = nav.child(Node::Element(ul));
        Node::Element(nav)
    }
}

/// Convenience function to construct a new [`Pagination`].
pub fn pagination(
    current_page: usize,
    total_pages: usize,
    base_url: impl Into<String>,
) -> Pagination {
    Pagination::new(current_page, total_pages, base_url)
}

impl IntoNode for Pagination {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Pagination> for Node {
    fn from(p: Pagination) -> Self {
        p.render()
    }
}

impl Component for Pagination {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Online availability status for an [`Avatar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarStatus {
    Online,
    Offline,
    Busy,
    Away,
}

/// User profile picture or initials avatar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Avatar {
    src: Option<String>,
    alt: Option<String>,
    initials: Option<String>,
    size: Size,
    circle: bool,
    status: Option<AvatarStatus>,
    class: String,
}

impl Avatar {
    /// Create a new avatar.
    pub fn new() -> Self {
        Self {
            src: None,
            alt: None,
            initials: None,
            size: Size::Md,
            circle: true,
            status: None,
            class: String::new(),
        }
    }

    /// Set image URL source.
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Set image `alt` text.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Set fallback initials shown when no image is available.
    pub fn initials(mut self, initials: impl Into<String>) -> Self {
        self.initials = Some(initials.into());
        self
    }

    /// Set size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set whether the avatar is circular (`rounded-full` vs `rounded-md`).
    pub fn circle(mut self, circle: bool) -> Self {
        self.circle = circle;
        self
    }

    /// Set optional online status indicator dot.
    pub fn status(mut self, status: AvatarStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let (dim_class, text_class) = match self.size {
            Size::Sm => ("h-8 w-8", "text-xs"),
            Size::Md => ("h-10 w-10", "text-sm"),
            Size::Lg => ("h-14 w-14", "text-base"),
            Size::Xl => ("h-20 w-20", "text-xl"),
        };

        let shape_class = if self.circle {
            "rounded-full"
        } else {
            "rounded-md"
        };

        let container_class = merge_classes(&[
            "relative inline-flex items-center justify-center overflow-hidden bg-gray-100 font-medium text-gray-700 dark:bg-gray-700 dark:text-gray-200 select-none",
            dim_class,
            shape_class,
            text_class,
            &self.class,
        ]);

        let mut container = Element::new("div").attr("class", container_class);

        if let Some(src) = self.src {
            let alt_text = self.alt.unwrap_or_default();
            let img = Element::new("img")
                .attr("src", src)
                .attr("alt", alt_text)
                .attr("class", "h-full w-full object-cover");
            container = container.child(Node::Element(img));
        } else if let Some(initials) = self.initials {
            container = container.child(Node::text(initials));
        } else {
            // Default placeholder silhouette SVG
            let svg = Element::new("svg")
                .attr("class", "h-full w-full text-gray-400")
                .attr("fill", "currentColor")
                .attr("viewBox", "0 0 24 24")
                .child(Node::Element(
                    Element::new("path").attr(
                        "d",
                        "M24 20.993V24H0v-2.996A14.977 14.977 0 0112.004 15c4.904 0 9.26 2.354 11.996 5.993zM16.002 8.999a4 4 0 11-8 0 4 4 0 018 0z",
                    ),
                ));
            container = container.child(Node::Element(svg));
        }

        // Status indicator
        if let Some(status) = self.status {
            let status_color = match status {
                AvatarStatus::Online => "bg-green-500",
                AvatarStatus::Offline => "bg-gray-400",
                AvatarStatus::Busy => "bg-red-500",
                AvatarStatus::Away => "bg-yellow-500",
            };
            let status_class = format!(
                "absolute bottom-0 right-0 block h-2.5 w-2.5 rounded-full ring-2 ring-white dark:ring-gray-800 {}",
                status_color
            );
            let status_el = Element::new("span")
                .attr("class", status_class)
                .attr("aria-hidden", "true");
            container = container.child(Node::Element(status_el));
        }

        Node::Element(container)
    }
}

/// Convenience function to construct a new [`Avatar`].
pub fn avatar() -> Avatar {
    Avatar::new()
}

impl IntoNode for Avatar {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Avatar> for Node {
    fn from(a: Avatar) -> Self {
        a.render()
    }
}

impl Component for Avatar {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── RSX Functional Component Adapters ───────────────────────────────

/// Properties for constructing a [`Table`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableProps {
    pub headers: Vec<String>,
    pub rows: Vec<TableRow>,
    pub striped: bool,
    pub hoverable: bool,
    pub caption: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Table`], enabling `<Table ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Table(props: TableProps) -> Node {
    let mut t = Table::new()
        .headers(props.headers)
        .rows(props.rows)
        .striped(props.striped)
        .hoverable(props.hoverable);
    if let Some(c) = props.caption {
        t = t.caption(c);
    }
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    t.render()
}

/// Properties for constructing a [`Pagination`] via `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct PaginationProps {
    pub current_page: usize,
    pub total_pages: usize,
    pub base_url: String,
    pub show_edges: bool,
    pub class: String,
}

impl Default for PaginationProps {
    fn default() -> Self {
        Self {
            current_page: 1,
            total_pages: 1,
            base_url: "?page=".to_string(),
            show_edges: false,
            class: String::new(),
        }
    }
}

/// Functional component adapter for [`Pagination`], enabling `<Pagination ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Pagination(props: PaginationProps) -> Node {
    let mut p = Pagination::new(props.current_page, props.total_pages, props.base_url)
        .show_edges(props.show_edges);
    if !props.class.is_empty() {
        p = p.class(props.class);
    }
    p.render()
}

/// Properties for constructing an [`Avatar`] via `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct AvatarProps {
    pub src: Option<String>,
    pub alt: Option<String>,
    pub initials: Option<String>,
    pub size: Option<String>,
    pub circle: bool,
    pub status: Option<String>,
    pub class: String,
}

impl Default for AvatarProps {
    fn default() -> Self {
        Self {
            src: None,
            alt: None,
            initials: None,
            size: None,
            circle: true,
            status: None,
            class: String::new(),
        }
    }
}

/// Functional component adapter for [`Avatar`], enabling `<Avatar ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Avatar(props: AvatarProps) -> Node {
    let mut a = Avatar::new().circle(props.circle);
    if let Some(src) = props.src {
        a = a.src(src);
    }
    if let Some(alt) = props.alt {
        a = a.alt(alt);
    }
    if let Some(init) = props.initials {
        a = a.initials(init);
    }
    if let Some(sz) = props.size {
        match sz.to_lowercase().as_str() {
            "sm" => a = a.size(Size::Sm),
            "md" => a = a.size(Size::Md),
            "lg" => a = a.size(Size::Lg),
            "xl" => a = a.size(Size::Xl),
            _ => {}
        }
    }
    if let Some(st) = props.status {
        match st.to_lowercase().as_str() {
            "online" => a = a.status(AvatarStatus::Online),
            "offline" => a = a.status(AvatarStatus::Offline),
            "busy" => a = a.status(AvatarStatus::Busy),
            "away" => a = a.status(AvatarStatus::Away),
            _ => {}
        }
    }
    if !props.class.is_empty() {
        a = a.class(props.class);
    }
    a.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn table_renders_headers_and_rows() {
        let node = Table::new()
            .headers(vec!["Name", "Role", "Email"])
            .row(
                TableRow::new()
                    .cell(TableCell::new("Alice"))
                    .cell(TableCell::new("Engineer"))
                    .cell(TableCell::new("alice@example.com")),
            )
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("<table"));
        assert!(html.contains("<th scope=\"col\""));
        assert!(html.contains("Name"));
        assert!(html.contains("Role"));
        assert!(html.contains("alice@example.com"));
        assert!(html.contains("</table>"));
    }

    #[test]
    fn table_striped_and_hoverable() {
        let node = Table::new()
            .striped(true)
            .hoverable(true)
            .row(TableRow::new().cell(TableCell::new("1")))
            .row(TableRow::new().cell(TableCell::new("2")))
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("bg-gray-50"));
        assert!(html.contains("hover:bg-gray-100"));
    }

    #[test]
    fn pagination_renders_pages_and_aria_current() {
        let node = Pagination::new(2, 5, "/items?page=").render();
        let html = render_to_string(&node);
        assert!(html.contains("aria-label=\"Pagination\""));
        assert!(html.contains("href=\"/items?page=1\""));
        assert!(html.contains("href=\"/items?page=2\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("href=\"/items?page=3\""));
        assert!(html.contains("Next"));
    }

    #[test]
    fn avatar_renders_img_or_initials() {
        let img_avatar = Avatar::new()
            .src("https://example.com/photo.jpg")
            .alt("Jane Doe")
            .render();
        let html_img = render_to_string(&img_avatar);
        assert!(html_img.contains("<img"));
        assert!(html_img.contains("src=\"https://example.com/photo.jpg\""));
        assert!(html_img.contains("alt=\"Jane Doe\""));

        let initials_avatar = Avatar::new()
            .initials("JD")
            .status(AvatarStatus::Online)
            .render();
        let html_initials = render_to_string(&initials_avatar);
        assert!(html_initials.contains("JD"));
        assert!(html_initials.contains("bg-green-500"));
    }
}
