//! HTML escaping utilities.
//!
//! All text content and attribute values are escaped before rendering to prevent
//! cross-site scripting (XSS) attacks. The escaping functions handle the five
//! characters that are meaningful in HTML: `&`, `<`, `>`, `"`, and `'`.

/// Escape a string for use as HTML text content.
///
/// Replaces `&`, `<`, `>`, `"`, and `'` with their HTML entity equivalents.
///
/// # Examples
/// ```
/// use yav_core::escape_html;
/// assert_eq!(escape_html("<script>alert('xss')</script>"),
///            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
/// ```
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
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
    output
}

/// Escape a string for use inside an HTML attribute value (double-quoted).
///
/// Same as [`escape_html`] but also handles characters that are problematic
/// in attribute contexts.
///
/// # Examples
/// ```
/// use yav_core::escape_attribute;
/// assert_eq!(escape_attribute(r#"hello "world""#),
///            "hello &quot;world&quot;");
/// ```
pub fn escape_attribute(input: &str) -> String {
    // For double-quoted attributes, the same escaping applies.
    escape_html(input)
}

/// Escape a URL for use in `href` or `src` attributes.
///
/// Rejects `javascript:` and `data:` URLs (case-insensitive) to prevent XSS.
/// Returns `None` for dangerous URLs.
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
    if trimmed.chars().any(|c| c.is_control()) {
        let sanitized_scheme: String = trimmed
            .chars()
            .filter(|c| !c.is_control() && !c.is_whitespace())
            .take(64)
            .collect();
        let lower_scheme = sanitized_scheme.to_lowercase();
        if lower_scheme.starts_with("javascript:")
            || lower_scheme.starts_with("vbscript:")
            || (lower_scheme.starts_with("data:")
                && !lower_scheme.starts_with("data:image/")
                && !lower_scheme.starts_with("data:font/"))
        {
            return None;
        }
    }

    let lower = trimmed.to_lowercase();

    // Block dangerous URL schemes (javascript:, vbscript:, and non-media data: URLs)
    if lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || (lower.starts_with("data:")
            && !lower.starts_with("data:image/")
            && !lower.starts_with("data:font/"))
    {
        return None;
    }

    Some(escape_attribute(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_basic() {
        assert_eq!(escape_html("hello"), "hello");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
    }

    #[test]
    fn escape_html_xss() {
        assert_eq!(
            escape_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_html_quotes() {
        assert_eq!(escape_html(r#"say "hello""#), "say &quot;hello&quot;");
        assert_eq!(escape_html("it's"), "it&#x27;s");
    }

    #[test]
    fn escape_attribute_basic() {
        assert_eq!(
            escape_attribute(r#"foo "bar" baz"#),
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
        assert_eq!(escape_url("vbscript:msgbox"), None);
    }

    #[test]
    fn escape_empty() {
        assert_eq!(escape_html(""), "");
        assert_eq!(escape_attribute(""), "");
    }

    #[test]
    fn escape_no_changes_needed() {
        let safe = "Hello World 123";
        assert_eq!(escape_html(safe), safe);
    }
}
