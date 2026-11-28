//! Navigation components: Link, Navbar, Sidebar, Breadcrumbs, and Tabs.
//!
//! Provides accessible navigation structures for site headers, sidebars, and hierarchical location paths.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// An accessible hyperlink component.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    href: String,
    label: Option<String>,
    children: Vec<Node>,
    external: bool,
    prefetch: bool,
    underline: bool,
    class: String,
}

impl Link {
    /// Create a new link pointing to `href`.
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            label: None,
            children: Vec::new(),
            external: false,
            prefetch: true,
            underline: false,
            class: String::new(),
        }
    }

    /// Set a text label for the link.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Mark as an external link (adds `target="_blank"` and `rel="noopener noreferrer"`).
    pub fn external(mut self, external: bool) -> Self {
        self.external = external;
        if external {
            self.prefetch = false;
        }
        self
    }

    /// Configure whether to prefetch this route in the background (defaults to true for internal links).
    pub fn prefetch(mut self, prefetch: bool) -> Self {
        self.prefetch = prefetch;
        self
    }

    /// Whether to force underline decoration.
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
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

    /// Add multiple children.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "text-blue-600 hover:text-blue-800 transition-colors dark:text-blue-400 dark:hover:text-blue-300";
        let underline_class = if self.underline {
            "underline"
        } else {
            "hover:underline"
        };
        let full_class = merge_classes(&[base_class, underline_class, &self.class]);

        let mut el = Element::new("a")
            .attr("href", &self.href)
            .attr("class", full_class);

        if self.external {
            el = el
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer");
        } else {
            el = el.attr("data-yav-link", "true");
            if self.prefetch {
                el = el.attr("data-prefetch", "true");
            }
        }

        if let Some(lbl) = self.label {
            el = el.child(Node::text(lbl));
        }

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Properties for the [`Link`] functional RSX component.
#[derive(Debug, Clone, Default)]
pub struct LinkProps {
    pub href: String,
    pub label: Option<String>,
    pub external: bool,
    pub prefetch: Option<bool>,
    pub underline: bool,
    pub class: Option<String>,
    pub children: Vec<Node>,
}

/// Functional RSX component: `<Link href="..." prefetch=true> ... </Link>`.
#[allow(non_snake_case)]
pub fn Link(props: LinkProps) -> Node {
    let prefetch = props.prefetch.unwrap_or(!props.external);
    let mut l = Link::new(props.href)
        .external(props.external)
        .prefetch(prefetch)
        .underline(props.underline);

    if let Some(lbl) = props.label {
        l = l.label(lbl);
    }
    if let Some(c) = props.class {
        l = l.class(c);
    }
    l.children(props.children).render()
}

/// Injects a lightweight client-side progressive navigation runtime.
///
/// Features:
/// - Prefetches routes into an in-memory cache via `IntersectionObserver` or mouse hover.
/// - Intercepts internal link clicks and swaps `<main>` or `<body>` DOM nodes instantly.
/// - Updates URL history (`history.pushState`) with scroll restoration and title updates.
/// - Fallback: Completely zero-overhead standard HTML navigation when JS is disabled.
pub fn client_router_script() -> Node {
    let script = r#"(function(){
if (window.__YAV_ROUTER_INSTALLED__) return;
window.__YAV_ROUTER_INSTALLED__ = true;
const cache = new Map();
const MAX_CACHE_ENTRIES = 50;

function currentUrl() {
  return window.location.pathname + window.location.search;
}

function isInternalUrl(href) {
  if (!href || href.startsWith('#') || href.startsWith('//')) return false;
  try {
    const parsed = new URL(href, window.location.origin);
    return parsed.origin === window.location.origin &&
      (parsed.protocol === 'http:' || parsed.protocol === 'https:');
  } catch {
    return false;
  }
}

function setCache(url, html) {
  if (cache.size >= MAX_CACHE_ENTRIES && !cache.has(url)) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
  cache.set(url, html);
}

function prefetch(url) {
  if (!isInternalUrl(url) || cache.has(url) || url === currentUrl()) return;
  fetch(url, { headers: { 'X-Yav-Prefetch': 'true' } })
    .then(r => r.ok ? r.text() : null)
    .then(html => { if (html) setCache(url, html); })
    .catch(() => {});
}

const observer = 'IntersectionObserver' in window ? new IntersectionObserver((entries) => {
  entries.forEach(e => {
    if (e.isIntersecting) {
      const href = e.target.getAttribute('href');
      if (isInternalUrl(href)) prefetch(href);
      observer.unobserve(e.target);
    }
  });
}) : null;

function scanLinks() {
  if (!observer) return;
  document.querySelectorAll('a[data-prefetch="true"]').forEach(el => observer.observe(el));
}

async function navigate(url, push = true) {
  if (!isInternalUrl(url)) {
    window.location.href = url;
    return;
  }
  let html = cache.get(url);
  if (!html) {
    try {
      const res = await fetch(url);
      if (!res.ok) { window.location.href = url; return; }
      html = await res.text();
      setCache(url, html);
    } catch {
      window.location.href = url;
      return;
    }
  }

  const parser = new DOMParser();
  const doc = parser.parseFromString(html, 'text/html');

  const currentMain = document.querySelector('main');
  const newMain = doc.querySelector('main');

  const applyUpdate = () => {
    if (doc.title) document.title = doc.title;
    if (currentMain && newMain) {
      currentMain.replaceWith(newMain);
    } else {
      document.body.innerHTML = doc.body.innerHTML;
    }
  };

  if (document.startViewTransition) {
    document.startViewTransition(applyUpdate);
  } else {
    applyUpdate();
  }

  if (push) window.history.pushState({}, '', url);
  window.scrollTo(0, 0);
  scanLinks();
  window.dispatchEvent(new CustomEvent('yav:navigate', { detail: { url } }));
}

document.addEventListener('submit', () => cache.clear());

document.addEventListener('click', (e) => {
  const link = e.target.closest('a[data-yav-link="true"]');
  if (!link) return;
  const href = link.getAttribute('href');
  if (!isInternalUrl(href) || link.target === '_blank' || link.hasAttribute('download')) return;
  if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button !== 0) return;
  e.preventDefault();
  navigate(href);
});

document.addEventListener('mouseover', (e) => {
  const link = e.target.closest('a[data-yav-link="true"]');
  if (link) {
    const href = link.getAttribute('href');
    if (isInternalUrl(href)) prefetch(href);
  }
});

window.addEventListener('popstate', () => navigate(currentUrl(), false));

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', scanLinks);
} else {
  scanLinks();
}
})();"#;

    let mut el = Element::new("script");
    el = el.child(Node::raw(script));
    Node::Element(el)
}

/// Nonce-aware variant of [`client_router_script`] for strict CSP.
pub fn client_router_script_with_nonce(nonce: &str) -> Node {
    match client_router_script() {
        Node::Element(el) => Node::Element(el.attr("nonce", nonce)),
        other => other,
    }
}

/// Convenience function to construct a new [`Link`].
pub fn link(href: impl Into<String>) -> Link {
    Link::new(href)
}

impl IntoNode for Link {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Link> for Node {
    fn from(l: Link) -> Self {
        l.render()
    }
}

impl Component for Link {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Brand logo/title element for a [`Navbar`].
#[derive(Debug, Clone, PartialEq)]
pub struct NavbarBrand {
    href: String,
    label: Option<String>,
    children: Vec<Node>,
    class: String,
}

impl NavbarBrand {
    /// Create a new brand linking to `href` (defaults to `"/"`).
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            label: None,
            children: Vec::new(),
            class: String::new(),
        }
    }

    /// Set brand label text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a child node (e.g. logo icon).
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
        let base_class =
            "text-lg font-bold text-gray-900 dark:text-white flex items-center gap-2 hover:opacity-90";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut el = Element::new("a")
            .attr("href", &self.href)
            .attr("class", full_class);

        if let Some(lbl) = self.label {
            el = el.child(Node::text(lbl));
        }

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`NavbarBrand`].
pub fn navbar_brand(href: impl Into<String>) -> NavbarBrand {
    NavbarBrand::new(href)
}

impl IntoNode for NavbarBrand {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<NavbarBrand> for Node {
    fn from(b: NavbarBrand) -> Self {
        b.render()
    }
}

impl Component for NavbarBrand {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Individual item in a [`Navbar`].
#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    label: String,
    href: String,
    active: bool,
    class: String,
}

impl NavItem {
    /// Create a new navigation item with label and target `href`.
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
            active: false,
            class: String::new(),
        }
    }

    /// Mark this item as the active/current route.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let state_class = if self.active {
            "text-blue-600 font-semibold dark:text-blue-400"
        } else {
            "text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-white"
        };
        let full_class = merge_classes(&[
            "text-sm font-medium transition-colors px-3 py-2 rounded-md",
            state_class,
            &self.class,
        ]);

        let mut el = Element::new("a")
            .attr("href", &self.href)
            .attr("class", full_class)
            .child(Node::text(self.label));

        if self.active {
            el = el.attr("aria-current", "page");
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`NavItem`].
pub fn nav_item(label: impl Into<String>, href: impl Into<String>) -> NavItem {
    NavItem::new(label, href)
}

impl IntoNode for NavItem {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<NavItem> for Node {
    fn from(i: NavItem) -> Self {
        i.render()
    }
}

impl Component for NavItem {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Site header / top navigation bar component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Navbar {
    brand: Option<Node>,
    nav_items: Vec<Node>,
    actions: Option<Node>,
    sticky: bool,
    class: String,
}

impl Navbar {
    /// Create a new navbar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set brand element.
    pub fn brand(mut self, brand: impl IntoNode) -> Self {
        self.brand = Some(brand.into_node());
        self
    }

    /// Add a navigation item or link.
    pub fn item(mut self, item: impl IntoNode) -> Self {
        self.nav_items.push(item.into_node());
        self
    }

    /// Add multiple navigation items.
    pub fn items(mut self, items: impl IntoIterator<Item = Node>) -> Self {
        self.nav_items.extend(items);
        self
    }

    /// Set right-aligned action elements (e.g. login button, theme toggle).
    pub fn actions(mut self, actions: impl IntoNode) -> Self {
        self.actions = Some(actions.into_node());
        self
    }

    /// Make the navbar sticky to the top of viewport.
    pub fn sticky(mut self, sticky: bool) -> Self {
        self.sticky = sticky;
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
            "w-full border-b border-gray-200 bg-white/95 backdrop-blur dark:border-gray-800 dark:bg-gray-900/95";
        let sticky_class = if self.sticky { "sticky top-0 z-40" } else { "" };
        let full_class = merge_classes(&[base_class, sticky_class, &self.class]);

        let mut nav = Element::new("nav").attr("class", full_class);

        let mut container = Element::new("div").attr(
            "class",
            "max-w-screen-xl mx-auto flex h-16 items-center justify-between px-4 sm:px-6 lg:px-8",
        );

        // Brand + Navigation items container
        let mut left_section = Element::new("div").attr("class", "flex items-center gap-8");

        if let Some(brand) = self.brand {
            left_section = left_section.child(brand);
        }

        if !self.nav_items.is_empty() {
            let mut links_container =
                Element::new("div").attr("class", "hidden md:flex items-center space-x-4");
            for item in self.nav_items {
                links_container = links_container.child(item);
            }
            left_section = left_section.child(Node::Element(links_container));
        }

        container = container.child(Node::Element(left_section));

        if let Some(actions) = self.actions {
            let actions_container = Element::new("div")
                .attr("class", "flex items-center gap-4")
                .child(actions);
            container = container.child(Node::Element(actions_container));
        }

        nav = nav.child(Node::Element(container));
        Node::Element(nav)
    }
}

/// Convenience function to construct a new [`Navbar`].
pub fn navbar() -> Navbar {
    Navbar::new()
}

impl IntoNode for Navbar {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Navbar> for Node {
    fn from(n: Navbar) -> Self {
        n.render()
    }
}

impl Component for Navbar {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Navigation item within a [`Sidebar`].
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarItem {
    label: String,
    href: String,
    icon: Option<Node>,
    active: bool,
    badge: Option<String>,
    class: String,
}

impl SidebarItem {
    /// Create a new sidebar navigation item.
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
            icon: None,
            active: false,
            badge: None,
            class: String::new(),
        }
    }

    /// Add an icon before the label.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Mark this item as currently active.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add a badge text (e.g. counter).
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let state_class = if self.active {
            "bg-blue-50 text-blue-700 dark:bg-blue-900/50 dark:text-blue-200 font-semibold"
        } else {
            "text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800"
        };

        let full_class = merge_classes(&[
            "flex items-center gap-3 px-3 py-2 text-sm rounded-lg transition-colors group",
            state_class,
            &self.class,
        ]);

        let mut el = Element::new("a")
            .attr("href", &self.href)
            .attr("class", full_class);

        if self.active {
            el = el.attr("aria-current", "page");
        }

        if let Some(icon) = self.icon {
            el = el.child(icon);
        }

        el = el.child(Node::Element(
            Element::new("span")
                .attr("class", "flex-1")
                .child(Node::text(self.label)),
        ));

        if let Some(badge) = self.badge {
            let badge_el = Element::new("span")
                .attr("class", "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-200 text-gray-800 dark:bg-gray-700 dark:text-gray-300")
                .child(Node::text(badge));
            el = el.child(Node::Element(badge_el));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`SidebarItem`].
pub fn sidebar_item(label: impl Into<String>, href: impl Into<String>) -> SidebarItem {
    SidebarItem::new(label, href)
}

impl IntoNode for SidebarItem {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<SidebarItem> for Node {
    fn from(s: SidebarItem) -> Self {
        s.render()
    }
}

impl Component for SidebarItem {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Vertical sidebar navigation drawer or panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sidebar {
    header: Option<Node>,
    items: Vec<Node>,
    footer: Option<Node>,
    class: String,
}

impl Sidebar {
    /// Create a new sidebar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an optional top header/logo in the sidebar.
    pub fn header(mut self, header: impl IntoNode) -> Self {
        self.header = Some(header.into_node());
        self
    }

    /// Add a navigation item or custom node to the sidebar.
    pub fn item(mut self, item: impl IntoNode) -> Self {
        self.items.push(item.into_node());
        self
    }

    /// Add multiple navigation items.
    pub fn items(mut self, items: impl IntoIterator<Item = Node>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set a bottom footer node (e.g. user profile / logout).
    pub fn footer(mut self, footer: impl IntoNode) -> Self {
        self.footer = Some(footer.into_node());
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
            "w-64 flex flex-col border-r border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800 min-h-screen p-4";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut el = Element::new("aside").attr("class", full_class);

        if let Some(header) = self.header {
            let header_el = Element::new("div")
                .attr(
                    "class",
                    "mb-6 pb-4 border-b border-gray-100 dark:border-gray-700",
                )
                .child(header);
            el = el.child(Node::Element(header_el));
        }

        let mut nav = Element::new("nav").attr("class", "flex-1 space-y-1");
        for item in self.items {
            nav = nav.child(item);
        }
        el = el.child(Node::Element(nav));

        if let Some(footer) = self.footer {
            let footer_el = Element::new("div")
                .attr(
                    "class",
                    "mt-auto pt-4 border-t border-gray-100 dark:border-gray-700",
                )
                .child(footer);
            el = el.child(Node::Element(footer_el));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Sidebar`].
pub fn sidebar() -> Sidebar {
    Sidebar::new()
}

impl IntoNode for Sidebar {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Sidebar> for Node {
    fn from(s: Sidebar) -> Self {
        s.render()
    }
}

impl Component for Sidebar {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An individual crumb in a [`Breadcrumbs`] trail.
#[derive(Debug, Clone, PartialEq)]
pub struct BreadcrumbItem {
    label: String,
    href: Option<String>,
    current: bool,
}

impl BreadcrumbItem {
    /// Create a new breadcrumb item with text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
            current: false,
        }
    }

    /// Set the destination URL.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Mark this item as the current page.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }
}

/// Breadcrumb navigation path component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Breadcrumbs {
    items: Vec<BreadcrumbItem>,
    separator: Option<String>,
    class: String,
}

impl Breadcrumbs {
    /// Create a new breadcrumb container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breadcrumb item.
    pub fn item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple breadcrumb items.
    pub fn items(mut self, items: impl IntoIterator<Item = BreadcrumbItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set custom separator text (default is `"/"`).
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = Some(separator.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "flex items-center space-x-2 text-sm text-gray-500 dark:text-gray-400";
        let full_class = merge_classes(&[base_class, &self.class]);
        let sep_text = self.separator.unwrap_or_else(|| "/".to_string());

        let total = self.items.len();
        let mut ol = Element::new("ol").attr("class", full_class);

        for (index, item) in self.items.into_iter().enumerate() {
            let is_last = index == total - 1;
            let mut li = Element::new("li").attr("class", "inline-flex items-center");

            if item.current || is_last || item.href.is_none() {
                let span = Element::new("span")
                    .attr("class", "font-medium text-gray-900 dark:text-white")
                    .attr("aria-current", "page")
                    .child(Node::text(item.label));
                li = li.child(Node::Element(span));
            } else if let Some(href) = item.href {
                let link_el = Element::new("a")
                    .attr("href", href)
                    .attr(
                        "class",
                        "hover:text-gray-700 dark:hover:text-gray-200 transition-colors",
                    )
                    .child(Node::text(item.label));
                li = li.child(Node::Element(link_el));
            }

            if !is_last {
                let sep_el = Element::new("span")
                    .attr("class", "mx-2 text-gray-400 dark:text-gray-600")
                    .attr("aria-hidden", "true")
                    .child(Node::text(&sep_text));
                li = li.child(Node::Element(sep_el));
            }

            ol = ol.child(Node::Element(li));
        }

        let nav = Element::new("nav")
            .attr("aria-label", "Breadcrumb")
            .child(Node::Element(ol));

        Node::Element(nav)
    }
}

/// Convenience function to construct a new [`Breadcrumbs`].
pub fn breadcrumbs() -> Breadcrumbs {
    Breadcrumbs::new()
}

impl IntoNode for Breadcrumbs {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Breadcrumbs> for Node {
    fn from(b: Breadcrumbs) -> Self {
        b.render()
    }
}

impl Component for Breadcrumbs {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An individual tab in a [`Tabs`] component.
#[derive(Debug, Clone, PartialEq)]
pub struct TabItem {
    id: String,
    label: String,
    href: Option<String>,
    active: bool,
    count: Option<usize>,
    icon: Option<Node>,
}

impl TabItem {
    /// Create a new tab item with id and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            href: None,
            active: false,
            count: None,
            icon: None,
        }
    }

    /// Set an optional navigation URL.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Mark this tab as active.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add an optional counter badge.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Add an icon before the tab label.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }
}

/// Accessible tab navigation bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tabs {
    items: Vec<TabItem>,
    class: String,
}

impl Tabs {
    /// Create a new tabs container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tab item.
    pub fn tab(mut self, item: TabItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple tabs.
    pub fn tabs(mut self, items: impl IntoIterator<Item = TabItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "border-b border-gray-200 dark:border-gray-700";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut nav = Element::new("nav")
            .attr("class", "-mb-px flex space-x-8")
            .attr("aria-label", "Tabs");

        for item in self.items {
            let active_class = if item.active {
                "border-blue-600 text-blue-600 dark:border-blue-500 dark:text-blue-400 font-semibold"
            } else {
                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
            };

            let item_class = merge_classes(&[
                "group inline-flex items-center py-4 px-1 border-b-2 text-sm font-medium transition-colors gap-2",
                active_class,
            ]);

            let tag = if item.href.is_some() { "a" } else { "button" };
            let mut tab_el = Element::new(tag)
                .attr("class", item_class)
                .attr("id", &item.id);

            if let Some(href) = &item.href {
                tab_el = tab_el.attr("href", href);
            } else {
                tab_el = tab_el.attr("type", "button");
            }

            if item.active {
                tab_el = tab_el.attr("aria-current", "page");
            }

            if let Some(icon) = item.icon {
                tab_el = tab_el.child(icon);
            }

            tab_el = tab_el.child(Node::text(item.label));

            if let Some(count) = item.count {
                let badge_class = if item.active {
                    "bg-blue-100 text-blue-600 dark:bg-blue-900/60 dark:text-blue-300"
                } else {
                    "bg-gray-100 text-gray-900 dark:bg-gray-800 dark:text-gray-300"
                };
                let badge_full = merge_classes(&[
                    "ml-2 rounded-full py-0.5 px-2.5 text-xs font-medium inline-block",
                    badge_class,
                ]);
                let badge_el = Element::new("span")
                    .attr("class", badge_full)
                    .child(Node::text(count.to_string()));
                tab_el = tab_el.child(Node::Element(badge_el));
            }

            nav = nav.child(Node::Element(tab_el));
        }

        let container = Element::new("div")
            .attr("class", full_class)
            .child(Node::Element(nav));

        Node::Element(container)
    }
}

/// Convenience function to construct a new [`Tabs`].
pub fn tabs() -> Tabs {
    Tabs::new()
}

impl IntoNode for Tabs {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Tabs> for Node {
    fn from(t: Tabs) -> Self {
        t.render()
    }
}

impl Component for Tabs {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── RSX Functional Component Adapters ───────────────────────────────

/// Properties for constructing a [`Navbar`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NavbarProps {
    pub brand: Option<Node>,
    pub sticky: bool,
    pub actions: Option<Node>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Navbar`], enabling `<Navbar ...>...</Navbar>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Navbar(props: NavbarProps) -> Node {
    let mut n = Navbar::new().sticky(props.sticky).items(props.children);
    if let Some(b) = props.brand {
        n = n.brand(b);
    }
    if let Some(a) = props.actions {
        n = n.actions(a);
    }
    if !props.class.is_empty() {
        n = n.class(props.class);
    }
    n.render()
}

/// Properties for constructing a [`Sidebar`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SidebarProps {
    pub header: Option<Node>,
    pub footer: Option<Node>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Sidebar`], enabling `<Sidebar ...>...</Sidebar>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Sidebar(props: SidebarProps) -> Node {
    let mut s = Sidebar::new().items(props.children);
    if let Some(h) = props.header {
        s = s.header(h);
    }
    if let Some(f) = props.footer {
        s = s.footer(f);
    }
    if !props.class.is_empty() {
        s = s.class(props.class);
    }
    s.render()
}

/// Properties for constructing a [`SidebarItem`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SidebarItemProps {
    pub label: String,
    pub href: String,
    pub icon: Option<Node>,
    pub active: bool,
    pub badge: Option<String>,
    pub class: String,
}

/// Functional component adapter for [`SidebarItem`], enabling `<SidebarItem ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn SidebarItem(props: SidebarItemProps) -> Node {
    let mut item = SidebarItem::new(props.label, props.href).active(props.active);
    if let Some(ic) = props.icon {
        item = item.icon(ic);
    }
    if let Some(b) = props.badge {
        item = item.badge(b);
    }
    if !props.class.is_empty() {
        item = item.class(props.class);
    }
    item.render()
}

/// Properties for constructing [`Breadcrumbs`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BreadcrumbsProps {
    pub items: Vec<BreadcrumbItem>,
    pub separator: Option<String>,
    pub class: String,
}

/// Functional component adapter for [`Breadcrumbs`], enabling `<Breadcrumbs ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Breadcrumbs(props: BreadcrumbsProps) -> Node {
    let mut b = Breadcrumbs::new().items(props.items);
    if let Some(sep) = props.separator {
        b = b.separator(sep);
    }
    if !props.class.is_empty() {
        b = b.class(props.class);
    }
    b.render()
}

/// Properties for constructing [`Tabs`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TabsProps {
    pub items: Vec<TabItem>,
    pub class: String,
}

/// Functional component adapter for [`Tabs`], enabling `<Tabs ... />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Tabs(props: TabsProps) -> Node {
    let mut t = Tabs::new().tabs(props.items);
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    t.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn link_renders_href_and_external_attributes() {
        let external_link = Link::new("https://rust-lang.org")
            .label("Rust")
            .external(true)
            .render();
        let html = render_to_string(&external_link);
        assert!(html.starts_with("<a"));
        assert!(html.contains("href=\"https://rust-lang.org\""));
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
        assert!(html.contains("Rust"));
        assert!(html.ends_with("</a>"));
    }

    #[test]
    fn navbar_renders_brand_and_items() {
        let node = Navbar::new()
            .brand(NavbarBrand::new("/").label("YavApp"))
            .item(NavItem::new("Docs", "/docs").active(true))
            .item(NavItem::new("Blog", "/blog"))
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<nav"));
        assert!(html.contains("YavApp"));
        assert!(html.contains("href=\"/docs\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("href=\"/blog\""));
    }

    #[test]
    fn sidebar_renders_navigation() {
        let node = Sidebar::new()
            .header(Node::text("Dashboard"))
            .item(
                SidebarItem::new("Overview", "/overview")
                    .active(true)
                    .badge("3"),
            )
            .item(SidebarItem::new("Settings", "/settings"))
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<aside"));
        assert!(html.contains("Dashboard"));
        assert!(html.contains("Overview"));
        assert!(html.contains("Settings"));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains(">3<"));
    }

    #[test]
    fn breadcrumbs_render_aria_and_separators() {
        let node = Breadcrumbs::new()
            .item(BreadcrumbItem::new("Home").href("/"))
            .item(BreadcrumbItem::new("Library").href("/library"))
            .item(BreadcrumbItem::new("Data").current(true))
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("aria-label=\"Breadcrumb\""));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("href=\"/library\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("Data"));
        assert!(html.contains("/"));
    }

    #[test]
    fn tabs_render_active_state() {
        let node = Tabs::new()
            .tab(TabItem::new("tab-1", "Account").active(true))
            .tab(TabItem::new("tab-2", "Notifications").count(5))
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("aria-label=\"Tabs\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("Account"));
        assert!(html.contains("Notifications"));
        assert!(html.contains(">5<"));
    }
}
