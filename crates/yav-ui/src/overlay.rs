//! Overlay components: Dialog modal and Dropdown menu.
//!
//! Provides accessible overlays, dialogs, and popover menus.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// An accessible modal dialog component using HTML5 `<dialog>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    id: String,
    open: bool,
    title: Option<String>,
    description: Option<String>,
    children: Vec<Node>,
    footer: Option<Node>,
    class: String,
}

impl Dialog {
    /// Create a new modal dialog with a DOM `id`.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            open: false,
            title: None,
            description: None,
            children: Vec::new(),
            footer: None,
            class: String::new(),
        }
    }

    /// Set whether the dialog is open by default.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set dialog title text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set dialog description / subtitle text.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set dialog footer actions.
    pub fn footer(mut self, footer: impl IntoNode) -> Self {
        self.footer = Some(footer.into_node());
        self
    }

    /// Add a child node to the dialog body.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple children to the dialog body.
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
        let base_class =
            "rounded-lg bg-white p-6 shadow-xl backdrop:bg-black/50 dark:bg-gray-800 dark:text-white max-w-lg w-full m-auto border border-gray-200 dark:border-gray-700";
        let full_class = merge_classes(&[base_class, &self.class]);

        let title_id = format!("{}-title", self.id);
        let desc_id = format!("{}-desc", self.id);

        let mut el = Element::new("dialog")
            .attr("id", &self.id)
            .attr("role", "dialog")
            .attr("aria-modal", "true")
            .attr("class", full_class);

        if self.open {
            el = el.bool_attr("open", true);
        }

        if self.title.is_some() {
            el = el.attr("aria-labelledby", &title_id);
        }
        if self.description.is_some() {
            el = el.attr("aria-describedby", &desc_id);
        }

        // Header section (Title & Description)
        if self.title.is_some() || self.description.is_some() {
            let mut header = Element::new("div").attr("class", "mb-4");

            if let Some(title) = self.title {
                let h3 = Element::new("h3")
                    .attr("id", title_id)
                    .attr(
                        "class",
                        "text-lg font-semibold text-gray-900 dark:text-white",
                    )
                    .child(Node::text(title));
                header = header.child(Node::Element(h3));
            }

            if let Some(desc) = self.description {
                let p = Element::new("p")
                    .attr("id", desc_id)
                    .attr("class", "mt-1 text-sm text-gray-500 dark:text-gray-400")
                    .child(Node::text(desc));
                header = header.child(Node::Element(p));
            }

            el = el.child(Node::Element(header));
        }

        // Body
        if !self.children.is_empty() {
            let mut body = Element::new("div").attr("class", "mb-6");
            for child in self.children {
                body = body.child(child);
            }
            el = el.child(Node::Element(body));
        }

        // Footer
        if let Some(footer) = self.footer {
            let footer_el = Element::new("div")
                .attr("class", "flex items-center justify-end gap-3 pt-3 border-t border-gray-100 dark:border-gray-700")
                .child(footer);
            el = el.child(Node::Element(footer_el));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Dialog`].
pub fn dialog(id: impl Into<String>) -> Dialog {
    Dialog::new(id)
}

impl IntoNode for Dialog {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Dialog> for Node {
    fn from(d: Dialog) -> Self {
        d.render()
    }
}

impl Component for Dialog {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An individual action or item inside a [`Dropdown`].
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownItem {
    label: String,
    href: Option<String>,
    icon: Option<Node>,
    danger: bool,
    disabled: bool,
    class: String,
}

impl DropdownItem {
    /// Create a new dropdown item with label text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
            icon: None,
            danger: false,
            disabled: false,
            class: String::new(),
        }
    }

    /// Set an optional navigation URL, rendering as an `<a>` link.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Add an icon before the label.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Mark this item with destructive / danger styling.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Mark this item as disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "group flex w-full items-center px-4 py-2 text-sm text-left transition-colors gap-2.5";

        let color_class = if self.danger {
            "text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/30"
        } else {
            "text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-200 dark:hover:bg-gray-700 dark:hover:text-white"
        };

        let disabled_class = if self.disabled {
            "opacity-50 pointer-events-none cursor-not-allowed"
        } else {
            ""
        };

        let full_class = merge_classes(&[base_class, color_class, disabled_class, &self.class]);

        let (tag, is_link) = if self.href.is_some() {
            ("a", true)
        } else {
            ("button", false)
        };

        let mut el = Element::new(tag)
            .attr("class", full_class)
            .attr("role", "menuitem");

        if is_link {
            if let Some(href) = &self.href {
                el = el.attr("href", href);
            }
            if self.disabled {
                el = el.attr("aria-disabled", "true").attr("tabindex", "-1");
            }
        } else {
            el = el.attr("type", "button");
            if self.disabled {
                el = el.bool_attr("disabled", true);
            }
        }

        if let Some(icon) = self.icon {
            el = el.child(icon);
        }

        el = el.child(Node::text(self.label));
        Node::Element(el)
    }
}

/// Convenience function to construct a new [`DropdownItem`].
pub fn dropdown_item(label: impl Into<String>) -> DropdownItem {
    DropdownItem::new(label)
}

impl IntoNode for DropdownItem {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<DropdownItem> for Node {
    fn from(item: DropdownItem) -> Self {
        item.render()
    }
}

impl Component for DropdownItem {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A popover dropdown menu triggered by an interactive element.
#[derive(Debug, Clone, PartialEq)]
pub struct Dropdown {
    trigger: Node,
    items: Vec<DropdownItem>,
    align_right: bool,
    id: Option<String>,
    class: String,
}

impl Dropdown {
    /// Create a new dropdown with a trigger node (typically a [`Button`]).
    pub fn new(trigger: impl IntoNode) -> Self {
        Self {
            trigger: trigger.into_node(),
            items: Vec::new(),
            align_right: true,
            id: None,
            class: String::new(),
        }
    }

    /// Add an item to the dropdown.
    pub fn item(mut self, item: DropdownItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items.
    pub fn items(mut self, items: impl IntoIterator<Item = DropdownItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Align the menu panel to the right edge of the trigger (default `true`).
    pub fn align_right(mut self, align_right: bool) -> Self {
        self.align_right = align_right;
        self
    }

    /// Set an optional DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let align_class = if self.align_right {
            "right-0 origin-top-right"
        } else {
            "left-0 origin-top-left"
        };

        let menu_class = merge_classes(&[
            "absolute z-10 mt-2 w-56 rounded-md bg-white py-1 shadow-lg ring-1 ring-black/5 focus:outline-none dark:bg-gray-800 dark:ring-white/10",
            align_class,
            &self.class,
        ]);

        let mut menu = Element::new("div")
            .attr("class", menu_class)
            .attr("role", "menu")
            .attr("aria-orientation", "vertical");

        for item in self.items {
            menu = menu.child(item.render());
        }

        let mut container = Element::new("div")
            .attr("class", "relative inline-block text-left")
            .child(self.trigger)
            .child(Node::Element(menu));

        if let Some(id) = &self.id {
            container = container.attr("id", id);
        }

        Node::Element(container)
    }
}

/// Convenience function to construct a new [`Dropdown`].
pub fn dropdown(trigger: impl IntoNode) -> Dropdown {
    Dropdown::new(trigger)
}

impl IntoNode for Dropdown {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Dropdown> for Node {
    fn from(d: Dropdown) -> Self {
        d.render()
    }
}

/// Placement direction for off-canvas [`Drawer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerPlacement {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}

/// An accessible slide-over drawer / sheet component.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawer {
    id: String,
    open: bool,
    title: Option<String>,
    description: Option<String>,
    placement: DrawerPlacement,
    children: Vec<Node>,
    footer: Option<Node>,
    class: String,
}

impl Drawer {
    /// Create a new drawer with an element `id`.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            open: false,
            title: None,
            description: None,
            placement: DrawerPlacement::Right,
            children: Vec::new(),
            footer: None,
            class: String::new(),
        }
    }

    /// Set whether the drawer is open by default.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the placement / side where the drawer slides in from.
    pub fn placement(mut self, placement: DrawerPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set drawer title text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set drawer description / subtitle text.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set drawer footer actions.
    pub fn footer(mut self, footer: impl IntoNode) -> Self {
        self.footer = Some(footer.into_node());
        self
    }

    /// Add a child node to the drawer body.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let placement_class = match self.placement {
            DrawerPlacement::Right => "right-0 top-0 h-full w-full max-w-md border-l",
            DrawerPlacement::Left => "left-0 top-0 h-full w-full max-w-md border-r",
            DrawerPlacement::Top => "top-0 left-0 w-full h-auto max-h-[80vh] border-b",
            DrawerPlacement::Bottom => "bottom-0 left-0 w-full h-auto max-h-[80vh] border-t",
        };

        let base_class = "fixed z-50 bg-white p-6 shadow-2xl backdrop:bg-black/50 dark:bg-gray-900 dark:text-white border-gray-200 dark:border-gray-800";
        let full_class = merge_classes(&[base_class, placement_class, &self.class]);

        let title_id = format!("{}-drawer-title", self.id);
        let desc_id = format!("{}-drawer-desc", self.id);

        let mut el = Element::new("dialog")
            .attr("id", &self.id)
            .attr("role", "dialog")
            .attr("aria-modal", "true")
            .attr("data-drawer", "true")
            .attr("class", full_class);

        if self.open {
            el = el.bool_attr("open", true);
        }

        if self.title.is_some() {
            el = el.attr("aria-labelledby", &title_id);
        }
        if self.description.is_some() {
            el = el.attr("aria-describedby", &desc_id);
        }

        // Header with close button
        let mut header = Element::new("div").attr("class", "flex items-center justify-between mb-4 pb-3 border-b border-gray-100 dark:border-gray-800");
        let mut title_box = Element::new("div");
        if let Some(title) = self.title {
            title_box = title_box.child(Node::Element(
                Element::new("h3")
                    .attr("id", title_id)
                    .attr(
                        "class",
                        "text-lg font-semibold text-gray-900 dark:text-white",
                    )
                    .child(Node::text(title)),
            ));
        }
        if let Some(desc) = self.description {
            title_box = title_box.child(Node::Element(
                Element::new("p")
                    .attr("id", desc_id)
                    .attr("class", "text-sm text-gray-500 dark:text-gray-400 mt-0.5")
                    .child(Node::text(desc)),
            ));
        }
        header = header.child(Node::Element(title_box));

        let close_btn = Element::new("button")
            .attr("type", "button")
            .attr("class", "rounded-md p-1.5 text-gray-400 hover:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors")
            .attr("onclick", "this.closest('dialog').close()")
            .attr("aria-label", "Fermer")
            .child(Node::raw(r#"<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>"#));
        header = header.child(Node::Element(close_btn));
        el = el.child(Node::Element(header));

        // Body
        let mut body = Element::new("div").attr("class", "flex-1 overflow-y-auto space-y-4");
        for child in self.children {
            body = body.child(child);
        }
        el = el.child(Node::Element(body));

        // Footer
        if let Some(footer) = self.footer {
            let footer_el = Element::new("div")
                .attr("class", "pt-4 mt-6 border-t border-gray-100 dark:border-gray-800 flex items-center justify-end gap-3")
                .child(footer);
            el = el.child(Node::Element(footer_el));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Drawer`].
pub fn drawer(id: impl Into<String>) -> Drawer {
    Drawer::new(id)
}

impl IntoNode for Drawer {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Drawer> for Node {
    fn from(d: Drawer) -> Self {
        d.render()
    }
}

impl Component for Drawer {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── RSX Functional Component Adapters ───────────────────────────────

/// Properties for constructing a [`Dialog`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogProps {
    pub id: String,
    pub open: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub footer: Option<Node>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Dialog`], enabling `<Dialog ...>...</Dialog>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Dialog(props: DialogProps) -> Node {
    let mut d = Dialog::new(props.id)
        .open(props.open)
        .children(props.children);
    if let Some(t) = props.title {
        d = d.title(t);
    }
    if let Some(desc) = props.description {
        d = d.description(desc);
    }
    if let Some(f) = props.footer {
        d = d.footer(f);
    }
    if !props.class.is_empty() {
        d = d.class(props.class);
    }
    d.render()
}

/// Properties for constructing a [`Drawer`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawerProps {
    pub id: String,
    pub open: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub placement: Option<String>,
    pub footer: Option<Node>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Drawer`], enabling `<Drawer ...>...</Drawer>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Drawer(props: DrawerProps) -> Node {
    let mut d = Drawer::new(props.id).open(props.open);
    if let Some(p) = props.placement {
        match p.to_lowercase().as_str() {
            "left" => d = d.placement(DrawerPlacement::Left),
            "right" => d = d.placement(DrawerPlacement::Right),
            "top" => d = d.placement(DrawerPlacement::Top),
            "bottom" => d = d.placement(DrawerPlacement::Bottom),
            _ => {}
        }
    }
    if let Some(t) = props.title {
        d = d.title(t);
    }
    if let Some(desc) = props.description {
        d = d.description(desc);
    }
    if let Some(f) = props.footer {
        d = d.footer(f);
    }
    if !props.class.is_empty() {
        d = d.class(props.class);
    }
    for child in props.children {
        d = d.child(child);
    }
    d.render()
}

/// Properties for constructing a [`Dropdown`] via `rsx!`.
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownProps {
    pub id: Option<String>,
    pub trigger: Node,
    pub items: Vec<DropdownItem>,
    pub align_right: bool,
    pub class: String,
}

impl Default for DropdownProps {
    fn default() -> Self {
        Self {
            id: None,
            trigger: Node::Empty,
            items: Vec::new(),
            align_right: false,
            class: String::new(),
        }
    }
}

/// Functional component adapter for [`Dropdown`], enabling `<Dropdown ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Dropdown(props: DropdownProps) -> Node {
    let mut d = Dropdown::new(props.trigger)
        .items(props.items)
        .align_right(props.align_right);
    if let Some(id) = props.id {
        d = d.id(id);
    }
    if !props.class.is_empty() {
        d = d.class(props.class);
    }
    d.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn dialog_renders_dialog_tag_with_attributes() {
        let node = Dialog::new("my-modal")
            .title("Confirm Action")
            .description("Are you sure you want to proceed?")
            .child(Node::text("Extra modal details"))
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<dialog"));
        assert!(html.contains("id=\"my-modal\""));
        assert!(html.contains("role=\"dialog\""));
        assert!(html.contains("aria-modal=\"true\""));
        assert!(html.contains("aria-labelledby=\"my-modal-title\""));
        assert!(html.contains("Confirm Action"));
        assert!(html.contains("Are you sure you want to proceed?"));
        assert!(html.contains("Extra modal details"));
        assert!(html.ends_with("</dialog>"));
    }

    #[test]
    fn dialog_open_attribute() {
        let open_dialog = Dialog::new("open-modal").open(true).render();
        let closed_dialog = Dialog::new("closed-modal").open(false).render();

        assert!(render_to_string(&open_dialog).contains("open"));
        assert!(!render_to_string(&closed_dialog).contains("open"));
    }

    #[test]
    fn dropdown_renders_trigger_and_menu_items() {
        let trigger = Node::Element(Element::new("button").child(Node::text("Options")));
        let node = Dropdown::new(trigger)
            .item(DropdownItem::new("Edit").href("/edit"))
            .item(DropdownItem::new("Delete").danger(true))
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("Options"));
        assert!(html.contains("role=\"menu\""));
        assert!(html.contains("href=\"/edit\""));
        assert!(html.contains("text-red-700"));
        assert!(html.contains("role=\"menuitem\""));
    }

    #[test]
    fn dropdown_item_link_or_button() {
        let link_item = DropdownItem::new("Link").href("/link").render();
        let btn_item = DropdownItem::new("Btn").disabled(true).render();

        let link_html = render_to_string(&link_item);
        let btn_html = render_to_string(&btn_item);

        assert!(link_html.starts_with("<a"));
        assert!(btn_html.starts_with("<button"));
        assert!(btn_html.contains("disabled"));
    }

    #[test]
    fn drawer_renders_with_placement_and_attributes() {
        let node = Drawer::new("filter-drawer")
            .placement(DrawerPlacement::Left)
            .title("Filtres avancés")
            .description("Affinez votre recherche")
            .open(true)
            .child(Node::text("Options de filtres"))
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("id=\"filter-drawer\""));
        assert!(html.contains("data-drawer=\"true\""));
        assert!(html.contains("Filtres avancés"));
        assert!(html.contains("left-0"));
        assert!(html.contains("open"));
    }
}
