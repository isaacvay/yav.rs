//! Markdown rendering powered by `pulldown-cmark` and `syntect`.
//!
//! Available behind the `markdown` feature flag. This is an alternative to the
//! zero-dependency [`crate::markdown`] engine: it offers full CommonMark + GFM
//! parsing and real syntax highlighting, at the cost of two dependencies.
//!
//! Raw HTML in the source is deliberately **escaped**: HTML events emitted by
//! the parser are rewritten as text events before the HTML renderer runs, so
//! `<script>` in a Markdown file can never become executable markup.

#![cfg(feature = "markdown")]

use pulldown_cmark::{html, Event, Options, Parser};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Render a Markdown string to an HTML fragment.
///
/// Enables tables, strikethrough, task lists and the remaining GFM extras.
/// Raw HTML is escaped rather than passed through (there is intentionally no
/// raw-HTML option enabled here).
pub fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    // Force any raw HTML event back into a text event. `html::push_html`
    // escapes `Event::Text`, so the resulting markup is inert. This also
    // covers both inline HTML (`Event::InlineHtml`) and HTML block lines
    // (`Event::Html`) across pulldown-cmark versions.
    let events = Parser::new_ext(md, options).map(|event| match event {
        Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
        other => other,
    });

    let mut output = String::with_capacity(md.len() * 2);
    html::push_html(&mut output, events);
    output
}

/// Highlight a code snippet with syntect's built-in syntax and theme sets.
///
/// Returns a `<pre><code class="language-...">` block whose lines are wrapped
/// in `<span style="color:#...">` elements. Unknown languages fall back to
/// plain text (still safely escaped); if syntect itself fails on a line, that
/// line is emitted as escaped plain text instead of aborting the render.
pub fn highlight_code(code: &str, lang: &str) -> String {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let theme = &theme_set.themes["base16-ocean.dark"];
    let syntax = syntax_set
        .find_syntax_by_token(lang)
        .or_else(|| syntax_set.find_syntax_by_extension(lang))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut body = String::with_capacity(code.len() * 2);
    for line in LinesWithEndings::from(code) {
        match highlighter.highlight_line(line, &syntax_set) {
            Ok(regions) => match styled_line_to_highlighted_html(&regions, IncludeBackground::No) {
                Ok(highlighted) => body.push_str(&highlighted),
                Err(_) => body.push_str(&escape_html(line)),
            },
            Err(_) => body.push_str(&escape_html(line)),
        }
    }

    let mut out = String::with_capacity(body.len() + 64);
    out.push_str("<pre><code class=\"language-");
    out.push_str(&escape_html(lang));
    out.push_str("\">");
    out.push_str(&body);
    out.push_str("</code></pre>");
    out
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_heading_table_task_list_and_code_block() {
        let md = r#"# Title

| Feature | Status |
| :--- | :---: |
| Tables | Ready |

- [x] Done
- [ ] Todo

~~obsolete~~

```rust
fn main() {}
```
"#;

        let html = render_markdown(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<table>"));
        // pulldown-cmark adds an inline `text-align` style for aligned GFM
        // tables, so assert the cell content rather than the exact opening tag.
        assert!(html.contains("Feature</th>"));
        assert!(html.contains("Status</th>"));
        assert!(html.contains("Tables</td>"));
        assert!(html.contains("text-align: center"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("<del>obsolete</del>"));
        assert!(html.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn raw_html_is_escaped() {
        let html = render_markdown("Safe <script>alert('x')</script> text");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;/script&gt;"));
    }

    #[test]
    fn syntect_produces_colored_spans_and_escapes_unknown_languages() {
        let highlighted = highlight_code("fn main() { let x = 42; }", "rust");
        assert!(highlighted.starts_with("<pre><code class=\"language-rust\">"));
        assert!(highlighted.contains("<span style="));
        assert!(highlighted.contains("</code></pre>"));

        let fallback = highlight_code("<b>not code</b>", "no-such-language");
        assert!(fallback.contains("&lt;b&gt;"));
        assert!(!fallback.contains("<b>not code</b>"));
    }
}
