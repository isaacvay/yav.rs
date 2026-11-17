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
pub fn escape_style(input: &str) -> String {
    let normalized: String = input
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();

    const FORBIDDEN: &[&str] = &["expression(", "url(javascript:", "javascript:", "</style"];
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
