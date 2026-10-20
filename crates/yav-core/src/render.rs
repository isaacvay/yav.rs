//! HTML rendering engine.
//!
//! Converts a [`Node`] tree into a valid HTML string with proper escaping,
//! attribute serialization, and void element handling.

use crate::escape::{escape_attribute, escape_html};
use crate::node::{AttributeValue, Node};

/// Render a [`Node`] tree to an HTML string.
///
/// All text content is automatically HTML-escaped. Attribute values are
/// attribute-escaped. Void elements are self-closed. Boolean attributes
/// that are `false` are omitted.
///
/// # Examples
/// ```
/// use yav_core::{Element, Node, render_to_string};
///
/// let node = Node::Element(
///     Element::new("div")
///         .attr("class", "container")
///         .child(Node::text("Hello, world!"))
/// );
/// assert_eq!(render_to_string(&node), r#"<div class="container">Hello, world!</div>"#);
/// ```
pub fn render_to_string(node: &Node) -> String {
    let mut buf = String::with_capacity(1024);
    render_node(node, &mut buf);
    buf
}

const MAX_RENDER_DEPTH: usize = 512;

/// Render a [`Node`] tree into an existing buffer.
pub fn render_to_buf(node: &Node, buf: &mut String) {
    render_node_depth(node, buf, 0);
}

fn render_node(node: &Node, buf: &mut String) {
    render_node_depth(node, buf, 0);
}

fn render_node_depth(node: &Node, buf: &mut String, depth: usize) {
    if depth > MAX_RENDER_DEPTH {
        return;
    }
    match node {
        Node::Element(el) => render_element_depth(el, buf, depth + 1),
        Node::Text(text) => buf.push_str(&escape_html(text)),
        Node::Raw(html) => buf.push_str(html),
        Node::Static(s) => buf.push_str(s),
        Node::Fragment(children) => {
            for child in children {
                render_node_depth(child, buf, depth + 1);
            }
        }
        Node::Empty => {}
    }
}

fn is_url_attribute(name: &str) -> bool {
    matches!(
        name,
        "href" | "src" | "action" | "formaction" | "xlink:href" | "poster" | "data"
    )
}

fn is_valid_tag_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
}

fn is_valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && !name.chars().any(|c| {
            c.is_ascii_whitespace()
                || c.is_control()
                || c == '"'
                || c == '\''
                || c == '>'
                || c == '/'
                || c == '='
        })
}

fn render_element_depth(el: &crate::node::Element, buf: &mut String, depth: usize) {
    let safe_tag = if is_valid_tag_name(&el.tag) {
        el.tag.as_ref()
    } else {
        "div"
    };

    // Opening tag
    buf.push('<');
    buf.push_str(safe_tag);

    // Attributes
    for attr in &el.attributes {
        if !is_valid_attribute_name(&attr.name) {
            continue;
        }
        match &attr.value {
            AttributeValue::String(val) => {
                buf.push(' ');
                buf.push_str(&attr.name);
                buf.push_str("=\"");
                if is_url_attribute(&attr.name) {
                    if let Some(safe_url) = crate::escape::escape_url(val) {
                        buf.push_str(&safe_url);
                    }
                } else {
                    buf.push_str(&escape_attribute(val));
                }
                buf.push('"');
            }
            AttributeValue::Static(val) => {
                buf.push(' ');
                buf.push_str(&attr.name);
                buf.push_str("=\"");
                if is_url_attribute(&attr.name) {
                    if let Some(safe_url) = crate::escape::escape_url(val) {
                        buf.push_str(&safe_url);
                    }
                } else {
                    buf.push_str(&escape_attribute(val));
                }
                buf.push('"');
            }
            AttributeValue::Bool(true) => {
                buf.push(' ');
                buf.push_str(&attr.name);
            }
            AttributeValue::Bool(false) => {
                // Omit false boolean attributes entirely
            }
            AttributeValue::None => {
                buf.push(' ');
                buf.push_str(&attr.name);
            }
        }
    }

    if el.self_closing && crate::node::is_void_element(safe_tag) {
        buf.push_str(" />");
        return;
    }

    buf.push('>');

    // Children
    for child in &el.children {
        render_node_depth(child, buf, depth + 1);
    }

    // Closing tag
    buf.push_str("</");
    buf.push_str(safe_tag);
    buf.push('>');
}

/// Wrap rendered body content in a full HTML5 document shell with default `"en"` language.
///
/// # Arguments
/// * `title` — the `<title>` content
/// * `head_extra` — additional HTML to inject into `<head>` (CSS links, meta tags, etc.)
/// * `body` — the body content (already rendered HTML)
/// * `body_end` — scripts or other content to inject before `</body>`
pub fn document_shell(title: &str, head_extra: &str, body: &str, body_end: &str) -> String {
    document_shell_with_lang(title, head_extra, body, body_end, "en")
}

/// Wrap rendered body content in a full HTML5 document shell with custom `lang` attribute.
///
/// # Arguments
/// * `title` — the `<title>` content
/// * `head_extra` — additional HTML to inject into `<head>` (CSS links, meta tags, etc.)
/// * `body` — the body content (already rendered HTML)
/// * `body_end` — scripts or other content to inject before `</body>`
/// * `lang` — the language code (e.g. `"en"`, `"fr"`, `"es"`)
pub fn document_shell_with_lang(
    title: &str,
    head_extra: &str,
    body: &str,
    body_end: &str,
    lang: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
{head_extra}</head>
<body>
{body}
{body_end}</body>
</html>"#,
        lang = escape_attribute(lang),
        title = escape_html(title),
        head_extra = head_extra,
        body = body,
        body_end = body_end,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Element, Node};

    #[test]
    fn render_text() {
        let node = Node::text("Hello <world>");
        assert_eq!(render_to_string(&node), "Hello &lt;world&gt;");
    }

    #[test]
    fn render_raw() {
        let node = Node::raw("<b>bold</b>");
        assert_eq!(render_to_string(&node), "<b>bold</b>");
    }

    #[test]
    fn render_static() {
        let node = Node::static_str(r#"<div class="hero">Static Content</div>"#);
        assert_eq!(
            render_to_string(&node),
            r#"<div class="hero">Static Content</div>"#
        );
    }

    #[test]
    fn render_empty() {
        let node = Node::empty();
        assert_eq!(render_to_string(&node), "");
    }

    #[test]
    fn render_simple_element() {
        let node = Node::Element(Element::new("div"));
        assert_eq!(render_to_string(&node), "<div></div>");
    }

    #[test]
    fn render_element_with_text() {
        let node = Node::Element(Element::new("p").child(Node::text("Hello")));
        assert_eq!(render_to_string(&node), "<p>Hello</p>");
    }

    #[test]
    fn render_element_with_attributes() {
        let node = Node::Element(
            Element::new("a")
                .attr("href", "/about")
                .attr("class", "link"),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<a href="/about" class="link"></a>"#
        );
    }

    #[test]
    fn render_void_element() {
        let node = Node::Element(Element::new("br"));
        assert_eq!(render_to_string(&node), "<br />");
    }

    #[test]
    fn render_non_void_element_marked_self_closing() {
        let mut button = Element::new("button");
        button.self_closing = true;
        assert_eq!(
            render_to_string(&Node::Element(button)),
            "<button></button>"
        );

        let mut div = Element::new("div");
        div.self_closing = true;
        assert_eq!(render_to_string(&Node::Element(div)), "<div></div>");
    }

    #[test]
    fn render_document_shell_with_lang_test() {
        let html = document_shell_with_lang("Titre", "", "<p>Bonjour</p>", "", "fr");
        assert!(html.contains(r#"<html lang="fr">"#));
        assert!(html.contains("<title>Titre</title>"));
    }

    #[test]
    fn render_input_with_attrs() {
        let node = Node::Element(
            Element::new("input")
                .attr("type", "text")
                .attr("name", "query")
                .bool_attr("required", true),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<input type="text" name="query" required />"#
        );
    }

    #[test]
    fn render_bool_attr_false_omitted() {
        let node = Node::Element(Element::new("input").bool_attr("disabled", false));
        assert_eq!(render_to_string(&node), "<input />");
    }

    #[test]
    fn render_nested_elements() {
        let node = Node::Element(
            Element::new("div")
                .attr("class", "outer")
                .child(Node::Element(
                    Element::new("span").child(Node::text("inner")),
                )),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<div class="outer"><span>inner</span></div>"#
        );
    }

    #[test]
    fn render_fragment() {
        let node = Node::Fragment(vec![Node::text("one"), Node::text(" "), Node::text("two")]);
        assert_eq!(render_to_string(&node), "one two");
    }

    #[test]
    fn render_attribute_escaping() {
        let node = Node::Element(Element::new("div").attr("data-value", r#"say "hello" & 'bye'"#));
        assert_eq!(
            render_to_string(&node),
            r#"<div data-value="say &quot;hello&quot; &amp; &#x27;bye&#x27;"></div>"#
        );
    }

    #[test]
    fn render_xss_prevention() {
        let node =
            Node::Element(Element::new("p").child(Node::text("<script>alert('xss')</script>")));
        let html = render_to_string(&node);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn render_document_shell() {
        let html = document_shell(
            "My App",
            r#"<link rel="stylesheet" href="/style.css" />"#,
            "<h1>Hello</h1>",
            "",
        );
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>My App</title>"));
        assert!(html.contains("style.css"));
        assert!(html.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn render_complex_tree() {
        let node = Node::Element(
            Element::new("main")
                .attr("class", "container")
                .child(Node::Element(
                    Element::new("nav")
                        .child(Node::Element(
                            Element::new("a")
                                .attr("href", "/")
                                .child(Node::text("Home")),
                        ))
                        .child(Node::Element(
                            Element::new("a")
                                .attr("href", "/about")
                                .child(Node::text("About")),
                        )),
                ))
                .child(Node::Element(
                    Element::new("section")
                        .child(Node::Element(
                            Element::new("h1").child(Node::text("Welcome")),
                        ))
                        .child(Node::Element(
                            Element::new("p").child(Node::text("Build with Rust.")),
                        )),
                )),
        );

        let html = render_to_string(&node);
        assert!(html.starts_with(r#"<main class="container">"#));
        assert!(html.contains(r#"<a href="/">Home</a>"#));
        assert!(html.contains("<h1>Welcome</h1>"));
        assert!(html.ends_with("</main>"));
    }

    #[test]
    fn test_render_src_xss_protection() {
        let node = Node::Element(Element::new("iframe").attr("src", "javascript:alert(1)"));
        let html = render_to_string(&node);
        assert_eq!(html, r#"<iframe src=""></iframe>"#);

        let safe_img = Node::Element(
            Element::new("img")
                .attr("src", "https://example.com/logo.png")
                .attr("alt", "Logo"),
        );
        let safe_html = render_to_string(&safe_img);
        assert_eq!(
            safe_html,
            r#"<img src="https://example.com/logo.png" alt="Logo" />"#
        );
    }

    #[test]
    fn test_render_multimedia_and_svg_url_protection() {
        // xlink:href on SVG <a>
        let svg_link = Node::Element(
            Element::new("a")
                .attr("xlink:href", "javascript:alert('svg')")
                .child(Node::text("SVG link")),
        );
        let html1 = render_to_string(&svg_link);
        assert_eq!(html1, r#"<a xlink:href="">SVG link</a>"#);

        // video poster
        let video =
            Node::Element(Element::new("video").attr("poster", "javascript:alert('poster')"));
        let html2 = render_to_string(&video);
        assert_eq!(html2, r#"<video poster=""></video>"#);

        // object data
        let obj = Node::Element(Element::new("object").attr("data", "javascript:alert('data')"));
        let html3 = render_to_string(&obj);
        assert_eq!(html3, r#"<object data=""></object>"#);

        // Invalid attribute names with injection attempts must be dropped
        let injected_attr =
            Node::Element(Element::new("div").attr("foo\" onfocus=\"alert(1)", "bar"));
        let html4 = render_to_string(&injected_attr);
        assert_eq!(html4, "<div></div>");

        // Invalid tag names with injection attempts must fall back to <div>
        let injected_tag = Node::Element(
            Element::new("\"><script>alert(1)</script><div foo=\"").child(Node::text("test")),
        );
        let html5 = render_to_string(&injected_tag);
        assert_eq!(html5, "<div>test</div>");
    }
}
