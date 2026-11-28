//! HTML escaping utilities.
//!
//! All text content and attribute values are escaped before rendering to prevent
//! cross-site scripting (XSS) attacks. The escaping functions handle the five
//! characters that are meaningful in HTML: `&`, `<`, `>`, `"`, and `'`.

use std::borrow::Cow;

/// Characters that require HTML escaping.
const ESCAPE_CHARS: [char; 5] = ['&', '<', '>', '"', '\''];

/// Escape a string for use as HTML text content.
///
/// Replaces `&`, `<`, `>`, `"`, and `'` with their HTML entity equivalents.
/// Returns a borrowed `Cow` (zero allocation) when no escaping is needed.
///
/// # Examples
/// ```
/// use yav_core::escape_html;
/// assert_eq!(escape_html("<script>alert('xss')</script>").as_ref(),
///            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
/// ```
pub fn escape_html(input: &str) -> Cow<'_, str> {
    // Fast path: if no special characters, return borrowed reference (zero alloc)
    if !input.contains(ESCAPE_CHARS) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len() + input.len() / 8);
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            _ => output.push(ch),
        }
    }
    Cow::Owned(output)
}

/// Escape a string for use inside an HTML attribute value (double-quoted).
///
/// Same as [`escape_html`] but also handles characters that are problematic
/// in attribute contexts.
/// Returns a borrowed `Cow` (zero allocation) when no escaping is needed.
///
/// # Examples
/// ```
/// use yav_core::escape_attribute;
/// assert_eq!(escape_attribute(r#"hello "world""#).as_ref(),
///            "hello &quot;world&quot;");
/// ```
pub fn escape_attribute(input: &str) -> Cow<'_, str> {
    // For double-quoted attributes, the same escaping applies.
    escape_html(input)
}

/// Escape a URL for use in `href` or `src` attributes.
///
fn is_allowed_data_url(lower: &str) -> bool {
    if !lower.starts_with("data:") {
        return false;
    }
    // Disallow SVG as it can contain script tags or active event handlers
    if lower.starts_with("data:image/svg") {
        return false;
    }
    lower.starts_with("data:image/png")
        || lower.starts_with("data:image/jpeg")
        || lower.starts_with("data:image/jpg")
        || lower.starts_with("data:image/gif")
        || lower.starts_with("data:image/webp")
        || lower.starts_with("data:image/avif")
        || lower.starts_with("data:font/")
}

/// Strip control characters from a string, preserving all other content.
fn strip_control_chars(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

/// Rejects `javascript:` and dangerous `data:` URLs (case-insensitive) to prevent XSS.
/// Returns `None` for dangerous URLs.
///
/// Control characters are stripped from the returned URL to prevent injection bypasses.
///
/// # Examples
/// ```
/// use yav_core::escape_url;
/// assert_eq!(escape_url("https://example.com"), Some("https://example.com".to_string()));
/// assert_eq!(escape_url("javascript:alert(1)"), None);
/// ```
pub fn escape_url(url: &str) -> Option<String> {
    let trimmed = url.trim();

    // Reject or inspect URLs with embedded control characters (tabs, newlines, null bytes)
    let has_control = trimmed.chars().any(|c| c.is_control());
    if has_control {
        // Build a sanitized version with control chars removed for scheme checking
        let sanitized: String = trimmed
            .chars()
            .filter(|c| !c.is_control() && !c.is_whitespace())
            .collect();
        let lower_scheme = sanitized.to_lowercase();
        if lower_scheme.starts_with("javascript:")
            || lower_scheme.starts_with("vbscript:")
            || (lower_scheme.starts_with("data:") && !is_allowed_data_url(&lower_scheme))
        {
            return None;
        }
        // Strip control characters from output to prevent injection bypasses
        let cleaned = strip_control_chars(trimmed);
        return Some(escape_attribute(&cleaned).into_owned());
    }

    let lower = trimmed.to_lowercase();

    // Block dangerous URL schemes (javascript:, vbscript:, and non-media/SVG data: URLs)
    if lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || (lower.starts_with("data:") && !is_allowed_data_url(&lower))
    {
        return None;
    }

    Some(escape_attribute(trimmed).into_owned())
}

/// Escape a value for the `srcdoc` attribute of an `<iframe>`.
///
/// `srcdoc` is special: the HTML parser decodes character references in the
/// attribute value *before* the resulting string is parsed as a standalone HTML
/// document for the iframe. A single round of escaping is therefore not enough:
/// `srcdoc="&lt;script&gt;"` would make the browser decode the attribute to
/// `<script>` and then parse it as a live element.
///
/// To keep the content inert we first HTML-escape it (turning `<script>` into
/// `&lt;script&gt;`) and then re-escape every `&` into `&amp;`. The attribute
/// decoder then yields the text `&lt;script&gt;`, which the nested document
/// renders as literal text instead of an active element.
///
/// # Examples
/// ```
/// use yav_core::escape_srcdoc;
/// assert_eq!(
///     escape_srcdoc("<script>alert(1)</script>"),
///     "&amp;lt;script&amp;gt;alert(1)&amp;lt;/script&amp;gt;"
/// );
/// ```
pub fn escape_srcdoc(input: &str) -> String {
    let once = escape_html(input);
    // Second pass: re-escape the ampersands of the entities produced above so
    // the browser's attribute decoding cannot resurrect them into live markup.
    once.replace('&', "&amp;")
}

/// Escape and validate a `srcset` attribute value.
///
/// `srcset` is a comma-separated list of image candidates, each an URL followed
/// by an optional descriptor (`1x`, `100w`, ...). Every URL is passed through
/// [`escape_url`]; if any candidate is dangerous the whole list is rejected
/// (`None`) so the renderer omits the attribute entirely. Descriptors are
/// attribute-escaped so a quote cannot break out of the attribute, and the
/// list is rebuilt from the sanitized parts.
///
/// # Examples
/// ```
/// use yav_core::escape_srcset;
/// assert_eq!(
///     escape_srcset("a.png 1x, b.png 2x"),
///     Some("a.png 1x, b.png 2x".to_string())
/// );
/// assert_eq!(escape_srcset("javascript:alert(1) 1x"), None);
/// ```
pub fn escape_srcset(input: &str) -> Option<String> {
    let mut rebuilt: Vec<String> = Vec::new();
    for candidate in input.split(',') {
        let candidate = candidate.trim();
        let mut parts = candidate.splitn(2, char::is_whitespace);
        let url = parts.next().unwrap_or("");
        let descriptor = parts.next().unwrap_or("").trim();

        // A single dangerous URL invalidates the entire srcset list.
        let safe_url = escape_url(url)?;
        let mut entry = safe_url;
        if !descriptor.is_empty() {
            entry.push(' ');
            entry.push_str(&escape_attribute(descriptor));
        }
        rebuilt.push(entry);
    }
    Some(rebuilt.join(", "))
}

/// Sanitize a `style` attribute value for embedding in a double-quoted attribute.
///
/// A handful of CSS constructs can execute script or break out of the CSS
/// context and cannot be made safe by HTML escaping alone: legacy IE
/// `expression(...)`, `url(javascript:...)` (and any `javascript:` scheme),
/// `</style` (which can close an enclosing `<style>` element) and `@import`
/// loading an external origin. If any of these is detected the whole value is
/// refused (an empty string is returned) — the safest neutralisation. Otherwise
/// the value is escaped with [`escape_attribute`].
///
/// Detection runs on a lowercased, whitespace/control-stripped copy so that
/// obfuscations such as `url ( javaScript:` are still caught.
///
/// # Examples
/// ```
/// use yav_core::escape_style;
/// assert_eq!(escape_style("color: red;"), "color: red;");
/// assert_eq!(escape_style("background: url(javascript:alert(1))"), "");
/// ```
/// Escape a string for embedding inside a single- or double-quoted JavaScript string literal.
///
/// Escapes backslashes, single and double quotes, newlines, tabs, `<`/`>`/`&` (to prevent
/// `</script>` breakouts), and JS line separators U+2028 / U+2029.
pub fn escape_js_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04X}", c as u32);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape serialized JSON for safe embedding inside an HTML `<script>` element.
pub fn escape_script_json(raw_json: &str) -> String {
    let mut out = String::with_capacity(raw_json.len());
    for ch in raw_json.chars() {
        match ch {
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(ch),
        }
    }
    out
}

/// Strip CSS comments (`/* ... */`) and decode simple CSS backslash escapes (`\hh` or `\c`)
/// for normalized security inspection in [`escape_style`].
fn normalize_css_for_security_check(input: &str) -> String {
    let mut no_comments = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            } else {
                break;
            }
            continue;
        }
        let ch = input[i..].chars().next().unwrap();
        no_comments.push(ch);
        i += ch.len_utf8();
    }

    // Decode CSS backslash escapes (up to 6 hex digits + optional trailing whitespace, or literal char)
    let mut decoded = String::with_capacity(no_comments.len());
    let chars: Vec<char> = no_comments.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '\\' && idx + 1 < chars.len() {
            idx += 1;
            let start_hex = idx;
            while idx < chars.len() && idx - start_hex < 6 && chars[idx].is_ascii_hexdigit() {
                idx += 1;
            }
            if idx > start_hex {
                let hex: String = chars[start_hex..idx].iter().collect();
                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                    if let Some(c) = char::from_u32(code) {
                        decoded.push(c);
                    }
                }
                if idx < chars.len() && chars[idx].is_whitespace() {
                    idx += 1;
                }
                continue;
            }
            decoded.push(chars[idx]);
            idx += 1;
        } else {
            decoded.push(chars[idx]);
            idx += 1;
        }
    }

    decoded
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect()
}

/// Sanitize an untrusted HTML fragment using a strict allowlist of safe structural
/// tags and safe URL/attribute filtering.
///
/// Strips dangerous elements (`<script>`, `<style>`, `<iframe>`, `<object>`, `<embed>`,
/// `<base>`, `<meta>`, `<link>`, `<form>`, `<svg>`), strips `on*=` event handlers
/// (including obfuscated whitespace around `=`), and validates URL attributes (`href`, `src`)
/// through [`escape_url`].
pub fn sanitize_html_strict(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        if bytes[i] == b'<' {
            let rest = &html[i..];
            // Strip HTML comments
            if rest.starts_with("<!--") {
                if let Some(end_c) = rest.find("-->") {
                    i += end_c + 3;
                    continue;
                }
                break;
            }

            // Find closing '>' respecting single/double quoted attribute values
            let mut quote: Option<u8> = None;
            let mut end_rel = None;
            for (k, &b) in rest.as_bytes().iter().enumerate().skip(1) {
                if let Some(q) = quote {
                    if b == q {
                        quote = None;
                    }
                } else if b == b'"' || b == b'\'' {
                    quote = Some(b);
                } else if b == b'>' {
                    end_rel = Some(k);
                    break;
                }
            }

            let Some(end_k) = end_rel else {
                // Unterminated tag: escape remainder so it cannot inject active markup
                out.push_str(&escape_html(&html[i..]));
                break;
            };

            let raw_tag = &rest[1..end_k];
            let trimmed = raw_tag.trim();
            let is_closing = trimmed.starts_with('/');
            let name_part = trimmed.strip_prefix('/').unwrap_or(trimmed).trim_start();
            let tag_name: String = name_part
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':')
                .collect::<String>()
                .to_ascii_lowercase();

            // Strip dangerous container elements along with their inner content
            if !is_closing
                && matches!(
                    tag_name.as_str(),
                    "script"
                        | "style"
                        | "iframe"
                        | "object"
                        | "embed"
                        | "svg"
                        | "math"
                        | "noscript"
                        | "template"
                )
            {
                let close_pat = format!("</{tag_name}");
                let after_open = &html[i + end_k + 1..];
                let lower_after = after_open.to_ascii_lowercase();
                if let Some(c_pos) = lower_after.find(&close_pat) {
                    if let Some(gt_pos) = after_open[c_pos..].find('>') {
                        i = i + end_k + 1 + c_pos + gt_pos + 1;
                        continue;
                    }
                }
                i = i + end_k + 1;
                continue;
            }

            const SAFE_TAGS: &[&str] = &[
                "a",
                "abbr",
                "b",
                "blockquote",
                "br",
                "code",
                "dd",
                "del",
                "div",
                "dl",
                "dt",
                "em",
                "figcaption",
                "figure",
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "hr",
                "i",
                "img",
                "ins",
                "kbd",
                "li",
                "mark",
                "ol",
                "p",
                "pre",
                "q",
                "s",
                "samp",
                "section",
                "small",
                "span",
                "strong",
                "sub",
                "sup",
                "table",
                "tbody",
                "td",
                "tfoot",
                "th",
                "thead",
                "tr",
                "u",
                "ul",
            ];

            if !SAFE_TAGS.contains(&tag_name.as_str()) {
                // Drop unsafe tag (keeping text between open/close tags for benign ones like <font>/<base>)
                i += end_k + 1;
                continue;
            }

            if is_closing {
                out.push_str("</");
                out.push_str(&tag_name);
                out.push('>');
                i += end_k + 1;
                continue;
            }

            // Rebuild opening tag with allowlisted, sanitized attributes
            let attrs_raw = &name_part[tag_name.len()..];
            let self_close = attrs_raw.trim_end().ends_with('/');
            let attrs_str = attrs_raw.trim_end().strip_suffix('/').unwrap_or(attrs_raw);
            let sanitized_attrs = sanitize_tag_attributes(attrs_str);

            out.push('<');
            out.push_str(&tag_name);
            out.push_str(&sanitized_attrs);
            if self_close {
                out.push_str(" />");
            } else {
                out.push('>');
            }
            i += end_k + 1;
        } else {
            let next_lt = html[i..].find('<').map(|p| i + p).unwrap_or(len);
            out.push_str(&html[i..next_lt]);
            i = next_lt;
        }
    }
    out
}

fn sanitize_tag_attributes(attrs: &str) -> String {
    const SAFE_ATTRS: &[&str] = &[
        "href",
        "src",
        "alt",
        "title",
        "class",
        "id",
        "target",
        "rel",
        "width",
        "height",
        "colspan",
        "rowspan",
        "aria-label",
        "aria-hidden",
        "role",
    ];
    let mut out = String::new();
    let chars: Vec<char> = attrs.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '/') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let name_start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '/' {
            i += 1;
        }
        let attr_name: String = chars[name_start..i].iter().collect();
        let lower_name = attr_name.to_ascii_lowercase();

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        let mut attr_val: Option<String> = None;
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let q = chars[i];
                i += 1;
                let val_start = i;
                while i < chars.len() && chars[i] != q {
                    i += 1;
                }
                attr_val = Some(chars[val_start..i].iter().collect());
                if i < chars.len() {
                    i += 1;
                }
            } else {
                let val_start = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                attr_val = Some(chars[val_start..i].iter().collect());
            }
        }

        if lower_name.starts_with("on") {
            continue;
        }
        let is_allowed = SAFE_ATTRS.contains(&lower_name.as_str())
            || (lower_name.starts_with("data-")
                && !lower_name.contains("yav")
                && lower_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-'));
        if !is_allowed {
            continue;
        }

        if let Some(raw_val) = attr_val {
            // Decode common numeric/named entities before URL validation
            let decoded_for_check = decode_basic_html_entities(&raw_val);
            if lower_name == "href" || lower_name == "src" {
                let Some(safe_url) = escape_url(&decoded_for_check) else {
                    continue;
                };
                out.push(' ');
                out.push_str(&lower_name);
                out.push_str("=\"");
                out.push_str(&safe_url);
                out.push('"');
            } else {
                out.push(' ');
                out.push_str(&lower_name);
                out.push_str("=\"");
                out.push_str(&escape_attribute(&raw_val));
                out.push('"');
            }
        } else {
            out.push(' ');
            out.push_str(&lower_name);
        }
    }
    out
}

fn decode_basic_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = input[i..].find(';').filter(|&p| p <= 10) {
                let entity = &input[i + 1..i + semi];
                if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    if let Ok(code) = u32::from_str_radix(hex, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                            i += semi + 1;
                            continue;
                        }
                    }
                } else if let Some(dec) = entity.strip_prefix('#') {
                    if let Ok(code) = dec.parse::<u32>() {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                            i += semi + 1;
                            continue;
                        }
                    }
                } else if entity.eq_ignore_ascii_case("colon") {
                    out.push(':');
                    i += semi + 1;
                    continue;
                }
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

pub fn escape_style(input: &str) -> String {
    let normalized = normalize_css_for_security_check(input);

    const FORBIDDEN: &[&str] = &[
        "expression(",
        "url(javascript:",
        "javascript:",
        "vbscript:",
        "</style",
        "behavior:",
        "-moz-binding:",
    ];
    if FORBIDDEN.iter().any(|pat| normalized.contains(*pat)) {
        return String::new();
    }

    // `@import` is only dangerous with an external origin; same-origin relative
    // imports are left to the caller.
    if let Some(pos) = normalized.find("@import") {
        let tail = &normalized[pos + "@import".len()..];
        if tail.contains("http:")
            || tail.contains("https:")
            || tail.contains("//")
            || tail.contains("data:")
            || tail.contains("javascript:")
        {
            return String::new();
        }
    }

    escape_attribute(input).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_basic() {
        assert_eq!(escape_html("hello").as_ref(), "hello");
        assert_eq!(escape_html("a & b").as_ref(), "a &amp; b");
        assert_eq!(escape_html("<div>").as_ref(), "&lt;div&gt;");
    }

    #[test]
    fn escape_html_xss() {
        assert_eq!(
            escape_html("<script>alert('xss')</script>").as_ref(),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_html_quotes() {
        assert_eq!(
            escape_html(r#"say "hello""#).as_ref(),
            "say &quot;hello&quot;"
        );
        assert_eq!(escape_html("it's").as_ref(), "it&#x27;s");
    }

    #[test]
    fn escape_html_zero_alloc_fast_path() {
        let safe = "Hello World 123";
        let result = escape_html(safe);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "safe strings should not allocate"
        );
        assert_eq!(result.as_ref(), safe);
    }

    #[test]
    fn escape_attribute_basic() {
        assert_eq!(
            escape_attribute(r#"foo "bar" baz"#).as_ref(),
            "foo &quot;bar&quot; baz"
        );
    }

    #[test]
    fn escape_url_safe() {
        assert_eq!(
            escape_url("https://example.com"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            escape_url("/about?q=hello"),
            Some("/about?q=hello".to_string())
        );
    }

    #[test]
    fn escape_url_dangerous() {
        assert_eq!(escape_url("javascript:alert(1)"), None);
        assert_eq!(escape_url("JAVASCRIPT:void(0)"), None);
        assert_eq!(escape_url("java\tscript:alert(1)"), None);
        assert_eq!(escape_url("java\nscript:alert(1)"), None);
        assert_eq!(escape_url("data:text/html,<h1>Hi</h1>"), None);
        assert_eq!(escape_url("data:image/svg+xml,<svg onload=alert(1)>"), None);
        assert_eq!(escape_url("vbscript:msgbox"), None);
    }

    #[test]
    fn escape_url_control_chars_stripped() {
        // A safe URL with embedded control chars should return without those chars
        let url_with_null = "https://example.com/path\x00?q=1";
        let result = escape_url(url_with_null);
        assert!(result.is_some());
        let safe = result.unwrap();
        assert!(
            !safe.contains('\x00'),
            "control chars must be stripped from output"
        );
        assert!(safe.contains("https://example.com/path?q=1"));
    }

    #[test]
    fn escape_url_allowed_data_url() {
        let png = "data:image/png;base64,iVBOR";
        assert!(escape_url(png).is_some());
    }

    #[test]
    fn escape_url_empty() {
        assert_eq!(escape_url(""), Some(String::new()));
        assert_eq!(escape_url("   "), Some(String::new()));
    }

    #[test]
    fn escape_empty() {
        assert_eq!(escape_html("").as_ref(), "");
        assert_eq!(escape_attribute("").as_ref(), "");
    }

    #[test]
    fn escape_no_changes_needed() {
        let safe = "Hello World 123";
        assert_eq!(escape_html(safe).as_ref(), safe);
    }

    #[test]
    fn escape_srcdoc_double_escapes_markup() {
        let escaped = escape_srcdoc("<script>alert(1)</script>");
        assert_eq!(
            escaped,
            "&amp;lt;script&amp;gt;alert(1)&amp;lt;/script&amp;gt;"
        );
        assert!(!escaped.contains("<script>"));
        assert!(!escaped.contains('"'));
    }

    #[test]
    fn escape_srcdoc_plain_and_ampersand() {
        assert_eq!(escape_srcdoc("Hello World"), "Hello World");
        assert_eq!(escape_srcdoc("a & b"), "a &amp;amp; b");
    }

    #[test]
    fn escape_srcset_safe_candidates() {
        assert_eq!(
            escape_srcset("a.png 1x, b.png 2x, wide.png 100w"),
            Some("a.png 1x, b.png 2x, wide.png 100w".to_string())
        );
    }

    #[test]
    fn escape_srcset_rejects_any_dangerous_url() {
        assert_eq!(escape_srcset("javascript:alert(1) 1x"), None);
        assert_eq!(escape_srcset("safe.png 1x, javascript:alert(1) 2x"), None);
        assert_eq!(escape_srcset("vbscript:msgbox 1x"), None);
    }

    #[test]
    fn escape_srcset_escapes_descriptor_breakout() {
        let escaped = escape_srcset("img.png 1x\" onload=\"alert(1)").unwrap();
        assert!(!escaped.contains('"'));
        assert!(escaped.contains("img.png"));
    }

    #[test]
    fn escape_style_refuses_dangerous_constructs() {
        assert_eq!(escape_style("width: expression(alert(1))"), "");
        assert_eq!(escape_style("background: url(javascript:alert(1))"), "");
        assert_eq!(escape_style("color: red; } </style><script>"), "");
        assert_eq!(
            escape_style("@import url('https://evil.example/x.css');"),
            ""
        );
        assert_eq!(escape_style("@import \"//evil.example/x.css\";"), "");
    }

    #[test]
    fn escape_style_preserves_and_escapes_safe_css() {
        assert_eq!(
            escape_style("color: red; font-size: 12px;"),
            "color: red; font-size: 12px;"
        );
        assert_eq!(
            escape_style("content: \"a & b\""),
            "content: &quot;a &amp; b&quot;"
        );
        // Same-origin relative imports are allowed (no external origin).
        assert!(escape_style("@import url('theme.css');").contains("@import"));
    }
}
