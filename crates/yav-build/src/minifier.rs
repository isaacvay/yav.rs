//! Zero-Dependency CSS and HTML Minifier for Production Builds.
//!
//! Strips comments, collapses redundant whitespace, and shrinks CSS/HTML payloads
//! before Brotli/Gzip pre-compression while preserving `<pre>`, `<code>`, `<script>`,
//! and `<textarea>` whitespace semantics.

/// Minify a CSS stylesheet string by removing `/* ... */` comments and collapsing whitespace.
pub fn minify_css(css: &str) -> String {
    let mut no_comments = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        no_comments.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("*/") {
            rest = &after[end + 2..];
        } else {
            rest = "";
            break;
        }
    }
    no_comments.push_str(rest);

    let mut out = String::with_capacity(no_comments.len());
    let mut prev_space = false;
    let mut in_quote: Option<char> = None;
    let mut escaped = false;
    let mut in_block: usize = 0;
    let mut calc_depth: usize = 0;

    for ch in no_comments.chars() {
        if let Some(q) = in_quote {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                in_quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            if prev_space
                && !out.is_empty()
                && !out.ends_with(['{', '}', ':', ';', ',', '>', '(', ' '])
            {
                out.push(' ');
            }
            out.push(ch);
            prev_space = false;
            continue;
        }

        if ch.is_whitespace() {
            prev_space = true;
            continue;
        }

        // Track CSS math function nesting: calc(...), min(...), max(...), clamp(...)
        if ch == '(' {
            if out.ends_with("calc")
                || out.ends_with("min")
                || out.ends_with("max")
                || out.ends_with("clamp")
            {
                calc_depth += 1;
            }
            out.push(ch);
            prev_space = false;
            continue;
        }

        if ch == ')' {
            calc_depth = calc_depth.saturating_sub(1);
            out.push(ch);
            prev_space = false;
            continue;
        }

        // Inside calc(), CSS specifications strictly require spaces around '+' and '-' binary operators
        if calc_depth > 0
            && (ch == '+' || ch == '-')
            && out.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '%' || c == ')')
        {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            out.push(ch);
            out.push(' ');
            prev_space = false;
            continue;
        }

        if matches!(ch, '{' | '}' | ':' | ';' | ',' | '>') {
            if ch == '{' {
                in_block += 1;
            } else if ch == '}' {
                in_block = in_block.saturating_sub(1);
            } else if ch == ':'
                && in_block == 0
                && prev_space
                && !out.is_empty()
                && !out.ends_with(['{', '}', ',', '>', '(', ' '])
            {
                // Preserve space before ':' in CSS selectors (e.g. '.parent :first-child')
                out.push(' ');
            }
            out.push(ch);
            prev_space = false;
        } else {
            if prev_space
                && !out.is_empty()
                && !out.ends_with(['{', '}', ':', ';', ',', '>', '(', ' '])
            {
                out.push(' ');
            }
            out.push(ch);
            prev_space = false;
        }
    }

    out.replace(";}", "}")
}

fn is_tag_match(preview: &str, tag_name: &str) -> bool {
    if !preview.starts_with(tag_name) {
        return false;
    }
    match preview.as_bytes().get(tag_name.len()) {
        Some(&b) => b.is_ascii_whitespace() || b == b'>' || b == b'/',
        None => true,
    }
}

/// Minify an HTML document string by stripping `<!-- ... -->` comments and collapsing
/// inter-tag whitespace outside `<pre>`, `<code>`, `<textarea>`, and `<script>`.
pub fn minify_html(html: &str) -> String {
    let mut no_comments = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        no_comments.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        if let Some(end) = after.find("-->") {
            rest = &after[end + 3..];
        } else {
            rest = "";
            break;
        }
    }
    no_comments.push_str(rest);

    let mut out = String::with_capacity(no_comments.len());
    let mut prev_space = false;
    let mut preserve_depth: usize = 0;
    let chars: Vec<char> = no_comments.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '<' {
            let tag_preview: String = chars[i..(i + 14).min(chars.len())]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();

            if let Some(rest) = tag_preview.strip_prefix("</") {
                if is_tag_match(rest, "pre")
                    || is_tag_match(rest, "code")
                    || is_tag_match(rest, "textarea")
                    || is_tag_match(rest, "script")
                {
                    preserve_depth = preserve_depth.saturating_sub(1);
                }
            } else if let Some(rest) = tag_preview.strip_prefix('<') {
                if is_tag_match(rest, "pre")
                    || is_tag_match(rest, "code")
                    || is_tag_match(rest, "textarea")
                    || is_tag_match(rest, "script")
                {
                    preserve_depth += 1;
                }
            }
        }

        let ch = chars[i];
        if preserve_depth == 0 && ch.is_whitespace() {
            prev_space = true;
            i += 1;
            continue;
        }

        if prev_space && preserve_depth == 0 {
            if !out.ends_with('>') && ch != '<' {
                out.push(' ');
            }
            prev_space = false;
        }
        out.push(ch);
        i += 1;
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_css_and_html() {
        let raw_css = "/* header */\n.card {\n  color: #fff;\n  padding: 16px;\n}\n";
        assert_eq!(minify_css(raw_css), ".card{color:#fff;padding:16px}");

        let utf8_css = "/* émojis */\n.btn::after {\n  content: '🚀';\n}\n";
        assert_eq!(minify_css(utf8_css), ".btn::after{content:'🚀'}");

        let raw_html = "<div>\n  <!-- comment -->\n  <h1>Hello</h1>\n  <pre>a   b</pre>\n  <script>\n    // do not break\n    let x = 1;\n  </script>\n  <preview>preserved</preview>\n</div>";
        let min_html = minify_html(raw_html);
        assert!(min_html.contains("<pre>a   b</pre>"));
        assert!(min_html.contains("<script>\n    // do not break\n    let x = 1;\n  </script>"));
        assert!(min_html.contains("<preview>preserved</preview>"));
    }

    #[test]
    fn test_minify_css_selectors_and_calc() {
        // Space before ':' in descendant combinator selector must be preserved
        let selector_css = ".container :first-child {\n  color: red;\n}\n";
        assert_eq!(
            minify_css(selector_css),
            ".container :first-child{color:red}"
        );

        // No space before ':' in pseudo-class attached to selector
        let pseudo_css = ".btn:hover {\n  opacity: 0.8;\n}\n";
        assert_eq!(minify_css(pseudo_css), ".btn:hover{opacity:0.8}");

        // Spaces around operators inside calc() must be preserved
        let calc_css = ".box {\n  width: calc(100% - 20px);\n  height: calc(50vh + 10px);\n}\n";
        assert_eq!(
            minify_css(calc_css),
            ".box{width:calc(100% - 20px);height:calc(50vh + 10px)}"
        );
    }
}
