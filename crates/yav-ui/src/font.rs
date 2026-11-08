//! Web Font Optimization & Zero-CLS Layout Stabilization.
//!
//! Provides `@next/font` parity for Rust:
//! - **GoogleFont**: Generates preconnect hints, Google Fonts v2 stylesheets, and
//!   zero-CLS size-adjusting `@font-face` fallback overrides.
//! - **LocalFont**: Preloads `.woff2` files and generates `@font-face` definitions
//!   with custom CSS variables.
//! - Eliminates Cumulative Layout Shift (CLS) when fonts swap in by matching
//!   x-height and ascent/descent metrics of fallback fonts (Arial / Times New Roman).

use yav_core::{Component, Element, IntoNode, Node};

/// Font loading display strategy (`font-display`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontDisplay {
    /// Swap fallback font immediately for web font once downloaded (recommended for UI text).
    #[default]
    Swap,
    /// Block rendering briefly, then swap.
    Block,
    /// Extremely brief block period with small swap period (recommended for slow connections).
    Fallback,
    /// Brief block period with zero swap period (prevents layout shift completely).
    Optional,
}

impl FontDisplay {
    /// CSS string value for this display strategy.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Swap => "swap",
            Self::Block => "block",
            Self::Fallback => "fallback",
            Self::Optional => "optional",
        }
    }
}

/// Fallback metric overrides to prevent Cumulative Layout Shift (CLS).
#[derive(Debug, Clone, PartialEq)]
pub struct FallbackMetrics {
    /// CSS `size-adjust` percentage (e.g. `"104.5%"`).
    pub size_adjust: &'static str,
    /// CSS `ascent-override` percentage (e.g. `"93.5%"`).
    pub ascent_override: &'static str,
    /// CSS `descent-override` percentage (e.g. `"23.9%"`).
    pub descent_override: &'static str,
    /// CSS `line-gap-override` percentage (e.g. `"0%"`).
    pub line_gap_override: &'static str,
}

impl Default for FallbackMetrics {
    fn default() -> Self {
        Self {
            size_adjust: "100%",
            ascent_override: "90%",
            descent_override: "22%",
            line_gap_override: "0%",
        }
    }
}

/// Google Font optimizer with automatic preconnect, subsetting, and zero-CLS fallback metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct GoogleFont {
    family: String,
    weights: Vec<u16>,
    subsets: Vec<String>,
    display: FontDisplay,
    italic: bool,
    variable: Option<String>,
    fallback: Vec<String>,
    adjust_fallback: bool,
}

impl GoogleFont {
    /// Create a new Google Font specification by family name (e.g. `"Inter"`, `"Roboto"`, `"Geist"`).
    pub fn new(family: impl Into<String>) -> Self {
        let fam = family.into();
        let var_name = format!("--font-{}", fam.to_lowercase().replace(' ', "-"));
        Self {
            family: fam,
            weights: vec![400, 600, 700],
            subsets: vec!["latin".to_string()],
            display: FontDisplay::Swap,
            italic: false,
            variable: Some(var_name),
            fallback: vec![
                "system-ui".to_string(),
                "-apple-system".to_string(),
                "sans-serif".to_string(),
            ],
            adjust_fallback: true,
        }
    }

    /// Specify font weights to load (e.g. `&[400, 500, 700]`).
    #[must_use]
    pub fn weights(mut self, weights: &[u16]) -> Self {
        self.weights = weights.to_vec();
        self
    }

    /// Specify character subsets to load (default: `&["latin"]`).
    #[must_use]
    pub fn subsets(mut self, subsets: &[&str]) -> Self {
        self.subsets = subsets.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set font display strategy (`FontDisplay::Swap`, `FontDisplay::Optional`, etc.).
    #[must_use]
    pub fn display(mut self, display: FontDisplay) -> Self {
        self.display = display;
        self
    }

    /// Include italic variants.
    #[must_use]
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Custom CSS custom property name (e.g. `"--font-inter"`).
    #[must_use]
    pub fn variable(mut self, var_name: impl Into<String>) -> Self {
        self.variable = Some(var_name.into());
        self
    }

    /// Fallback font stack.
    #[must_use]
    pub fn fallback(mut self, fallback: &[&str]) -> Self {
        self.fallback = fallback.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Whether to generate zero-CLS fallback `@font-face` overrides.
    #[must_use]
    pub fn adjust_fallback(mut self, adjust: bool) -> Self {
        self.adjust_fallback = adjust;
        self
    }

    /// Compute Google Fonts CSS2 API URL.
    #[must_use]
    pub fn fonts_api_url(&self) -> String {
        let encoded_family = self.family.replace(' ', "+");
        let mut sorted_weights = self.weights.clone();
        sorted_weights.sort_unstable();
        sorted_weights.dedup();

        let weights_str = sorted_weights
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
            .join(";");

        let mut url = format!(
            "https://fonts.googleapis.com/css2?family={encoded_family}:wght@{weights_str}&display={}",
            self.display.as_str()
        );

        if !self.subsets.is_empty() {
            url.push_str("&subset=");
            url.push_str(&self.subsets.join(","));
        }

        url
    }

    /// Get zero-CLS fallback metrics for this font family.
    #[must_use]
    pub fn fallback_metrics(&self) -> FallbackMetrics {
        match self.family.to_lowercase().as_str() {
            "inter" => FallbackMetrics {
                size_adjust: "104.5%",
                ascent_override: "93.5%",
                descent_override: "23.9%",
                line_gap_override: "0%",
            },
            "roboto" => FallbackMetrics {
                size_adjust: "100.2%",
                ascent_override: "92.8%",
                descent_override: "24.4%",
                line_gap_override: "0%",
            },
            "geist" | "geist sans" => FallbackMetrics {
                size_adjust: "102.1%",
                ascent_override: "95.0%",
                descent_override: "25.0%",
                line_gap_override: "0%",
            },
            "fira code" | "jetbrains mono" => FallbackMetrics {
                size_adjust: "101.0%",
                ascent_override: "98.0%",
                descent_override: "27.0%",
                line_gap_override: "0%",
            },
            "poppins" => FallbackMetrics {
                size_adjust: "108.0%",
                ascent_override: "105.0%",
                descent_override: "35.0%",
                line_gap_override: "0%",
            },
            _ => FallbackMetrics::default(),
        }
    }

    /// Generate CSS class name to apply this font family (e.g. `"font-inter"`).
    #[must_use]
    pub fn class_name(&self) -> String {
        format!("font-{}", self.family.to_lowercase().replace(' ', "-"))
    }

    /// Generate CSS custom variable name (e.g. `"--font-inter"`).
    #[must_use]
    pub fn css_variable(&self) -> String {
        self.variable
            .clone()
            .unwrap_or_else(|| format!("--font-{}", self.family.to_lowercase().replace(' ', "-")))
    }

    /// Render zero-CLS `@font-face` override CSS rules.
    #[must_use]
    pub fn generate_override_css(&self) -> String {
        let safe_family = self.family.replace(['\'', '"', '\\', '<', '>'], "");
        let fallback_font_name = format!("{} Fallback", safe_family);
        let metrics = self.fallback_metrics();
        let var_name = self.css_variable();
        let fallback_chain = self.fallback.join(", ");

        format!(
            "@font-face {{\n  font-family: '{fallback_font_name}';\n  src: local('Arial'), local('Helvetica');\n  ascent-override: {};\n  descent-override: {};\n  size-adjust: {};\n  line-gap-override: {};\n}}\n:root {{\n  {var_name}: '{safe_family}', '{fallback_font_name}', {fallback_chain};\n}}",
            metrics.ascent_override,
            metrics.descent_override,
            metrics.size_adjust,
            metrics.line_gap_override
        )
    }

    /// Generate all HTML `<link>` and `<style>` elements for `<Head>`.
    #[must_use]
    pub fn to_head_nodes(&self) -> Vec<Node> {
        let mut nodes = Vec::new();

        // 1. Preconnect to fonts.googleapis.com
        nodes.push(
            Element::new("link")
                .attr("rel", "preconnect")
                .attr("href", "https://fonts.googleapis.com")
                .into_node(),
        );

        // 2. Preconnect to fonts.gstatic.com with crossorigin
        nodes.push(
            Element::new("link")
                .attr("rel", "preconnect")
                .attr("href", "https://fonts.gstatic.com")
                .attr("crossorigin", "")
                .into_node(),
        );

        // 3. Stylesheet link
        nodes.push(
            Element::new("link")
                .attr("rel", "stylesheet")
                .attr("href", self.fonts_api_url())
                .into_node(),
        );

        // 4. Zero-CLS style tag if enabled
        if self.adjust_fallback {
            nodes.push(
                Element::new("style")
                    .child(Node::raw(self.generate_override_css()))
                    .into_node(),
            );
        }

        nodes
    }
}

impl Component for GoogleFont {
    fn render(&self) -> Node {
        let nodes = self.to_head_nodes();
        Node::Fragment(nodes)
    }
}

/// Local font definition with preload hints and `@font-face` registration.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalFont {
    family: String,
    src: String,
    weight: String,
    style: String,
    display: FontDisplay,
    variable: Option<String>,
    preload: bool,
}

impl LocalFont {
    /// Create a new local font file definition.
    pub fn new(family: impl Into<String>, src: impl Into<String>) -> Self {
        let fam = family.into();
        let var_name = format!("--font-{}", fam.to_lowercase().replace(' ', "-"));
        Self {
            family: fam,
            src: src.into(),
            weight: "normal".to_string(),
            style: "normal".to_string(),
            display: FontDisplay::Swap,
            variable: Some(var_name),
            preload: true,
        }
    }

    /// Set font weight (e.g. `"400"`, `"700"`, `"100 900"`).
    #[must_use]
    pub fn weight(mut self, weight: impl Into<String>) -> Self {
        self.weight = weight.into();
        self
    }

    /// Set font style (e.g. `"normal"`, `"italic"`).
    #[must_use]
    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    /// Set font display strategy.
    #[must_use]
    pub fn display(mut self, display: FontDisplay) -> Self {
        self.display = display;
        self
    }

    /// Custom CSS custom property name.
    #[must_use]
    pub fn variable(mut self, var_name: impl Into<String>) -> Self {
        self.variable = Some(var_name.into());
        self
    }

    /// Whether to generate a `<link rel="preload">` element for this font file.
    #[must_use]
    pub fn preload(mut self, preload: bool) -> Self {
        self.preload = preload;
        self
    }

    /// Generate CSS custom variable name.
    #[must_use]
    pub fn css_variable(&self) -> String {
        self.variable
            .clone()
            .unwrap_or_else(|| format!("--font-{}", self.family.to_lowercase().replace(' ', "-")))
    }

    /// Render `@font-face` CSS rule.
    #[must_use]
    pub fn generate_font_face_css(&self) -> String {
        let safe_family = self.family.replace(['\'', '"', '\\', '<', '>'], "");
        let safe_src = self.src.replace(['\'', '"', '\\', '<', '>'], "");
        let var_name = self.css_variable();
        format!(
            "@font-face {{\n  font-family: '{safe_family}';\n  src: url('{safe_src}') format('woff2');\n  font-weight: {};\n  font-style: {};\n  font-display: {};\n}}\n:root {{\n  {var_name}: '{safe_family}', sans-serif;\n}}",
            self.weight,
            self.style,
            self.display.as_str()
        )
    }

    /// Generate HTML elements for `<Head>`.
    #[must_use]
    pub fn to_head_nodes(&self) -> Vec<Node> {
        let mut nodes = Vec::new();

        if self.preload {
            nodes.push(
                Element::new("link")
                    .attr("rel", "preload")
                    .attr("href", &self.src)
                    .attr("as", "font")
                    .attr("type", "font/woff2")
                    .attr("crossorigin", "")
                    .into_node(),
            );
        }

        nodes.push(
            Element::new("style")
                .child(Node::raw(self.generate_font_face_css()))
                .into_node(),
        );

        nodes
    }
}

impl Component for LocalFont {
    fn render(&self) -> Node {
        let nodes = self.to_head_nodes();
        Node::Fragment(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_google_font_url_and_nodes() {
        let font = GoogleFont::new("Inter")
            .weights(&[400, 700])
            .display(FontDisplay::Swap);

        let url = font.fonts_api_url();
        assert!(url.contains("Inter:wght@400;700"));
        assert!(url.contains("display=swap"));

        let nodes = font.to_head_nodes();
        assert_eq!(nodes.len(), 4); // preconnect x 2, link, style

        let rendered = render_to_string(&Node::Fragment(nodes));
        assert!(rendered.contains("https://fonts.googleapis.com"));
        assert!(rendered.contains("https://fonts.gstatic.com"));
        assert!(rendered.contains("size-adjust: 104.5%"));
        assert!(rendered.contains("--font-inter"));
    }

    #[test]
    fn test_local_font_preload() {
        let font = LocalFont::new("GeistSans", "/fonts/geist.woff2")
            .weight("400 800")
            .preload(true);

        let nodes = font.to_head_nodes();
        assert_eq!(nodes.len(), 2); // link preload, style

        let rendered = render_to_string(&Node::Fragment(nodes));
        assert!(rendered.contains("rel=\"preload\""));
        assert!(rendered.contains("as=\"font\""));
        assert!(rendered.contains("url('/fonts/geist.woff2')"));
        assert!(rendered.contains("--font-geistsans"));
    }
}
