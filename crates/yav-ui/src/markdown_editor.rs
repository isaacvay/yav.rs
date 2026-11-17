//! Dual-Pane Live Markdown Editor Component.
//!
//! Provides a responsive split-view editor featuring a rich markdown toolbar,
//! synchronized preview pane, and zero-JS instant editing capabilities.
//!
//! # Security
//!
//! The preview pane renders HTML, which is frequently produced from untrusted
//! Markdown. [`sanitize_preview_html`] applies a strict tag/attribute
//! allowlist before the preview is passed to `Node::raw`. Use
//! [`MarkdownEditor::preview_html_trusted`] only for markup that has already
//! been sanitised by a trusted renderer.

use yav_core::{Component, Element, Node};

/// HTML tags allowed inside a rendered Markdown preview.
///
/// Deliberately excludes `script`, `style`, `iframe`, `object`, `embed`,
/// `svg`, `math`, `form`, `input`, `link`, `meta`, `base` and every other
/// active-content element.
const ALLOWED_PREVIEW_TAGS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "strong",
    "b",
    "em",
    "i",
    "u",
    "s",
    "del",
    "ins",
    "sub",
    "sup",
    "mark",
    "small",
    "blockquote",
    "pre",
    "code",
    "kbd",
    "samp",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "th",
    "td",
    "caption",
    "colgroup",
    "col",
    "a",
    "img",
    "br",
    "hr",
    "div",
    "span",
];

/// Attributes allowed on every allowed tag.
const ALLOWED_PREVIEW_ATTRS: &[&str] = &["class", "title", "alt", "id"];

/// HTML tags that do not have a closing tag.
const VOID_PREVIEW_TAGS: &[&str] = &["br", "hr", "img", "col"];

fn is_allowed_tag(name: &str) -> bool {
    ALLOWED_PREVIEW_TAGS.contains(&name)
}

/// Escape a disallowed tag so it is displayed as text rather than parsed as markup.
fn escape_disallowed(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Locate the `>` that closes a tag starting at `start`, ignoring `>` inside
/// quoted attribute values. Returns `None` for an unterminated tag.
fn find_tag_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut quote = 0u8;
    let mut i = start + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract `(name, value)` pairs from an attribute substring.
fn parse_preview_attrs(s: &str) -> Vec<(String, String)> {
    let bytes = s.as_bytes();
    let mut result = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'/' {
            break;
        }
        let key_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
        {
            i += 1;
        }
        if i == key_start {
            i += 1;
            continue;
        }
        let key = s[key_start..i].to_string();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let value_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value = s[value_start..i].to_string();
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let value_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                value = s[value_start..i].to_string();
            }
        }
        result.push((key, value));
    }
    result
}

/// Reject entity-encoded schemes such as `&#106;avascript:` which a browser
/// would decode back into an executable URL after our scheme check.
fn contains_encoded_scheme(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("&#")
        || lower.contains("&colon;")
        || lower.contains("&tab;")
        || lower.contains("&newline;")
        || lower.contains("&sol;")
}

fn safe_url(value: &str) -> Option<String> {
    if contains_encoded_scheme(value) {
        return None;
    }
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("data:")
    {
        return None;
    }
    // Reject control characters (including embedded tabs/newlines) outright.
    if trimmed.chars().any(char::is_control) {
        return None;
    }
    // Attribute escaping: reuse the framework's HTML escaper.
    Some(yav_core::escape_attribute(trimmed).into_owned())
}

/// Rewrite a single `<tag …>` into a safe form, or return `None` when the
/// whole tag must be shown as escaped text.
fn sanitize_tag(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('<')?.strip_suffix('>')?.trim();
    // Comments, doctypes, CDATA and processing instructions are never allowed.
    if inner.starts_with('!') || inner.starts_with('?') {
        return None;
    }
    let (is_closing, body) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest.trim()),
        None => (false, inner),
    };
    let trimmed_body = body.trim_end();
    let self_closing = !is_closing && trimmed_body.ends_with('/');
    let body = trimmed_body.trim_end_matches('/').trim_end();
    let name_end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(body.len());
    let name = &body[..name_end];
    if name.is_empty() {
        return None;
    }
    let lower_name = name.to_ascii_lowercase();
    if !is_allowed_tag(&lower_name) {
        return None;
    }
    if is_closing {
        if VOID_PREVIEW_TAGS.contains(&lower_name.as_str()) {
            return Some(String::new());
        }
        return Some(format!("</{lower_name}>"));
    }

    let mut out = format!("<{lower_name}");
    for (key, value) in parse_preview_attrs(&body[name_end..]) {
        let lower_key = key.to_ascii_lowercase();
        // Event handlers and active-content attributes are always dropped.
        if lower_key.starts_with("on")
            || matches!(
                lower_key.as_str(),
                "style" | "srcdoc" | "formaction" | "srcset" | "xlink:href"
            )
        {
            continue;
        }
        let is_href = lower_name == "a" && lower_key == "href";
        let is_img_src = lower_name == "img" && lower_key == "src";
        let is_url_attr = is_href || is_img_src;
        if !ALLOWED_PREVIEW_ATTRS.contains(&lower_key.as_str()) && !is_url_attr {
            continue;
        }
        let escaped = if is_url_attr {
            match safe_url(&value) {
                Some(url) => url,
                None => continue,
            }
        } else {
            yav_core::escape_attribute(&value).into_owned()
        };
        out.push_str(&format!(" {lower_key}=\"{escaped}\""));
    }
    if self_closing {
        out.push('/');
    }
    out.push('>');
    Some(out)
}

/// Sanitize HTML produced by a Markdown renderer before it is embedded in a
/// page with [`Node::raw`].
///
/// This is an allowlist sanitizer: only the tags in `ALLOWED_PREVIEW_TAGS`
/// survive, only safe attributes are kept, event handlers are dropped, and
/// `javascript:` / `vbscript:` / `data:` URLs (including entity-encoded
/// variants) are rejected. Everything else is displayed as escaped text.
///
/// ```
/// use yav_ui::sanitize_preview_html;
/// let clean = sanitize_preview_html("<p onclick=\"evil()\">Hi</p><script>alert(1)</script>");
/// assert!(!clean.contains("<script"));
/// assert!(!clean.contains("onclick"));
/// assert!(clean.contains("<p>Hi</p>"));
/// ```
pub fn sanitize_preview_html(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            let ch = input[i..]
                .chars()
                .next()
                .expect("byte index is a char boundary");
            // A stray `>` in text is harmless, but escaping it makes the output
            // unambiguous (and keeps `&` entities from Markdown intact).
            if ch == '>' {
                out.push_str("&gt;");
            } else {
                out.push(ch);
            }
            i += ch.len_utf8();
            continue;
        }
        let Some(end) = find_tag_end(input, i) else {
            out.push_str(&escape_disallowed(&input[i..]));
            break;
        };
        let raw = &input[i..=end];
        match sanitize_tag(raw) {
            Some(safe) => out.push_str(&safe),
            None => out.push_str(&escape_disallowed(raw)),
        }
        i = end + 1;
    }
    out
}

/// Configurable Markdown Editor component with live split-view preview.
#[derive(Debug, Clone)]
pub struct MarkdownEditor {
    pub name: String,
    pub initial_value: String,
    pub height_px: u32,
    /// Raw preview HTML. Sanitised at render time unless `preview_is_trusted`.
    pub preview_html: Option<String>,
    /// When true, `preview_html` is embedded verbatim. Only set this for HTML
    /// that has already been sanitised by a trusted renderer.
    pub preview_is_trusted: bool,
    pub endpoint: Option<String>,
}

impl MarkdownEditor {
    /// Create a new Markdown editor with input field name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initial_value: String::new(),
            height_px: 420,
            preview_html: None,
            preview_is_trusted: false,
            endpoint: None,
        }
    }

    #[must_use]
    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.initial_value = value.into();
        self
    }

    #[must_use]
    pub fn height_px(mut self, height: u32) -> Self {
        self.height_px = height;
        self
    }

    /// Set the rendered preview HTML. It is sanitised with an allowlist before
    /// being embedded, so untrusted Markdown output is safe to pass here.
    #[must_use]
    pub fn preview_html(mut self, html: impl Into<String>) -> Self {
        self.preview_html = Some(html.into());
        self.preview_is_trusted = false;
        self
    }

    /// Embed preview HTML **without sanitisation**.
    ///
    /// # Safety
    ///
    /// Only use this with markup from a trusted, already-sanitised renderer.
    /// Passing untrusted user content here is an XSS vulnerability.
    #[must_use]
    pub fn preview_html_trusted(mut self, html: impl Into<String>) -> Self {
        self.preview_html = Some(html.into());
        self.preview_is_trusted = true;
        self
    }

    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }
}

impl Component for MarkdownEditor {
    fn render(&self) -> Node {
        let preview_id = format!("{}-preview", self.name);

        // Toolbar with formatting shortcut buttons
        let toolbar = Element::new("div")
            .attr(
                "class",
                "flex items-center gap-1 border-b border-slate-800 bg-slate-900/80 px-3 py-2 text-xs select-none",
            )
            .child(render_tool_btn("B", "Bold", "**"))
            .child(render_tool_btn("I", "Italic", "*"))
            .child(render_tool_btn("H1", "Heading", "# "))
            .child(render_tool_btn("H2", "Subheading", "## "))
            .child(render_tool_btn("“ ”", "Quote", "> "))
            .child(render_tool_btn("<>", "Code", "```\n\n```"))
            .child(render_tool_btn("🔗", "Link", "[Link text](https://)"))
            .child(render_tool_btn("⊞", "Table", "| Header 1 | Header 2 |\n| --- | --- |\n| Cell 1 | Cell 2 |"))
            .child(
                Element::new("div")
                    .attr("class", "ml-auto text-slate-500 font-mono text-[11px]")
                    .child(Node::text("Markdown + KaTeX")),
            );

        // Left pane: Raw Markdown Textarea
        let mut textarea = Element::new("textarea")
            .attr("name", self.name.clone())
            .attr("id", format!("{}-input", self.name))
            .attr(
                "class",
                "w-full h-full bg-slate-950 p-4 font-mono text-xs text-slate-200 outline-none resize-none leading-relaxed border-r border-slate-800",
            )
            .attr("placeholder", "# Écrivez votre document en Markdown ici...")
            .child(Node::text(self.initial_value.clone()));

        if let Some(ep) = &self.endpoint {
            textarea = textarea
                .attr("yav:post", ep.clone())
                .attr("yav:target", format!("#{preview_id}"))
                .attr("yav:swap", "innerHTML");
        }

        let left_pane = Element::new("div")
            .attr("class", "flex-1 min-w-0")
            .child(textarea);

        // Right pane: Rendered HTML Preview. Caller-supplied HTML is sanitised
        // by default — `Node::raw` is only reached with allowlisted markup.
        let preview_content = match &self.preview_html {
            Some(html) if self.preview_is_trusted => Node::raw(html.clone()),
            Some(html) => Node::raw(sanitize_preview_html(html)),
            None => Node::raw(
                r#"<p class="text-slate-500 italic">La prévisualisation apparaîtra ici en temps réel...</p>"#,
            ),
        };

        let right_pane = Element::new("div")
            .attr("id", preview_id)
            .attr(
                "class",
                "flex-1 min-w-0 bg-slate-900/40 p-4 overflow-y-auto prose prose-invert prose-sm max-w-none",
            )
            .child(preview_content);

        let split_body = Element::new("div")
            .attr("class", "flex divide-x divide-slate-800")
            .attr("style", format!("height: {}px;", self.height_px))
            .child(left_pane)
            .child(right_pane);

        Node::Element(
            Element::new("div")
                .attr(
                    "class",
                    "yav-markdown-editor rounded-xl border border-slate-800 bg-slate-950 shadow-xl overflow-hidden",
                )
                .child(toolbar)
                .child(split_body),
        )
    }
}

fn render_tool_btn(label: &str, title: &str, insert_text: &str) -> Element {
    Element::new("button")
        .attr("type", "button")
        .attr("title", title)
        .attr("data-md-insert", insert_text)
        .attr(
            "class",
            "rounded px-2 py-1 font-mono text-slate-400 hover:bg-slate-800 hover:text-white transition-colors",
        )
        .child(Node::text(label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_markdown_editor_render() {
        let editor = MarkdownEditor::new("post_body")
            .initial_value("# Mon Titre\nTexte de test.")
            .preview_html("<h1>Mon Titre</h1><p>Texte de test.</p>");

        let html = render_to_string(&editor.render());
        assert!(html.contains("yav-markdown-editor"));
        assert!(html.contains("post_body-preview"));
        assert!(html.contains("<h1>Mon Titre</h1>"));
        assert!(html.contains("data-md-insert=\"# \""));
    }

    #[test]
    fn test_sanitizer_keeps_safe_markdown_html() {
        let clean = sanitize_preview_html(
            r##"<h1 class="title">Title</h1><p>Hello <strong>world</strong> &amp; friends</p><a href="https://example.com">link</a>"##,
        );
        assert!(clean.contains("<h1 class=\"title\">Title</h1>"));
        assert!(clean.contains("<strong>world</strong>"));
        assert!(clean.contains("&amp; friends"));
        assert!(clean.contains("href=\"https://example.com\""));
    }

    #[test]
    fn test_sanitizer_blocks_script_and_event_handlers() {
        let clean = sanitize_preview_html(
            r##"<p onclick="evil()">Hi</p><script>alert(1)</script><img src="x" onerror="alert(2)">"##,
        );
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("onclick"));
        assert!(!clean.contains("onerror"));
        assert!(clean.contains("&lt;script&gt;"));
        assert!(clean.contains("<p>Hi</p>"));
    }

    #[test]
    fn test_sanitizer_blocks_javascript_urls() {
        for payload in [
            r##"<a href="javascript:alert(1)">x</a>"##,
            r##"<a href="&#106;avascript:alert(1)">x</a>"##,
            r##"<a href="  JaVaScRiPt:alert(1)">x</a>"##,
            r##"<img src="data:text/html,<script>alert(1)</script>">"##,
        ] {
            let clean = sanitize_preview_html(payload);
            assert!(!clean.contains("javascript:"), "payload leaked: {payload}");
            assert!(!clean.contains("&#106;"), "entity leaked: {payload}");
            assert!(
                !clean.contains("data:text/html"),
                "data URL leaked: {payload}"
            );
        }
    }

    #[test]
    fn test_component_sanitizes_untrusted_preview() {
        let editor = MarkdownEditor::new("body")
            .preview_html("<p>ok</p><script>alert(document.cookie)</script>");
        let html = render_to_string(&editor.render());
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_trusted_escape_hatch_is_explicit() {
        let editor = MarkdownEditor::new("body").preview_html_trusted("<p>pre-sanitised</p>");
        let html = render_to_string(&editor.render());
        assert!(html.contains("<p>pre-sanitised</p>"));
    }

    #[test]
    fn test_unterminated_tag_is_escaped() {
        let clean = sanitize_preview_html("<p>hello<script");
        assert!(!clean.contains("<script"));
        assert!(clean.contains("&lt;script"));
    }
}
