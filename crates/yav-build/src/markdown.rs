//! Zero-dependency Markdown & Frontmatter content engine for documentation and blogs.
//!
//! Parses Markdown files with YAML/TOML-style frontmatter (`--- ... ---`), extracts a
//! structured Table of Contents ([`TocHeading`]), and compiles clean semantic HTML.

use std::{
    collections::HashMap,
    fs,
    path::Path,
};

/// A heading extracted from a Markdown document for Table of Contents navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocHeading {
    /// Heading level (`1..=6`).
    pub level: u8,
    /// Anchor slug ID (`id="..."`).
    pub id: String,
    /// Plain text of the heading.
    pub text: String,
}

/// A parsed Markdown document with frontmatter metadata, TOC headings, and rendered HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDoc {
    /// Document slug (derived from filename or frontmatter `slug`).
    pub slug: String,
    /// Document title (from frontmatter `title` or first `<h1>`).
    pub title: String,
    /// Key-value frontmatter attributes (`title`, `description`, `date`, `order`, etc.).
    pub frontmatter: HashMap<String, String>,
    /// Extracted headings (`h1`..`h6`) in document order.
    pub headings: Vec<TocHeading>,
    /// Compiled HTML body string.
    pub html: String,
}

/// Convert heading text into a URL-safe anchor slug (`"Getting Started!"` -> `"getting-started"`).
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut prev_dash = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

fn escape_md_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn format_inline(input: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code `...`
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let code: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str("<code>");
                out.push_str(&escape_md_html(&code));
                out.push_str("</code>");
                i += end + 2;
                continue;
            }
        }

        // Link [label](url)
        if chars[i] == '[' {
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let paren_start = i + 1 + close_bracket + 1;
                if paren_start < chars.len() && chars[paren_start] == '(' {
                    if let Some(close_paren) =
                        chars[paren_start + 1..].iter().position(|&c| c == ')')
                    {
                        let label: String = chars[i + 1..i + 1 + close_bracket].iter().collect();
                        let url: String = chars
                            [paren_start + 1..paren_start + 1 + close_paren]
                            .iter()
                            .collect();
                        out.push_str("<a href=\"");
                        out.push_str(&escape_md_html(url.trim()));
                        out.push_str("\">");
                        out.push_str(&escape_md_html(&label));
                        out.push_str("</a>");
                        i = paren_start + 1 + close_paren + 1;
                        continue;
                    }
                }
            }
        }

        // Bold **...**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            let rest: String = chars[i + 2..].iter().collect();
            if let Some(end_byte) = rest.find("**") {
                let inner = &rest[..end_byte];
                out.push_str("<strong>");
                out.push_str(&escape_md_html(inner));
                out.push_str("</strong>");
                i += 2 + inner.chars().count() + 2;
                continue;
            }
        }

        // Italic *...*
        if chars[i] == '*' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '*') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str("<em>");
                out.push_str(&escape_md_html(&inner));
                out.push_str("</em>");
                i += end + 2;
                continue;
            }
        }

        match chars[i] {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
        i += 1;
    }

    out
}

/// Parse a Markdown string (with optional `---` frontmatter) into a [`MarkdownDoc`].
pub fn parse_markdown(source: &str) -> MarkdownDoc {
    let normalized = source.replace("\r\n", "\n");
    let mut frontmatter = HashMap::new();
    let mut body = normalized.as_str();

    if let Some(after_open) = normalized.strip_prefix("---\n") {
        if let Some(end_idx) = after_open.find("\n---\n") {
            let fm_block = &after_open[..end_idx];
            body = &after_open[end_idx + 5..];
            for line in fm_block.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    let key = k.trim().to_string();
                    let val = v
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    frontmatter.insert(key, val);
                }
            }
        }
    }

    let mut headings = Vec::new();
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();
    let mut in_ul = false;
    let mut in_ol = false;
    let mut para_buf = Vec::new();

    let flush_para = |para: &mut Vec<String>, out: &mut String| {
        if !para.is_empty() {
            let joined = para.join(" ");
            out.push_str("<p>");
            out.push_str(&format_inline(&joined));
            out.push_str("</p>\n");
            para.clear();
        }
    };

    let close_lists = |ul: &mut bool, ol: &mut bool, out: &mut String| {
        if *ul {
            out.push_str("</ul>\n");
            *ul = false;
        }
        if *ol {
            out.push_str("</ol>\n");
            *ol = false;
        }
    };

    for line in body.lines() {
        if let Some(fence) = line.trim_start().strip_prefix("```") {
            flush_para(&mut para_buf, &mut html);
            close_lists(&mut in_ul, &mut in_ol, &mut html);
            if in_code_block {
                if code_lang.is_empty() {
                    html.push_str("<pre><code>");
                } else {
                    html.push_str("<pre><code class=\"language-");
                    html.push_str(&escape_md_html(&code_lang));
                    html.push_str("\">");
                }
                html.push_str(&escape_md_html(code_buf.trim_end_matches('\n')));
                html.push_str("</code></pre>\n");
                code_buf.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                code_lang = fence.trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_para(&mut para_buf, &mut html);
            close_lists(&mut in_ul, &mut in_ol, &mut html);
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" {
            flush_para(&mut para_buf, &mut html);
            close_lists(&mut in_ul, &mut in_ol, &mut html);
            html.push_str("<hr />\n");
            continue;
        }

        // Heading (# .. ######)
        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hash_count) && trimmed[hash_count..].starts_with(' ') {
            flush_para(&mut para_buf, &mut html);
            close_lists(&mut in_ul, &mut in_ol, &mut html);
            let text = trimmed[hash_count..].trim().to_string();
            let id = slugify(&text);
            let level = hash_count as u8;
            headings.push(TocHeading {
                level,
                id: id.clone(),
                text: text.clone(),
            });
            html.push_str(&format!(
                "<h{level} id=\"{id}\">{}</h{level}>\n",
                format_inline(&text)
            ));
            continue;
        }

        // Blockquote
        if let Some(quote_text) = trimmed.strip_prefix("> ") {
            flush_para(&mut para_buf, &mut html);
            close_lists(&mut in_ul, &mut in_ol, &mut html);
            html.push_str("<blockquote><p>");
            html.push_str(&format_inline(quote_text.trim()));
            html.push_str("</p></blockquote>\n");
            continue;
        }

        // Unordered list item
        if let Some(item_text) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_para(&mut para_buf, &mut html);
            if in_ol {
                html.push_str("</ol>\n");
                in_ol = false;
            }
            if !in_ul {
                html.push_str("<ul>\n");
                in_ul = true;
            }
            html.push_str("<li>");
            html.push_str(&format_inline(item_text.trim()));
            html.push_str("</li>\n");
            continue;
        }

        // Ordered list item (e.g. `1. `)
        if let Some((num_part, item_text)) = trimmed.split_once(". ") {
            if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                flush_para(&mut para_buf, &mut html);
                if in_ul {
                    html.push_str("</ul>\n");
                    in_ul = false;
                }
                if !in_ol {
                    html.push_str("<ol>\n");
                    in_ol = true;
                }
                html.push_str("<li>");
                html.push_str(&format_inline(item_text.trim()));
                html.push_str("</li>\n");
                continue;
            }
        }

        para_buf.push(trimmed.to_string());
    }

    flush_para(&mut para_buf, &mut html);
    close_lists(&mut in_ul, &mut in_ol, &mut html);

    let title = frontmatter
        .get("title")
        .cloned()
        .or_else(|| headings.first().map(|h| h.text.clone()))
        .unwrap_or_default();

    let slug = frontmatter
        .get("slug")
        .cloned()
        .unwrap_or_else(|| slugify(&title));

    MarkdownDoc {
        slug,
        title,
        frontmatter,
        headings,
        html,
    }
}

/// Recursively load and parse all `.md` files in a directory into [`MarkdownDoc`] entries.
pub fn load_markdown_dir(dir: impl AsRef<Path>) -> std::io::Result<Vec<MarkdownDoc>> {
    let base = dir.as_ref();
    let mut docs = Vec::new();
    collect_markdown_files(base, base, &mut docs)?;
    docs.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(docs)
}

fn collect_markdown_files(
    base: &Path,
    current: &Path,
    out: &mut Vec<MarkdownDoc>,
) -> std::io::Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(base, &path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let content = fs::read_to_string(&path)?;
            let mut doc = parse_markdown(&content);
            if !doc.frontmatter.contains_key("slug") {
                if let Ok(rel) = path.strip_prefix(base) {
                    let rel_str = rel
                        .with_extension("")
                        .to_string_lossy()
                        .replace('\\', "/");
                    doc.slug = rel_str;
                }
            }
            out.push(doc);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_frontmatter_and_toc() {
        let raw = r#"---
title: "Quickstart Guide"
slug: "getting-started"
order: 1
---

# Quickstart Guide

Welcome to **Yav.rs** with `rsx!` and [Documentation](/docs).

## Installation

- Install Rust
- Run `yav new my-app`

```rust
let x = 42;
```
"#;
        let doc = parse_markdown(raw);
        assert_eq!(doc.title, "Quickstart Guide");
        assert_eq!(doc.slug, "getting-started");
        assert_eq!(doc.frontmatter.get("order"), Some(&"1".to_string()));
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(doc.headings[0].id, "quickstart-guide");
        assert_eq!(doc.headings[1].id, "installation");
        assert!(doc.html.contains("<h1 id=\"quickstart-guide\">Quickstart Guide</h1>"));
        assert!(doc.html.contains("<strong>Yav.rs</strong>"));
        assert!(doc.html.contains("<code>rsx!</code>"));
        assert!(doc.html.contains("<a href=\"/docs\">Documentation</a>"));
        assert!(doc.html.contains("<ul>"));
        assert!(doc.html.contains("<code class=\"language-rust\">let x = 42;</code>"));
    }
}
