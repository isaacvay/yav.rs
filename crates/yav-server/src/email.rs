//! Transactional Email Engine with RSX Template Rendering (`yav::email` — React Email equivalent).
//!
//! Compose emails using `rsx! { ... }` components, automatically generating both a responsive
//! email-client-safe HTML document and a clean plain-text fallback (`multipart/alternative`).
//!
//! # Development transport only — no SMTP
//!
//! The `Mailer` in this module is an **in-memory / `.eml` development transport**: it
//! records messages in an inspectable outbox and can write `.eml` files for local
//! previewing. It performs **no SMTP** (or any other network) delivery and is **not
//! suitable for production delivery**. A successful `Mailer::send` means "stored in
//! the outbox", not "delivered to a recipient". For production, implement
//! `crate::adapters::MailAdapter` with a real provider.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, RwLock};
use yav_core::{render_to_string, Node};

/// A transactional email message with HTML and plain-text representations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailMessage {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub preheader: Option<String>,
    pub html_body: String,
    pub text_body: String,
}

impl Default for EmailMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailMessage {
    /// Create a new empty email message builder.
    pub fn new() -> Self {
        Self {
            from: "noreply@localhost".to_string(),
            to: Vec::new(),
            cc: Vec::new(),
            subject: String::new(),
            preheader: None,
            html_body: String::new(),
            text_body: String::new(),
        }
    }

    /// Set the sender `From` address.
    #[must_use]
    pub fn from(mut self, sender: impl Into<String>) -> Self {
        self.from = sender.into();
        self
    }

    /// Add a recipient `To` address.
    #[must_use]
    pub fn to(mut self, recipient: impl Into<String>) -> Self {
        self.to.push(recipient.into());
        self
    }

    /// Add a `CC` recipient address.
    #[must_use]
    pub fn cc(mut self, recipient: impl Into<String>) -> Self {
        self.cc.push(recipient.into());
        self
    }

    /// Set the email `Subject` line.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the hidden inbox preview text (`preheader`).
    #[must_use]
    pub fn preheader(mut self, preheader: impl Into<String>) -> Self {
        self.preheader = Some(preheader.into());
        self
    }

    /// Render an RSX [`Node`] tree into a responsive HTML email layout and automatically
    /// extract a plain-text fallback version.
    #[must_use]
    pub fn render_rsx(mut self, body_node: &Node) -> Self {
        let inner_html = render_to_string(body_node);
        let preheader_block = self
            .preheader
            .as_deref()
            .map(|p| {
                format!(
                    "<div style=\"display:none;max-height:0;overflow:hidden;mso-hide:all;\">{p}</div>"
                )
            })
            .unwrap_or_default();

        self.html_body = format!(
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
            <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
            <title>{subject}</title></head>\
            <body style=\"margin:0;padding:24px;background-color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#0f172a;\">\
            {preheader_block}\
            <table role=\"presentation\" width=\"100%\" cellspacing=\"0\" cellpadding=\"0\" border=\"0\">\
            <tr><td align=\"center\">\
            <table role=\"presentation\" width=\"600\" cellspacing=\"0\" cellpadding=\"0\" border=\"0\" style=\"max-width:600px;width:100%;background:#ffffff;border:1px solid #e2e8f0;border-radius:12px;padding:32px;\">\
            <tr><td>{inner_html}</td></tr>\
            </table></td></tr></table></body></html>",
            subject = self.subject
        );

        let mut plain = String::new();
        extract_plain_text_from_node(body_node, &mut plain);
        self.text_body = plain.trim().to_string();
        self
    }

    /// Format this message as a standard RFC 5322 `.eml` MIME payload.
    pub fn to_eml(&self) -> String {
        let boundary = "yav_mime_boundary_2026";
        format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n\
            --{}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\n\r\n\
            --{}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}\r\n\r\n\
            --{}--\r\n",
            self.from,
            self.to.join(", "),
            self.subject,
            boundary,
            boundary,
            self.text_body,
            boundary,
            self.html_body,
            boundary
        )
    }
}

/// Recursively extract readable plain text from an RSX [`Node`] tree.
pub fn extract_plain_text_from_node(node: &Node, out: &mut String) {
    match node {
        Node::Empty => {}
        Node::Text(t) => {
            out.push_str(t);
        }
        Node::Static(s) => {
            out.push_str(s);
        }
        Node::Raw(raw) => {
            // Strip basic HTML tags for plain-text fallback
            let mut in_tag = false;
            for ch in raw.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => out.push(ch),
                    _ => {}
                }
            }
        }
        Node::Fragment(children) => {
            for child in children {
                extract_plain_text_from_node(child, out);
            }
        }
        Node::Element(el) => {
            let tag_str: &str = &el.tag;
            let is_block = matches!(
                tag_str,
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "li" | "tr" | "br" | "hr"
            );
            if tag_str == "li" {
                out.push_str("\n• ");
            }
            for child in &el.children {
                extract_plain_text_from_node(child, out);
            }
            if tag_str == "a" {
                if let Some(attr) = el.attributes.iter().find(|a| a.name == "href") {
                    match &attr.value {
                        yav_core::AttributeValue::String(href) => {
                            out.push_str(&format!(" ({href})"));
                        }
                        yav_core::AttributeValue::Static(href) => {
                            out.push_str(&format!(" ({href})"));
                        }
                        _ => {}
                    }
                }
            }
            if is_block {
                out.push('\n');
            }
        }
    }
}

/// Thread-safe **in-memory / `.eml` development transport** with an inspectable outbox.
///
/// No SMTP or network delivery is performed; messages are only recorded in memory
/// (or written as `.eml` files for local preview). Not suitable for production delivery.
#[derive(Clone, Default)]
pub struct Mailer {
    outbox: Arc<RwLock<Vec<EmailMessage>>>,
}

impl Mailer {
    /// Create a new [`Mailer`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an [`EmailMessage`] in the in-memory outbox.
    ///
    /// **In-memory / `.eml` development transport, no SMTP; not suitable for production
    /// delivery.** `Ok(())` means the message was stored in the outbox, **not** that it
    /// was delivered to any recipient.
    pub fn send(&self, msg: EmailMessage) -> Result<(), String> {
        if msg.to.is_empty() {
            return Err("EmailMessage must have at least one recipient".to_string());
        }
        self.outbox
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(msg);
        Ok(())
    }

    /// Write an [`EmailMessage`] as a `.eml` file inside `dir` for local dev previewing.
    pub fn save_eml(
        &self,
        msg: &EmailMessage,
        dir: impl AsRef<Path>,
    ) -> Result<std::path::PathBuf, String> {
        let d = dir.as_ref();
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        let idx = self.sent_count() + 1;
        let file_path = d.join(format!("email_{idx}.eml"));
        std::fs::write(&file_path, msg.to_eml()).map_err(|e| e.to_string())?;
        Ok(file_path)
    }

    /// Return all sent messages recorded in the outbox.
    pub fn sent_messages(&self) -> Vec<EmailMessage> {
        self.outbox
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Return the total number of sent emails.
    pub fn sent_count(&self) -> usize {
        self.outbox.read().unwrap_or_else(|e| e.into_inner()).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::Element;

    #[test]
    fn test_email_rsx_rendering_and_plain_text_fallback() {
        let tree = Node::Element(
            Element::new("div")
                .child(Node::Element(
                    Element::new("h1").child(Node::Text("Welcome to Yav!".to_string())),
                ))
                .child(Node::Element(
                    Element::new("p")
                        .child(Node::Text("Click ".to_string()))
                        .child(Node::Element(
                            Element::new("a")
                                .attr("href", "https://yav.rs/verify")
                                .child(Node::Text("here".to_string())),
                        ))
                        .child(Node::Text(" to verify your account.".to_string())),
                )),
        );

        let email = EmailMessage::new()
            .from("team@yav.rs")
            .to("user@example.com")
            .subject("Verify your Yav account")
            .preheader("One-click verification link inside")
            .render_rsx(&tree);

        assert!(email.html_body.contains("<h1>Welcome to Yav!</h1>"));
        assert!(email.html_body.contains("max-width:600px"));
        assert!(email.text_body.contains("Welcome to Yav!"));
        assert!(email.text_body.contains("here (https://yav.rs/verify)"));

        let mailer = Mailer::new();
        mailer.send(email).unwrap();
        assert_eq!(mailer.sent_count(), 1);
    }
}
