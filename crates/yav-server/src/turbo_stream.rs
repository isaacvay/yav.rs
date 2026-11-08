//! # YAV TurboStream & Multi-Zone Partial DOM Streaming
//!
//! Provides real-time server-push DOM mutations over SSE (Server-Sent Events)
//! without full-page reloads. Supports `append`, `prepend`, `replace`, `update`,
//! `remove`, and non-destructive `morph` actions.

use yav_core::escape_html;

/// Action specifying how the client should mutate the target DOM element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurboAction {
    Append,
    Prepend,
    Replace,
    Update,
    Remove,
    Morph,
}

impl TurboAction {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Append => "append",
            Self::Prepend => "prepend",
            Self::Replace => "replace",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Morph => "morph",
        }
    }
}

/// A single atomic turbo-stream DOM mutation message.
#[derive(Debug, Clone, PartialEq)]
pub struct TurboMessage {
    pub action: TurboAction,
    pub target: String,
    pub html_content: Option<String>,
}

impl TurboMessage {
    #[must_use]
    pub fn new(action: TurboAction, target: impl Into<String>) -> Self {
        Self {
            action,
            target: target.into(),
            html_content: None,
        }
    }

    #[must_use]
    pub fn content(mut self, html: impl Into<String>) -> Self {
        self.html_content = Some(html.into());
        self
    }

    /// Render message into a W3C-compliant `<turbo-stream>` element string.
    #[must_use]
    pub fn to_html(&self) -> String {
        let action = self.action.as_str();
        let target = escape_html(&self.target);

        if self.action == TurboAction::Remove {
            format!(r#"<turbo-stream action="{action}" target="{target}"></turbo-stream>"#)
        } else {
            let content = self.html_content.as_deref().unwrap_or("");
            format!(
                r#"<turbo-stream action="{action}" target="{target}"><template>{content}</template></turbo-stream>"#
            )
        }
    }
}

/// Collection and builder of multiple TurboStream messages streamed in a single frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TurboStream {
    messages: Vec<TurboMessage>,
}

impl TurboStream {
    #[must_use]
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    #[must_use]
    pub fn append(mut self, target: impl Into<String>, html: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Append, target).content(html));
        self
    }

    #[must_use]
    pub fn prepend(mut self, target: impl Into<String>, html: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Prepend, target).content(html));
        self
    }

    #[must_use]
    pub fn replace(mut self, target: impl Into<String>, html: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Replace, target).content(html));
        self
    }

    #[must_use]
    pub fn update(mut self, target: impl Into<String>, html: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Update, target).content(html));
        self
    }

    #[must_use]
    pub fn remove(mut self, target: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Remove, target));
        self
    }

    #[must_use]
    pub fn morph(mut self, target: impl Into<String>, html: impl Into<String>) -> Self {
        self.messages
            .push(TurboMessage::new(TurboAction::Morph, target).content(html));
        self
    }

    /// Render all turbo-stream mutations into a single HTML packet.
    #[must_use]
    pub fn render_html(&self) -> String {
        self.messages
            .iter()
            .map(TurboMessage::to_html)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format as Server-Sent Event (SSE) data payload.
    #[must_use]
    pub fn to_sse_event(&self) -> String {
        let html = self.render_html();
        let escaped = html.replace('\n', " ");
        format!("event: turbo-stream\ndata: {escaped}\n\n")
    }
}

/// Client runtime script providing listener for `turbo-stream` events.
pub const YAV_TURBO_STREAM_SCRIPT: &str = r#"(function(){
if (window.__yavTurboStreamInit) return;
window.__yavTurboStreamInit = true;

window.applyTurboStream = function(html) {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, 'text/html');
  const streams = doc.querySelectorAll('turbo-stream');

  streams.forEach(stream => {
    const action = stream.getAttribute('action');
    const targetId = stream.getAttribute('target');
    const target = document.getElementById(targetId) || document.querySelector(targetId);
    if (!target) return;

    if (action === 'remove') {
      target.remove();
      return;
    }

    const template = stream.querySelector('template');
    if (!template) return;
    const content = template.content.cloneNode(true);

    switch (action) {
      case 'append':
        target.appendChild(content);
        break;
      case 'prepend':
        target.insertBefore(content, target.firstChild);
        break;
      case 'replace':
        target.replaceWith(content);
        break;
      case 'update':
        target.innerHTML = '';
        target.appendChild(content);
        break;
      case 'morph':
        if (window.__yavMorph) {
          const wrapper = document.createElement('div');
          wrapper.appendChild(content);
          window.__yavMorph(target, wrapper.firstElementChild || wrapper);
        } else {
          target.innerHTML = '';
          target.appendChild(content);
        }
        break;
    }
  });
};
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turbo_message_formatting() {
        let msg = TurboMessage::new(TurboAction::Replace, "cart-count").content("<span>4</span>");
        let html = msg.to_html();
        assert_eq!(
            html,
            r#"<turbo-stream action="replace" target="cart-count"><template><span>4</span></template></turbo-stream>"#
        );

        let del = TurboMessage::new(TurboAction::Remove, "toast-102");
        assert_eq!(
            del.to_html(),
            r#"<turbo-stream action="remove" target="toast-102"></turbo-stream>"#
        );
    }

    #[test]
    fn test_turbo_stream_builder_and_sse() {
        let stream = TurboStream::new()
            .append("chat-feed", "<div class=\"msg\">Hello!</div>")
            .morph("user-status", "<span class=\"online\">Online</span>")
            .remove("typing-indicator");

        let html = stream.render_html();
        assert!(html.contains("action=\"append\" target=\"chat-feed\""));
        assert!(html.contains("action=\"morph\" target=\"user-status\""));
        assert!(html.contains("action=\"remove\" target=\"typing-indicator\""));

        let sse = stream.to_sse_event();
        assert!(sse.starts_with("event: turbo-stream\ndata: "));
        assert!(sse.ends_with("\n\n"));
    }
}
