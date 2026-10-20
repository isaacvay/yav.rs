//! Command palette and searchable menu component.
//!
//! Provides a keyboard-driven command menu (`CommandMenu`, `CommandGroup`, `CommandItem`)
//! for fast navigation, quick actions, and search palettes.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// An individual action or navigational item within a [`CommandGroup`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommandItem {
    pub id: Option<String>,
    pub label: String,
    pub icon: Option<Node>,
    pub shortcut: Option<String>,
    pub href: Option<String>,
    pub action: Option<String>,
    pub disabled: bool,
    pub class: String,
}

impl CommandItem {
    /// Create a new command item with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    /// Set an optional unique identifier.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set an optional SVG or text icon.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Set a keyboard shortcut label (e.g. `"⌘P"`, `"Ctrl+S"`).
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set a destination URL for navigation.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Set an action identifier or JavaScript handler.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Disable the item.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the item into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let tag = if self.href.is_some() { "a" } else { "button" };
        let base_class = if self.disabled {
            "flex w-full items-center justify-between rounded-md px-3 py-2 text-sm text-gray-400 cursor-not-allowed opacity-50 dark:text-gray-500"
        } else {
            "flex w-full items-center justify-between rounded-md px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 hover:text-gray-900 focus:bg-gray-100 focus:outline-none dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white transition-colors cursor-pointer"
        };
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new(tag)
            .attr("class", full_class)
            .attr("role", "menuitem")
            .attr("data-command-item", "true");

        if let Some(id) = self.id {
            element = element.attr("id", id);
        }

        if let Some(href) = self.href {
            if !self.disabled {
                element = element.attr("href", href);
            }
        } else {
            element = element.attr("type", "button");
        }

        if let Some(action) = self.action {
            element = element.attr("data-action", action);
        }

        if self.disabled {
            element = element.attr("aria-disabled", "true");
        }

        // Left side: icon + label
        let mut left = Element::new("div").attr("class", "flex items-center space-x-2.5 truncate");
        if let Some(icon) = self.icon {
            left = left.child(icon);
        }
        left = left.child(Node::text(self.label));
        element = element.child(Node::Element(left));

        // Right side: shortcut
        if let Some(sc) = self.shortcut {
            let kbd = Element::new("kbd")
                .attr(
                    "class",
                    "inline-flex items-center rounded border border-gray-200 bg-gray-100 px-1.5 py-0.5 text-xs font-mono text-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400",
                )
                .child(Node::text(sc));
            element = element.child(Node::Element(kbd));
        }

        Node::Element(element)
    }
}

impl IntoNode for CommandItem {
    fn into_node(self) -> Node {
        self.render()
    }
}

/// A logical group of [`CommandItem`]s with an optional section heading.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommandGroup {
    pub title: Option<String>,
    pub items: Vec<CommandItem>,
    pub class: String,
}

impl CommandGroup {
    /// Create a new command group with an optional heading title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            items: Vec::new(),
            class: String::new(),
        }
    }

    /// Add an item to the group.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items to the group.
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Add custom classes to the group container.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the group into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "py-1.5";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut group_el = Element::new("div")
            .attr("class", full_class)
            .attr("role", "group")
            .attr("data-command-group", "true");

        if let Some(title) = self.title {
            let heading = Element::new("div")
                .attr(
                    "class",
                    "px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400",
                )
                .child(Node::text(title));
            group_el = group_el.child(Node::Element(heading));
        }

        let mut list_el = Element::new("div").attr("class", "space-y-0.5 px-1");
        for item in self.items {
            list_el = list_el.child(item.render());
        }

        group_el = group_el.child(Node::Element(list_el));
        Node::Element(group_el)
    }
}

impl IntoNode for CommandGroup {
    fn into_node(self) -> Node {
        self.render()
    }
}

/// Command palette component offering modal search and quick keyboard actions.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandMenu {
    pub id: String,
    pub placeholder: String,
    pub shortcut: Option<String>,
    pub groups: Vec<CommandGroup>,
    pub open: bool,
    pub class: String,
}

impl Default for CommandMenu {
    fn default() -> Self {
        Self {
            id: "command-menu".to_string(),
            placeholder: "Type a command or search...".to_string(),
            shortcut: Some("⌘K".to_string()),
            groups: Vec::new(),
            open: false,
            class: String::new(),
        }
    }
}

impl CommandMenu {
    /// Create a new command menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the menu container ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the search input placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the displayed shortcut label (e.g. `"⌘K"` or `"Ctrl+K"`).
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set whether the menu is initially visible.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Add a command group.
    pub fn group(mut self, group: CommandGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Add multiple command groups.
    pub fn groups(mut self, groups: impl IntoIterator<Item = CommandGroup>) -> Self {
        self.groups.extend(groups);
        self
    }

    /// Add custom classes to the modal card.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the command palette dialog.
    pub fn render(self) -> Node {
        let root_hidden = if self.open { "" } else { "hidden " };
        let root_class = format!("{root_hidden}fixed inset-0 z-50 overflow-y-auto p-4 sm:p-6 md:p-20 flex items-start justify-center");

        let mut root = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .attr("role", "dialog")
            .attr("aria-modal", "true")
            .attr("data-command-palette", "true");

        // Backdrop
        let backdrop = Element::new("div")
            .attr(
                "class",
                "fixed inset-0 bg-gray-900/60 backdrop-blur-sm transition-opacity",
            )
            .attr("data-command-backdrop", "true")
            .attr("aria-hidden", "true");
        root = root.child(Node::Element(backdrop));

        // Dialog Panel
        let card_base = "relative w-full max-w-xl transform rounded-xl bg-white shadow-2xl ring-1 ring-black/5 transition-all dark:bg-gray-900 dark:ring-white/10 overflow-hidden divide-y divide-gray-200 dark:divide-gray-800";
        let card_class = merge_classes(&[card_base, &self.class]);
        let mut card = Element::new("div").attr("class", card_class);

        // Search Header
        let mut header = Element::new("div").attr("class", "relative flex items-center px-4");
        // Search icon
        let search_icon = Node::raw(
            r#"<svg class="h-5 w-5 text-gray-400" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true"><path fill-rule="evenodd" d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z" clip-rule="evenodd" /></svg>"#,
        );
        header = header.child(search_icon);

        // Input
        let input = Element::new("input")
            .attr("type", "text")
            .attr("class", "h-12 w-full border-0 bg-transparent pl-3 pr-4 text-gray-900 placeholder:text-gray-400 focus:outline-none focus:ring-0 sm:text-sm dark:text-white")
            .attr("placeholder", self.placeholder)
            .attr("role", "combobox")
            .attr("aria-expanded", "true")
            .attr("data-command-input", "true");
        header = header.child(Node::Element(input));

        if let Some(sc) = self.shortcut {
            let kbd = Element::new("kbd")
                .attr("class", "hidden sm:inline-flex items-center rounded border border-gray-200 bg-gray-100 px-2 py-0.5 text-xs font-mono text-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400")
                .child(Node::text(sc));
            header = header.child(Node::Element(kbd));
        }
        card = card.child(Node::Element(header));

        // Results Container
        let mut results = Element::new("div")
            .attr("class", "max-h-80 scroll-py-2 overflow-y-auto p-2")
            .attr("role", "listbox")
            .attr("data-command-results", "true");

        for group in self.groups {
            results = results.child(group.render());
        }
        card = card.child(Node::Element(results));

        // Footer / Help
        let footer = Element::new("div")
            .attr("class", "flex flex-wrap items-center justify-between px-4 py-2.5 text-xs text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800/50")
            .child(Node::raw(r#"<span>Use <kbd class="font-mono font-semibold">↑</kbd> <kbd class="font-mono font-semibold">↓</kbd> to navigate, <kbd class="font-mono font-semibold">↵</kbd> to select, <kbd class="font-mono font-semibold">ESC</kbd> to close</span>"#));
        card = card.child(Node::Element(footer));

        root = root.child(Node::Element(card));
        Node::Element(root)
    }
}

impl IntoNode for CommandMenu {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CommandMenu> for Node {
    fn from(m: CommandMenu) -> Self {
        m.render()
    }
}

impl Component for CommandMenu {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to create a new [`CommandMenu`].
pub fn command_menu() -> CommandMenu {
    CommandMenu::new()
}

/// Helper script that binds `Cmd+K` / `Ctrl+K`, search filtering, and keyboard navigation to command palettes.
pub fn command_palette_script() -> Node {
    let script = r#"<script>
(function() {
  function initCommandPalette() {
    const palette = document.querySelector('[data-command-palette]');
    if (!palette) return;

    const input = palette.querySelector('[data-command-input]');
    const backdrop = palette.querySelector('[data-command-backdrop]');
    const items = () => Array.from(palette.querySelectorAll('[data-command-item]'));

    function open() {
      palette.classList.remove('hidden');
      if (input) {
        input.value = '';
        input.focus();
        filter('');
      }
    }

    function close() {
      palette.classList.add('hidden');
    }

    function filter(query) {
      const q = query.toLowerCase().trim();
      palette.querySelectorAll('[data-command-group]').forEach(group => {
        let anyVisible = false;
        group.querySelectorAll('[data-command-item]').forEach(item => {
          const text = item.textContent.toLowerCase();
          const match = !q || text.includes(q);
          item.style.display = match ? '' : 'none';
          if (match) anyVisible = true;
        });
        group.style.display = anyVisible ? '' : 'none';
      });
    }

    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (palette.classList.contains('hidden')) {
          open();
        } else {
          close();
        }
      } else if (e.key === 'Escape' && !palette.classList.contains('hidden')) {
        e.preventDefault();
        close();
      }
    });

    if (backdrop) backdrop.addEventListener('click', close);
    if (input) {
      input.addEventListener('input', (e) => filter(e.target.value));
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCommandPalette);
  } else {
    initCommandPalette();
  }
})();
</script>"#;
    Node::raw(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn command_menu_renders_with_items_and_groups() {
        let menu = command_menu()
            .id("quick-search")
            .placeholder("Search vehicles...")
            .group(
                CommandGroup::new("Actions")
                    .item(
                        CommandItem::new("View Catalogue")
                            .href("/catalogue")
                            .shortcut("⌘C"),
                    )
                    .item(CommandItem::new("Book Test Drive").action("open-booking")),
            )
            .render();

        let html = render_to_string(&menu);
        assert!(html.contains("id=\"quick-search\""));
        assert!(html.contains("Search vehicles..."));
        assert!(html.contains("Actions"));
        assert!(html.contains("View Catalogue"));
        assert!(html.contains("href=\"/catalogue\""));
        assert!(html.contains("Book Test Drive"));
        assert!(html.contains("⌘C"));
    }

    #[test]
    fn command_item_disabled_state() {
        let item = CommandItem::new("Unavailable").disabled(true).render();
        let html = render_to_string(&item);
        assert!(html.contains("cursor-not-allowed"));
        assert!(html.contains("aria-disabled=\"true\""));
    }
}
