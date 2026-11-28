//! # YAV Rich Text Block Editor & AI Streaming Chat UI
//!
//! Provides `<RichTextEditor />` (Notion/TipTap/Lexical block editor with
//! formatting toolbar and hidden form synchronization) and `<AiChat />`
//! (Vercel AI SDK `<useChat />` streaming conversation interface).

use yav_core::{Component, Element, Node};

use crate::utils::merge_classes;

/// Props for [`RichTextEditor`].
#[derive(Debug, Clone, Default)]
pub struct RichTextEditorProps {
    pub id: String,
    pub name: String,
    pub initial_html: String,
    pub placeholder: String,
    pub class: String,
    pub children: Vec<Node>,
}

/// Notion/TipTap-style rich text editor with formatting toolbar, slash-command
/// hint, and automatic synchronization to a hidden `<input type="hidden">`.
#[derive(Debug, Clone, Default)]
pub struct RichTextEditor {
    pub id: String,
    pub name: String,
    pub initial_html: String,
    pub placeholder: String,
    pub class: String,
    pub children: Vec<Node>,
}

impl RichTextEditor {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        Self {
            id: format!("yav-rte-{name_str}"),
            name: name_str,
            initial_html: String::new(),
            placeholder: "Type '/' for commands or start writing...".to_string(),
            class: String::new(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn initial_html(mut self, html: impl Into<String>) -> Self {
        self.initial_html = html.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl RichTextEditor {
    pub fn from_props(props: RichTextEditorProps) -> Self {
        Self {
            id: props.id,
            name: props.name,
            initial_html: props.initial_html,
            placeholder: props.placeholder,
            class: props.class,
            children: props.children,
        }
    }
}

impl Component for RichTextEditor {
    fn render(&self) -> Node {
        let classes = merge_classes(&[
            "yav-rich-editor rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 shadow-sm overflow-hidden",
            &self.class,
        ]);

        let actions = [
            ("bold", "B"),
            ("italic", "I"),
            ("code", "</>"),
            ("h1", "H1"),
            ("h2", "H2"),
            ("checklist", "☑"),
            ("callout", "💡"),
        ];

        let mut toolbar = Element::new("div")
            .attr(
                "class",
                "flex items-center gap-1 border-b border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-950 px-2 py-1.5",
            )
            .attr("role", "toolbar");

        for (cmd, label) in actions {
            let btn = Element::new("button")
                .attr("type", "button")
                .attr(
                    "class",
                    "px-2 py-1 text-xs font-semibold rounded hover:bg-gray-200 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300",
                )
                .attr("data-rte-cmd", cmd)
                .child(Node::Text(label.to_string()));
            toolbar = toolbar.child(Node::Element(btn));
        }

        let surface = Element::new("div")
            .attr("id", format!("{}-surface", self.id))
            .attr(
                "class",
                "yav-rte-surface prose dark:prose-invert max-w-none min-h-[180px] p-4 focus:outline-none",
            )
            .attr("contenteditable", "true")
            .attr("role", "textbox")
            .attr("aria-multiline", "true")
            .attr("data-placeholder", &self.placeholder)
            .child(Node::Raw(yav_core::sanitize_html_strict(&self.initial_html)));

        let hidden_input = Element::new("input")
            .attr("type", "hidden")
            .attr("name", &self.name)
            .attr("value", &self.initial_html);

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &classes)
                .child(Node::Element(toolbar))
                .child(Node::Element(surface))
                .child(Node::Element(hidden_input)),
        )
    }
}

/// A message displayed in [`AiChat`].
#[derive(Debug, Clone)]
pub struct AiChatBubble {
    pub role: String,
    pub content: String,
}

impl AiChatBubble {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Props for [`AiChat`].
#[derive(Debug, Clone, Default)]
pub struct AiChatProps {
    pub id: String,
    pub endpoint: String,
    pub messages: Vec<AiChatBubble>,
    pub placeholder: String,
    pub class: String,
    pub children: Vec<Node>,
}

/// Vercel AI SDK (`useChat`) equivalent streaming chat interface with SSE
/// token target, generative UI slot, and prompt form.
#[derive(Debug, Clone, Default)]
pub struct AiChat {
    pub id: String,
    pub endpoint: String,
    pub messages: Vec<AiChatBubble>,
    pub placeholder: String,
    pub class: String,
    pub children: Vec<Node>,
}

impl AiChat {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            id: "yav-ai-chat".to_string(),
            endpoint: endpoint.into(),
            messages: Vec::new(),
            placeholder: "Ask anything...".to_string(),
            class: String::new(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn message(mut self, bubble: AiChatBubble) -> Self {
        self.messages.push(bubble);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl AiChat {
    pub fn from_props(props: AiChatProps) -> Self {
        Self {
            id: if props.id.is_empty() {
                "yav-ai-chat".to_string()
            } else {
                props.id
            },
            endpoint: props.endpoint,
            messages: props.messages,
            placeholder: props.placeholder,
            class: props.class,
            children: props.children,
        }
    }
}

impl Component for AiChat {
    fn render(&self) -> Node {
        let classes = merge_classes(&[
            "yav-ai-chat flex flex-col rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 overflow-hidden",
            &self.class,
        ]);

        let mut feed = Element::new("div")
            .attr("id", format!("{}-messages", self.id))
            .attr("class", "flex-1 space-y-3 p-4 overflow-y-auto")
            .attr("role", "log")
            .attr("aria-live", "polite");

        for msg in &self.messages {
            let bubble_cls = if msg.role == "user" {
                "ml-auto max-w-[80%] rounded-2xl bg-indigo-600 text-white px-4 py-2 text-sm"
            } else {
                "mr-auto max-w-[80%] rounded-2xl bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 px-4 py-2 text-sm"
            };
            feed = feed.child(Node::Element(
                Element::new("div")
                    .attr("class", bubble_cls)
                    .attr("data-role", &msg.role)
                    .child(Node::Text(msg.content.clone())),
            ));
        }

        for extra in &self.children {
            feed = feed.child(extra.clone());
        }

        let form = Element::new("form")
            .attr("action", &self.endpoint)
            .attr("method", "post")
            .attr("yav:ai-stream", &self.endpoint)
            .attr(
                "class",
                "flex items-center gap-2 border-t border-gray-200 dark:border-gray-800 p-3",
            )
            .child(Node::Element(
                Element::new("input")
                    .attr("type", "text")
                    .attr("name", "prompt")
                    .attr("placeholder", &self.placeholder)
                    .attr(
                        "class",
                        "flex-1 rounded-lg border border-gray-300 dark:border-gray-700 bg-transparent px-3 py-2 text-sm",
                    ),
            ))
            .child(Node::Element(
                Element::new("button")
                    .attr("type", "submit")
                    .attr(
                        "class",
                        "rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500",
                    )
                    .child(Node::Text("Send".to_string())),
            ));

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &classes)
                .child(Node::Element(feed))
                .child(Node::Element(form)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_rich_text_editor_and_ai_chat_render() {
        let editor = RichTextEditor::new("body")
            .initial_html("<p>Hello Notion</p>")
            .render();
        let html = render_to_string(&editor);
        assert!(html.contains("contenteditable=\"true\""));
        assert!(html.contains("data-rte-cmd=\"checklist\""));
        assert!(html.contains("<p>Hello Notion</p>"));

        let chat = AiChat::new("/api/ai/chat")
            .message(AiChatBubble::user("Summarize metrics"))
            .message(AiChatBubble::assistant("Revenue is up 24%."))
            .render();
        let chat_html = render_to_string(&chat);
        assert!(chat_html.contains("yav:ai-stream=\"/api/ai/chat\""));
        assert!(chat_html.contains("Summarize metrics"));
        assert!(chat_html.contains("Revenue is up 24%."));
    }
}
