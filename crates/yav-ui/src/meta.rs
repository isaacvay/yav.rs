//! Search Engine Optimization (SEO) & Social Metadata Engine.
//!
//! Provides declarative construction of document `<head>` metadata, OpenGraph tags,
//! Twitter Cards, canonical links, and Schema.org JSON-LD structured data.

use yav_core::{Element, IntoNode, Node};

/// Twitter Card presentation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TwitterCard {
    /// Standard summary card with small square thumbnail.
    Summary,
    /// Large full-width image summary card.
    #[default]
    SummaryLargeImage,
    /// App card for mobile app promotion.
    App,
    /// Player card for video/audio streams.
    Player,
}

impl TwitterCard {
    /// Return the Twitter card string identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::SummaryLargeImage => "summary_large_image",
            Self::App => "app",
            Self::Player => "player",
        }
    }
}

/// Declarative metadata builder for document headers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    title: Option<String>,
    description: Option<String>,
    canonical: Option<String>,
    robots: Option<String>,
    keywords: Option<String>,
    author: Option<String>,

    // OpenGraph
    og_title: Option<String>,
    og_description: Option<String>,
    og_image: Option<String>,
    og_url: Option<String>,
    og_type: Option<String>,
    og_site_name: Option<String>,

    // Twitter
    twitter_card: Option<TwitterCard>,
    twitter_site: Option<String>,
    twitter_creator: Option<String>,
    twitter_title: Option<String>,
    twitter_description: Option<String>,
    twitter_image: Option<String>,

    // Structured Data (JSON-LD)
    json_ld: Vec<String>,

    // Custom tags
    extra_meta: Vec<(String, String)>,
    extra_property_meta: Vec<(String, String)>,
}

impl Metadata {
    /// Create an empty metadata definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set meta description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set canonical URL.
    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical = Some(url.into());
        self
    }

    /// Set robots directive (e.g., "index, follow" or "noindex, nofollow").
    pub fn robots(mut self, directive: impl Into<String>) -> Self {
        self.robots = Some(directive.into());
        self
    }

    /// Set meta keywords (comma-separated).
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Set meta author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set OpenGraph title (defaults to `title` if not set).
    pub fn og_title(mut self, title: impl Into<String>) -> Self {
        self.og_title = Some(title.into());
        self
    }

    /// Set OpenGraph description (defaults to `description` if not set).
    pub fn og_description(mut self, desc: impl Into<String>) -> Self {
        self.og_description = Some(desc.into());
        self
    }

    /// Set OpenGraph image URL.
    pub fn og_image(mut self, url: impl Into<String>) -> Self {
        self.og_image = Some(url.into());
        self
    }

    /// Set OpenGraph canonical URL.
    pub fn og_url(mut self, url: impl Into<String>) -> Self {
        self.og_url = Some(url.into());
        self
    }

    /// Set OpenGraph page type (e.g. "website", "article"). Defaults to "website".
    pub fn og_type(mut self, page_type: impl Into<String>) -> Self {
        self.og_type = Some(page_type.into());
        self
    }

    /// Set OpenGraph site name.
    pub fn og_site_name(mut self, site_name: impl Into<String>) -> Self {
        self.og_site_name = Some(site_name.into());
        self
    }

    /// Set Twitter Card type.
    pub fn twitter_card(mut self, card: TwitterCard) -> Self {
        self.twitter_card = Some(card);
        self
    }

    /// Set Twitter account handle (e.g., "@myorg").
    pub fn twitter_site(mut self, handle: impl Into<String>) -> Self {
        self.twitter_site = Some(handle.into());
        self
    }

    /// Set Twitter creator handle.
    pub fn twitter_creator(mut self, handle: impl Into<String>) -> Self {
        self.twitter_creator = Some(handle.into());
        self
    }

    /// Set Twitter title.
    pub fn twitter_title(mut self, title: impl Into<String>) -> Self {
        self.twitter_title = Some(title.into());
        self
    }

    /// Set Twitter description.
    pub fn twitter_description(mut self, desc: impl Into<String>) -> Self {
        self.twitter_description = Some(desc.into());
        self
    }

    /// Set Twitter image.
    pub fn twitter_image(mut self, url: impl Into<String>) -> Self {
        self.twitter_image = Some(url.into());
        self
    }

    /// Add a Schema.org JSON-LD structured data payload.
    pub fn json_ld(mut self, payload: impl Into<String>) -> Self {
        self.json_ld.push(payload.into());
        self
    }

    /// Add a custom `<meta name="..." content="...">` tag.
    pub fn meta(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.extra_meta.push((name.into(), content.into()));
        self
    }

    /// Add a custom `<meta property="..." content="...">` tag.
    pub fn property(mut self, prop: impl Into<String>, content: impl Into<String>) -> Self {
        self.extra_property_meta.push((prop.into(), content.into()));
        self
    }

    /// Render all tags as a list of virtual DOM [`Node`] elements.
    pub fn render(self) -> Node {
        let mut nodes = Vec::new();

        // 1. Title
        if let Some(ref title) = self.title {
            let mut el = Element::new("title");
            el = el.child(Node::text(title));
            nodes.push(Node::Element(el));
        }

        // 2. Standard Meta
        if let Some(ref desc) = self.description {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "description")
                    .attr("content", desc),
            ));
        }

        if let Some(ref canon) = self.canonical {
            nodes.push(Node::Element(
                Element::new("link")
                    .attr("rel", "canonical")
                    .attr("href", canon),
            ));
        }

        if let Some(ref robots) = self.robots {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "robots")
                    .attr("content", robots),
            ));
        }

        if let Some(ref kw) = self.keywords {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "keywords")
                    .attr("content", kw),
            ));
        }

        if let Some(ref author) = self.author {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "author")
                    .attr("content", author),
            ));
        }

        // 3. OpenGraph
        let effective_og_title = self.og_title.as_ref().or(self.title.as_ref());
        if let Some(og_t) = effective_og_title {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", "og:title")
                    .attr("content", og_t),
            ));
        }

        let effective_og_desc = self.og_description.as_ref().or(self.description.as_ref());
        if let Some(og_d) = effective_og_desc {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", "og:description")
                    .attr("content", og_d),
            ));
        }

        let effective_og_type = self.og_type.as_deref().unwrap_or("website");
        nodes.push(Node::Element(
            Element::new("meta")
                .attr("property", "og:type")
                .attr("content", effective_og_type),
        ));

        if let Some(ref og_img) = self.og_image {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", "og:image")
                    .attr("content", og_img),
            ));
        }

        let effective_og_url = self.og_url.as_ref().or(self.canonical.as_ref());
        if let Some(og_u) = effective_og_url {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", "og:url")
                    .attr("content", og_u),
            ));
        }

        if let Some(ref site_name) = self.og_site_name {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", "og:site_name")
                    .attr("content", site_name),
            ));
        }

        // 4. Twitter Cards
        let card = self.twitter_card.unwrap_or_default();
        nodes.push(Node::Element(
            Element::new("meta")
                .attr("name", "twitter:card")
                .attr("content", card.as_str()),
        ));

        if let Some(ref site) = self.twitter_site {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "twitter:site")
                    .attr("content", site),
            ));
        }

        if let Some(ref creator) = self.twitter_creator {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "twitter:creator")
                    .attr("content", creator),
            ));
        }

        let effective_tw_title = self.twitter_title.as_ref().or(effective_og_title);
        if let Some(tw_t) = effective_tw_title {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "twitter:title")
                    .attr("content", tw_t),
            ));
        }

        let effective_tw_desc = self.twitter_description.as_ref().or(effective_og_desc);
        if let Some(tw_d) = effective_tw_desc {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "twitter:description")
                    .attr("content", tw_d),
            ));
        }

        let effective_tw_img = self.twitter_image.as_ref().or(self.og_image.as_ref());
        if let Some(tw_i) = effective_tw_img {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", "twitter:image")
                    .attr("content", tw_i),
            ));
        }

        // 5. Custom Meta
        for (name, content) in self.extra_meta {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("name", name)
                    .attr("content", content),
            ));
        }

        for (prop, content) in self.extra_property_meta {
            nodes.push(Node::Element(
                Element::new("meta")
                    .attr("property", prop)
                    .attr("content", content),
            ));
        }

        // 6. JSON-LD scripts (sanitize </ to <\/ to prevent script breakout XSS)
        for json in self.json_ld {
            let mut script = Element::new("script").attr("type", "application/ld+json");
            let safe_json = json.replace("</", "<\\/");
            script = script.child(Node::raw(safe_json));
            nodes.push(Node::Element(script));
        }

        Node::Fragment(nodes)
    }
}

impl IntoNode for Metadata {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Metadata> for Node {
    fn from(m: Metadata) -> Self {
        m.render()
    }
}

/// Properties for `<MetaTags />` RSX component.
#[derive(Debug, Clone, Default)]
pub struct MetaTagsProps {
    /// Pre-configured metadata object.
    pub metadata: Option<Metadata>,
    /// Inline title shortcut.
    pub title: Option<String>,
    /// Inline description shortcut.
    pub description: Option<String>,
    /// Inline canonical URL shortcut.
    pub canonical: Option<String>,
    /// Inline OpenGraph image shortcut.
    pub image: Option<String>,
}

/// RSX functional component: `<MetaTags ... />`.
#[allow(non_snake_case)]
pub fn MetaTags(props: MetaTagsProps) -> Node {
    let mut meta = props.metadata.unwrap_or_default();
    if let Some(title) = props.title {
        meta = meta.title(title);
    }
    if let Some(desc) = props.description {
        meta = meta.description(desc);
    }
    if let Some(canonical) = props.canonical {
        meta = meta.canonical(canonical);
    }
    if let Some(img) = props.image {
        meta = meta.og_image(img);
    }
    meta.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_basic() {
        use yav_core::render_to_string;

        let meta = Metadata::new()
            .title("Accueil | Mon App")
            .description("Description de mon application")
            .canonical("https://example.com")
            .og_image("https://example.com/banner.png");

        let node = meta.render();
        match node {
            Node::Fragment(nodes) => {
                assert!(!nodes.is_empty());
                let html = nodes
                    .into_iter()
                    .map(|n| render_to_string(&n))
                    .collect::<String>();
                assert!(html.contains("<title>Accueil | Mon App</title>"));
                assert!(html.contains(
                    r#"<meta name="description" content="Description de mon application""#
                ));
                assert!(html.contains(r#"<link rel="canonical" href="https://example.com""#));
                assert!(html.contains(
                    r#"<meta property="og:image" content="https://example.com/banner.png""#
                ));
                assert!(html.contains(r#"<meta name="twitter:card" content="summary_large_image""#));
            }
            _ => panic!("Expected fragment"),
        }
    }

    #[test]
    fn test_json_ld() {
        use yav_core::render_to_string;

        let meta =
            Metadata::new().json_ld(r#"{"@context":"https://schema.org","@type":"WebSite"}"#);
        let node = meta.render();
        let html = render_to_string(&node);
        assert!(html.contains(r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"WebSite"}</script>"#));
    }

    #[test]
    fn test_json_ld_script_breakout_sanitization() {
        use yav_core::render_to_string;

        let payload = r#"{"title":"Malicious </script><script>alert(1)</script>"}"#;
        let meta = Metadata::new().json_ld(payload);
        let node = meta.render();
        let html = render_to_string(&node);
        assert!(!html.contains("</script><script>"));
        assert!(html.contains(r#"<script type="application/ld+json">{"title":"Malicious <\/script><script>alert(1)<\/script>"}</script>"#));
    }
}
