//! Accessibility (a11y) linting for rendered HTML.
//!
//! This module is available behind the `a11y` feature flag. It uses
//! [`scraper`] (html5ever + CSS selectors) to inspect a finished HTML
//! document and report a small, stable set of common accessibility defects.
//!
//! The rules are intentionally limited and deterministic so the output can
//! be used in build reports or snapshot tests:
//!
//! - `img-alt`     — `<img>` without an `alt` attribute
//! - `button-name` — `<button>` without text or an ARIA label
//! - `input-label` — `<input>` without an ARIA label or a `<label for>`
//! - `link-name`   — `<a>` without text or an ARIA label
//! - `html-lang`   — `<html>` without a `lang` attribute

#![cfg(feature = "a11y")]

use std::collections::HashSet;

use scraper::{ElementRef, Html, Selector};

/// A single accessibility problem found in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yIssue {
    /// Stable rule identifier (e.g. `img-alt`).
    pub rule: String,
    /// Human-readable explanation of the problem.
    pub message: String,
    /// Short, escaped description of the offending element (e.g. `<img src="logo.png">`).
    pub element: String,
}

/// Lint an HTML document and return its accessibility issues.
///
/// Issues are returned in a stable order: rules are checked in the order
/// documented in the module description, and elements are visited in document
/// order within each rule.
pub fn lint_html(html: &str) -> Vec<A11yIssue> {
    let document = Html::parse_document(html);
    let mut issues = Vec::new();

    // Rule 1: images need an `alt` attribute (empty `alt=""` is valid for
    // purely decorative images, so only the attribute *presence* is checked).
    let img_selector = Selector::parse("img").expect("static `img` selector is valid");
    for element in document.select(&img_selector) {
        if element.value().attr("alt").is_none() {
            issues.push(A11yIssue {
                rule: "img-alt".to_string(),
                message:
                    "Image is missing an `alt` attribute (use alt=\"\" for decorative images)."
                        .to_string(),
                element: describe_element(&element),
            });
        }
    }

    // Rule 2: buttons need an accessible name from their text content or ARIA.
    let button_selector = Selector::parse("button").expect("static `button` selector is valid");
    for element in document.select(&button_selector) {
        if !has_name(&element) {
            issues.push(A11yIssue {
                rule: "button-name".to_string(),
                message: "Button has no text content, `aria-label` or `aria-labelledby`."
                    .to_string(),
                element: describe_element(&element),
            });
        }
    }

    // Rule 3: inputs need a label. We collect every `<label for="id">` first,
    // then accept an input that carries `aria-label`, `aria-labelledby`, or an
    // `id` referenced by one of those labels.
    let label_selector =
        Selector::parse("label[for]").expect("static `label[for]` selector is valid");
    let labelled_ids: HashSet<&str> = document
        .select(&label_selector)
        .filter_map(|label| label.value().attr("for"))
        .collect();

    let input_selector = Selector::parse("input").expect("static `input` selector is valid");
    for element in document.select(&input_selector) {
        let has_id_label = element
            .value()
            .attr("id")
            .map(|id| labelled_ids.contains(id))
            .unwrap_or(false);
        if !has_name(&element) && !has_id_label {
            issues.push(A11yIssue {
                rule: "input-label".to_string(),
                message:
                    "Input has no `aria-label`, `aria-labelledby`, or associated `<label for>`."
                        .to_string(),
                element: describe_element(&element),
            });
        }
    }

    // Rule 4: links need an accessible name, same criterion as buttons.
    let link_selector = Selector::parse("a").expect("static `a` selector is valid");
    for element in document.select(&link_selector) {
        if !has_name(&element) {
            issues.push(A11yIssue {
                rule: "link-name".to_string(),
                message: "Link has no text content, `aria-label` or `aria-labelledby`.".to_string(),
                element: describe_element(&element),
            });
        }
    }

    // Rule 5: the document element must declare a language.
    let html_selector = Selector::parse("html").expect("static `html` selector is valid");
    let mut html_elements = document.select(&html_selector);
    match html_elements.next() {
        Some(element) if element.value().attr("lang").is_none() => issues.push(A11yIssue {
            rule: "html-lang".to_string(),
            message: "The `<html>` element is missing a `lang` attribute.".to_string(),
            element: describe_element(&element),
        }),
        None => issues.push(A11yIssue {
            rule: "html-lang".to_string(),
            message: "The document has no `<html lang=\"...\">` element.".to_string(),
            element: "<html>".to_string(),
        }),
        _ => {}
    }

    issues
}

/// Render a compact, escapable HTML report for a list of issues.
pub fn lint_report_html(issues: &[A11yIssue]) -> String {
    if issues.is_empty() {
        return "<p class=\"a11y-report-ok\">No accessibility issues found.</p>".to_string();
    }

    let mut out = String::new();
    out.push_str("<div class=\"a11y-report\">");
    out.push_str(&format!(
        "<h2>Accessibility report ({} issues)</h2><ul>",
        issues.len()
    ));
    for issue in issues {
        out.push_str("<li><strong>");
        out.push_str(&escape_html(&issue.rule));
        out.push_str("</strong>: ");
        out.push_str(&escape_html(&issue.message));
        out.push_str(" <code>");
        out.push_str(&escape_html(&issue.element));
        out.push_str("</code></li>");
    }
    out.push_str("</ul></div>");
    out
}

/// An element has an accessible name if it has non-whitespace descendant text
/// or a non-empty ARIA labelling attribute.
fn has_name(element: &ElementRef<'_>) -> bool {
    let has_text = element.text().any(|text| !text.trim().is_empty());
    if has_text {
        return true;
    }
    ["aria-label", "aria-labelledby"].iter().any(|attr| {
        element
            .value()
            .attr(attr)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

/// Build a short, deterministic description of an offending element.
fn describe_element(element: &ElementRef<'_>) -> String {
    let mut out = format!("<{}", element.value().name());
    for attr in ["id", "class", "src", "href", "type", "name"] {
        if let Some(value) = element.value().attr(attr) {
            out.push(' ');
            out.push_str(attr);
            out.push_str("=\"");
            out.push_str(&escape_html(value));
            out.push('"');
        }
    }
    out.push('>');
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
    fn valid_document_has_no_issues() {
        let html = r#"<!DOCTYPE html>
<html lang="en"><head><title>Test</title></head><body>
  <img src="/logo.png" alt="Yav logo" />
  <button>Send</button>
  <label for="email">Email</label><input id="email" type="email" />
  <input type="text" aria-label="Search" />
  <a href="/docs">Documentation</a>
</body></html>"#;

        let issues = lint_html(html);
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
        assert!(lint_report_html(&issues).contains("No accessibility issues found"));
    }

    #[test]
    fn invalid_document_reports_every_rule_in_stable_order() {
        let html = r#"<!DOCTYPE html><html><body>
  <img src="/logo.png" />
  <button></button>
  <input type="text" />
  <a href="/docs"></a>
</body></html>"#;

        let issues = lint_html(html);
        let rules: Vec<&str> = issues.iter().map(|issue| issue.rule.as_str()).collect();
        assert_eq!(
            rules,
            vec![
                "img-alt",
                "button-name",
                "input-label",
                "link-name",
                "html-lang",
            ]
        );
        assert!(issues.iter().all(|issue| issue.element.contains('<')));
    }

    #[test]
    fn aria_labels_and_whitespace_are_handled() {
        let html = r#"<html lang="fr"><body>
          <button aria-label="Fermer"><span> </span></button>
          <input aria-labelledby="lbl" />
          <a><strong>Accueil</strong></a>
        </body></html>"#;
        // Decorative image with an explicit empty alt is accepted.
        let html = html.replace("<body>", "<body><img alt=\"\" src=\"dot.png\" />");
        assert!(lint_html(&html).is_empty());
    }

    #[test]
    fn report_is_escaped_and_lists_rules() {
        let issues = lint_html("<img src=\"a.png\" />");
        assert_eq!(issues.len(), 2); // img-alt + auto-inserted <html> without lang
        let report = lint_report_html(&issues);
        assert!(report.contains("<strong>img-alt</strong>"));
        assert!(report.contains("&lt;img"));
        assert!(report.contains("Accessibility report (2 issues)"));
    }
}
