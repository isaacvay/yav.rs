//! Component trait and related types.
//!
//! Components are the primary building blocks of Yav applications. Each
//! component implements the [`Component`] trait to produce a [`Node`] tree.

use crate::node::Node;

/// A renderable component that produces a virtual DOM [`Node`].
///
/// Components are typically used with the `#[component]` attribute macro
/// and the `rsx!` macro, but can also be implemented manually.
///
/// # Example
/// ```
/// use yav_core::{Component, Node, Element};
///
/// struct Greeting {
///     name: String,
/// }
///
/// impl Component for Greeting {
///     fn render(&self) -> Node {
///         Node::Element(
///             Element::new("p")
///                 .child(Node::text(format!("Hello, {}!", self.name)))
///         )
///     }
/// }
///
/// let greeting = Greeting { name: "World".into() };
/// let node = greeting.render();
/// assert_eq!(yav_core::render_to_string(&node), "<p>Hello, World!</p>");
/// ```
pub trait Component {
    /// Render this component into a [`Node`] tree.
    fn render(&self) -> Node;
}

/// Metadata for a page component, used by the router, SEO engine, and document shell.
#[derive(Debug, Clone, Default)]
pub struct PageMeta {
    /// Page title for the `<title>` tag.
    pub title: Option<String>,
    /// Meta description `<meta name="description" ...>`.
    pub description: Option<String>,
    /// Canonical URL `<link rel="canonical" ...>`.
    pub canonical_url: Option<String>,
    /// Favicon URL `<link rel="icon" ...>`.
    pub favicon: Option<String>,
    /// OpenGraph title `<meta property="og:title" ...>`.
    pub og_title: Option<String>,
    /// OpenGraph description `<meta property="og:description" ...>`.
    pub og_description: Option<String>,
    /// OpenGraph image URL `<meta property="og:image" ...>`.
    pub og_image: Option<String>,
    /// OpenGraph type `<meta property="og:type" ...>` (default: "website").
    pub og_type: Option<String>,
    /// Twitter card type `<meta name="twitter:card" ...>` (e.g. "summary_large_image").
    pub twitter_card: Option<String>,
    /// JSON-LD structured data string `<script type="application/ld+json">`.
    pub json_ld: Option<String>,
    /// Additional CSS file paths to include.
    pub css: Vec<String>,
    /// Additional script paths to include.
    pub scripts: Vec<String>,
    /// HTTP status code override (default 200).
    pub status: Option<u16>,
}

impl PageMeta {
    /// Create a new `PageMeta` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set meta description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set canonical URL.
    #[must_use]
    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical_url = Some(url.into());
        self
    }

    /// Set favicon URL.
    #[must_use]
    pub fn favicon(mut self, url: impl Into<String>) -> Self {
        self.favicon = Some(url.into());
        self
    }

    /// Set OpenGraph metadata.
    #[must_use]
    pub fn open_graph(
        mut self,
        title: impl Into<String>,
        description: impl Into<String>,
        image: impl Into<String>,
    ) -> Self {
        self.og_title = Some(title.into());
        self.og_description = Some(description.into());
        self.og_image = Some(image.into());
        if self.og_type.is_none() {
            self.og_type = Some("website".to_string());
        }
        self
    }

    /// Set Twitter card type (e.g. "summary_large_image").
    #[must_use]
    pub fn twitter_card(mut self, card_type: impl Into<String>) -> Self {
        self.twitter_card = Some(card_type.into());
        self
    }

    /// Set JSON-LD structured data (Schema.org).
    #[must_use]
    pub fn json_ld(mut self, json: impl Into<String>) -> Self {
        self.json_ld = Some(json.into());
        self
    }

    /// Add a CSS stylesheet path.
    #[must_use]
    pub fn css(mut self, path: impl Into<String>) -> Self {
        self.css.push(path.into());
        self
    }

    /// Add a JavaScript script path.
    #[must_use]
    pub fn script(mut self, path: impl Into<String>) -> Self {
        self.scripts.push(path.into());
        self
    }

    /// Set HTTP status code override.
    #[must_use]
    pub fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    /// Render all meta tags, canonical link, favicon, and JSON-LD as an HTML `<head>` string.
    pub fn render_head_tags(&self) -> String {
        let mut out = String::new();

        if let Some(ref desc) = self.description {
            out.push_str(&format!(
                r#"<meta name="description" content="{}" />"#,
                crate::escape::escape_attribute(desc)
            ));
        }

        if let Some(ref canon) = self.canonical_url {
            if let Some(safe_canon) = crate::escape::escape_url(canon) {
                out.push_str(&format!(r#"<link rel="canonical" href="{safe_canon}" />"#));
            }
        }

        if let Some(ref fav) = self.favicon {
            if let Some(safe_fav) = crate::escape::escape_url(fav) {
                out.push_str(&format!(r#"<link rel="icon" href="{safe_fav}" />"#));
            }
        }

        // OpenGraph
        if let Some(ref og_t) = self.og_title {
            out.push_str(&format!(
                r#"<meta property="og:title" content="{}" />"#,
                crate::escape::escape_attribute(og_t)
            ));
        }
        if let Some(ref og_d) = self.og_description {
            out.push_str(&format!(
                r#"<meta property="og:description" content="{}" />"#,
                crate::escape::escape_attribute(og_d)
            ));
        }
        if let Some(ref og_img) = self.og_image {
            if let Some(safe_img) = crate::escape::escape_url(og_img) {
                out.push_str(&format!(
                    r#"<meta property="og:image" content="{safe_img}" />"#
                ));
            }
        }
        if let Some(ref og_typ) = self.og_type {
            out.push_str(&format!(
                r#"<meta property="og:type" content="{}" />"#,
                crate::escape::escape_attribute(og_typ)
            ));
        }

        // Twitter Card
        if let Some(ref tw) = self.twitter_card {
            out.push_str(&format!(
                r#"<meta name="twitter:card" content="{}" />"#,
                crate::escape::escape_attribute(tw)
            ));
        }

        // JSON-LD (sanitize </ to <\/ to prevent script breakout XSS)
        if let Some(ref ld) = self.json_ld {
            let safe_ld = ld.replace("</", "<\\/");
            out.push_str(&format!(
                r#"<script type="application/ld+json">{}</script>"#,
                safe_ld
            ));
        }

        out
    }
}

/// An entry in a sitemap.xml document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SitemapEntry {
    /// URL location of the page.
    pub loc: String,
    /// Last modification date (YYYY-MM-DD).
    pub lastmod: Option<String>,
    /// Expected change frequency ("always", "hourly", "daily", "weekly", "monthly", "yearly").
    pub changefreq: Option<String>,
    /// Priority relative to other URLs (0.0 to 1.0).
    pub priority: Option<f32>,
}

/// Sitemap XML generator for SEO crawlers.
#[derive(Debug, Clone, Default)]
pub struct Sitemap {
    entries: Vec<SitemapEntry>,
}

impl Sitemap {
    /// Create a new empty sitemap.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a simple URL to the sitemap.
    #[must_use]
    pub fn add_url(mut self, loc: impl Into<String>) -> Self {
        self.entries.push(SitemapEntry {
            loc: loc.into(),
            ..Default::default()
        });
        self
    }

    /// Add an entry with full metadata.
    #[must_use]
    pub fn add_entry(mut self, entry: SitemapEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Render the sitemap into XML format.
    pub fn to_xml(&self) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
        xml.push('\n');
        for e in &self.entries {
            xml.push_str("  <url>\n");
            xml.push_str(&format!(
                "    <loc>{}</loc>\n",
                crate::escape::escape_html(&e.loc)
            ));
            if let Some(ref lm) = e.lastmod {
                xml.push_str(&format!(
                    "    <lastmod>{}</lastmod>\n",
                    crate::escape::escape_html(lm)
                ));
            }
            if let Some(ref cf) = e.changefreq {
                xml.push_str(&format!(
                    "    <changefreq>{}</changefreq>\n",
                    crate::escape::escape_html(cf)
                ));
            }
            if let Some(p) = e.priority {
                xml.push_str(&format!("    <priority>{:.1}</priority>\n", p));
            }
            xml.push_str("  </url>\n");
        }
        xml.push_str("</urlset>");
        xml
    }
}

/// Robots.txt generator for search engines.
#[derive(Debug, Clone)]
pub struct RobotsTxt {
    user_agent: String,
    disallow: Vec<String>,
    allow: Vec<String>,
    sitemap: Option<String>,
}

impl Default for RobotsTxt {
    fn default() -> Self {
        Self {
            user_agent: "*".to_string(),
            disallow: Vec::new(),
            allow: vec!["/".to_string()],
            sitemap: None,
        }
    }
}

impl RobotsTxt {
    /// Create a new robots.txt allowing all crawlers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Disallow paths.
    #[must_use]
    pub fn disallow(mut self, path: impl Into<String>) -> Self {
        self.disallow.push(path.into());
        self
    }

    /// Set sitemap URL.
    #[must_use]
    pub fn sitemap(mut self, url: impl Into<String>) -> Self {
        self.sitemap = Some(url.into());
        self
    }

    /// Render to robots.txt format (stripping CR/LF to prevent directive injection).
    pub fn to_txt(&self) -> String {
        let sanitize_line =
            |s: &str| -> String { s.chars().filter(|&c| c != '\r' && c != '\n').collect() };
        let mut txt = format!("User-agent: {}\n", sanitize_line(&self.user_agent));
        for a in &self.allow {
            txt.push_str(&format!("Allow: {}\n", sanitize_line(a)));
        }
        for d in &self.disallow {
            txt.push_str(&format!("Disallow: {}\n", sanitize_line(d)));
        }
        if let Some(ref sm) = self.sitemap {
            txt.push_str(&format!("Sitemap: {}\n", sanitize_line(sm)));
        }
        txt
    }
}

/// A layout component that wraps page content.
///
/// Layouts receive the child content as a [`Node`] and wrap it with
/// shared UI elements (navigation, footer, etc.).
pub trait Layout {
    /// Render the layout wrapping the given child content.
    fn render_with(&self, children: Node) -> Node;
}

/// Children container for components that accept child nodes.
#[derive(Debug, Clone, Default)]
pub struct Children {
    nodes: Vec<Node>,
}

impl Children {
    /// Create an empty children container.
    pub fn new() -> Self {
        Children { nodes: Vec::new() }
    }

    /// Create children from a vector of nodes.
    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        Children { nodes }
    }

    /// Add a child node.
    pub fn push(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Render all children into a single fragment node.
    pub fn into_node(self) -> Node {
        if self.nodes.is_empty() {
            Node::Empty
        } else if self.nodes.len() == 1 {
            self.nodes.into_iter().next().unwrap()
        } else {
            Node::Fragment(self.nodes)
        }
    }

    /// Check if there are no children.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the number of children.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns a slice of child nodes.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}

impl crate::node::IntoNode for Children {
    fn into_node(self) -> Node {
        self.into_node()
    }
}

impl From<Vec<Node>> for Children {
    fn from(nodes: Vec<Node>) -> Self {
        Children { nodes }
    }
}

impl From<Node> for Children {
    fn from(node: Node) -> Self {
        Children { nodes: vec![node] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Element;

    struct TestComponent;

    impl Component for TestComponent {
        fn render(&self) -> Node {
            Node::Element(Element::new("div").child(Node::text("test")))
        }
    }

    #[test]
    fn component_renders() {
        let comp = TestComponent;
        let node = comp.render();
        let html = crate::render::render_to_string(&node);
        assert_eq!(html, "<div>test</div>");
    }

    #[test]
    fn children_empty() {
        let children = Children::new();
        assert!(children.is_empty());
        assert_eq!(children.into_node(), Node::Empty);
    }

    #[test]
    fn children_single() {
        let children = Children::from_nodes(vec![Node::text("hello")]);
        assert_eq!(children.len(), 1);
        assert_eq!(children.into_node(), Node::Text("hello".to_string()));
    }

    #[test]
    fn children_multiple() {
        let children = Children::from_nodes(vec![Node::text("a"), Node::text("b")]);
        assert_eq!(children.len(), 2);
        let node = children.into_node();
        match node {
            Node::Fragment(nodes) => assert_eq!(nodes.len(), 2),
            _ => panic!("Expected fragment"),
        }
    }

    #[test]
    fn page_meta_default() {
        let meta = PageMeta::default();
        assert!(meta.title.is_none());
        assert!(meta.status.is_none());
    }
}
