//! Dual-Pane Live Markdown Editor Component.
//!
//! Provides a responsive split-view editor featuring a rich markdown toolbar,
//! synchronized preview pane, and zero-JS instant editing capabilities.

use yav_core::{Component, Element, Node};

/// Configurable Markdown Editor component with live split-view preview.
#[derive(Debug, Clone)]
pub struct MarkdownEditor {
    pub name: String,
    pub initial_value: String,
    pub height_px: u32,
    pub preview_html: Option<String>,
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

    #[must_use]
    pub fn preview_html(mut self, html: impl Into<String>) -> Self {
        self.preview_html = Some(html.into());
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

        // Right pane: Rendered HTML Preview
        let preview_content = match &self.preview_html {
            Some(html) => Node::raw(html.clone()),
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
}
