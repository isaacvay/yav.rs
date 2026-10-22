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
}
