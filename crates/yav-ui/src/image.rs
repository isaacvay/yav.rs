//! High-performance responsive image component.
//!
//! Provides automatic lazy loading, async decoding, aspect-ratio reservation
//! to eliminate Cumulative Layout Shift (CLS), and responsive srcset support.

use yav_core::{Component, Element, IntoNode, Node};

/// A responsive, optimized image component.
///
/// Automatically adds `loading="lazy"` and `decoding="async"`, supports width/height
/// attributes for CLS prevention, and custom styling with Tailwind classes.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    src: String,
    alt: String,
    width: Option<u32>,
    height: Option<u32>,
    loading: String,
    decoding: String,
    class: String,
    srcset: Option<String>,
    sizes: Option<String>,
    priority: bool,
    aspect_ratio: Option<String>,
    object_fit: Option<String>,
}

impl Image {
    /// Create a new image with source URL and alt text.
    pub fn new(src: impl Into<String>, alt: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            alt: alt.into(),
            width: None,
            height: None,
            loading: "lazy".to_string(),
            decoding: "async".to_string(),
            class: String::new(),
            srcset: None,
            sizes: None,
            priority: false,
            aspect_ratio: None,
            object_fit: None,
        }
    }

    /// Set intrinsic width in pixels.
    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set intrinsic height in pixels.
    pub fn height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set dimensions (width and height) simultaneously.
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Mark this image as high priority (above-the-fold).
    /// Sets `loading="eager"` and adds `fetchpriority="high"`.
    pub fn priority(mut self, priority: bool) -> Self {
        self.priority = priority;
        if priority {
            self.loading = "eager".to_string();
        }
        self
    }

    /// Set the CSS class attribute.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set responsive `srcset` attribute.
    pub fn srcset(mut self, srcset: impl Into<String>) -> Self {
        self.srcset = Some(srcset.into());
        self
    }

    /// Set responsive `sizes` attribute.
    pub fn sizes(mut self, sizes: impl Into<String>) -> Self {
        self.sizes = Some(sizes.into());
        self
    }

    /// Set CSS `aspect-ratio` style (e.g. `"16/9"`, `"4/3"`).
    pub fn aspect_ratio(mut self, ratio: impl Into<String>) -> Self {
        self.aspect_ratio = Some(ratio.into());
        self
    }

    /// Set object fit styling (`"cover"`, `"contain"`).
    pub fn object_fit(mut self, fit: impl Into<String>) -> Self {
        self.object_fit = Some(fit.into());
        self
    }

    /// Render to virtual DOM element.
    pub fn render(&self) -> Node {
        let mut el = Element::new("img")
            .attr("src", &self.src)
            .attr("alt", &self.alt)
            .attr("loading", &self.loading)
            .attr("decoding", &self.decoding);

        if self.priority {
            el = el.attr("fetchpriority", "high");
        }

        if let Some(w) = self.width {
            el = el.attr("width", w.to_string());
        }

        if let Some(h) = self.height {
            el = el.attr("height", h.to_string());
        }

        if let Some(ref ss) = self.srcset {
            el = el.attr("srcset", ss);
        }

        if let Some(ref sz) = self.sizes {
            el = el.attr("sizes", sz);
        }

        let mut classes = vec!["h-auto", "max-w-full"];
        if let Some(ref fit) = self.object_fit {
            match fit.as_str() {
                "cover" => classes.push("object-cover"),
                "contain" => classes.push("object-contain"),
                _ => {}
            }
        }
        if !self.class.is_empty() {
            classes.push(&self.class);
        }
        let full_class = crate::utils::merge_classes(&classes);
        if !full_class.is_empty() {
            el = el.attr("class", full_class);
        }

        if let Some(ref ratio) = self.aspect_ratio {
            let trimmed = ratio.trim();
            let is_safe = !trimmed.is_empty()
                && (trimmed.eq_ignore_ascii_case("auto")
                    || trimmed.chars().all(|c| {
                        c.is_ascii_digit() || c == '/' || c == '.' || c.is_ascii_whitespace()
                    }));
            if is_safe {
                el = el.attr("style", format!("aspect-ratio: {trimmed};"));
            }
        }

        Node::Element(el)
    }
}

/// Convenience builder function for [`Image`].
pub fn image(src: impl Into<String>, alt: impl Into<String>) -> Image {
    Image::new(src, alt)
}

impl IntoNode for Image {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Image> for Node {
    fn from(img: Image) -> Self {
        img.render()
    }
}

impl Component for Image {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Properties for constructing an `Image` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageProps {
    pub src: String,
    pub alt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub class: String,
    pub priority: bool,
    pub srcset: Option<String>,
    pub sizes: Option<String>,
    pub aspect_ratio: Option<String>,
    pub object_fit: Option<String>,
}

/// Functional component adapter for `Image`, enabling `<Image src="..." alt="..." />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Image(props: ImageProps) -> Node {
    let mut img = Image::new(props.src, props.alt);
    if let Some(w) = props.width {
        img = img.width(w);
    }
    if let Some(h) = props.height {
        img = img.height(h);
    }
    if props.priority {
        img = img.priority(true);
    }
    if !props.class.is_empty() {
        img = img.class(props.class);
    }
    if let Some(ss) = props.srcset {
        img = img.srcset(ss);
    }
    if let Some(sz) = props.sizes {
        img = img.sizes(sz);
    }
    if let Some(ar) = props.aspect_ratio {
        img = img.aspect_ratio(ar);
    }
    if let Some(fit) = props.object_fit {
        img = img.object_fit(fit);
    }
    img.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_image_default_attributes() {
        let img = Image::new("/logo.png", "Company Logo")
            .width(200)
            .height(50)
            .render();
        let html = render_to_string(&img);
        assert!(html.contains(r#"src="/logo.png""#));
        assert!(html.contains(r#"alt="Company Logo""#));
        assert!(html.contains(r#"loading="lazy""#));
        assert!(html.contains(r#"decoding="async""#));
        assert!(html.contains(r#"width="200""#));
        assert!(html.contains(r#"height="50""#));
    }

    #[test]
    fn test_image_priority() {
        let img = Image::new("/hero.jpg", "Hero banner")
            .priority(true)
            .render();
        let html = render_to_string(&img);
        assert!(html.contains(r#"loading="eager""#));
        assert!(html.contains(r#"fetchpriority="high""#));
    }

    #[test]
    fn test_image_aspect_ratio_safety() {
        let valid_img = Image::new("/test.png", "Test")
            .aspect_ratio("16/9")
            .render();
        let valid_html = render_to_string(&valid_img);
        assert!(valid_html.contains(r#"style="aspect-ratio: 16/9;""#));

        let auto_img = Image::new("/test.png", "Test")
            .aspect_ratio("auto")
            .render();
        let auto_html = render_to_string(&auto_img);
        assert!(auto_html.contains(r#"style="aspect-ratio: auto;""#));

        // Malicious style injection must be stripped
        let evil_img = Image::new("/test.png", "Test")
            .aspect_ratio("16/9; background: url('evil.com')")
            .render();
        let evil_html = render_to_string(&evil_img);
        assert!(!evil_html.contains("style="));
        assert!(!evil_html.contains("evil.com"));
    }
}
