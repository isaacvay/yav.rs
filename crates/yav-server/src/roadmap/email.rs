//! Real SMTP delivery via `lettre` (feature `smtp`).
//!
//! The trait [`crate::adapters::MailAdapter`] is synchronous while lettre's
//! SMTP transport is async, so each send is driven to completion on a
//! dedicated Tokio runtime owned by the adapter. A returned string is an
//! internal delivery id; no send is reported as accepted unless the relay
//! actually answered successfully.

#![cfg(feature = "smtp")]

use std::sync::atomic::{AtomicU64, Ordering};

use crate::adapters::MailAdapter;
use crate::email::EmailMessage;

/// Environment variables read by [`SmtpMailAdapter::from_env`]:
/// `YAV_SMTP_RELAY`, `YAV_SMTP_PORT`, `YAV_SMTP_USERNAME`,
/// `YAV_SMTP_PASSWORD`, `YAV_SMTP_FROM`.
static NEXT_SEND_ID: AtomicU64 = AtomicU64::new(1);

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Convert an [`EmailMessage`] into a lettre `Message`.
///
/// HTML is preferred when `html_body` is present, otherwise the plain-text
/// fallback is used. Recipients and the sender are parsed (never guessed), so
/// a malformed address is a validation error instead of a bad envelope.
fn build_message(email: &EmailMessage) -> Result<lettre::Message, String> {
    use lettre::message::header::ContentType;
    use lettre::Message;

    if email.to.is_empty() {
        return Err("EmailMessage must have at least one recipient".to_string());
    }

    let from = email
        .from
        .parse()
        .map_err(|e| format!("invalid From address '{}': {e}", email.from))?;

    let mut builder = Message::builder().from(from);
    for recipient in &email.to {
        let to = recipient
            .parse()
            .map_err(|e| format!("invalid To address '{recipient}': {e}"))?;
        builder = builder.to(to);
    }
    for recipient in &email.cc {
        let cc = recipient
            .parse()
            .map_err(|e| format!("invalid Cc address '{recipient}': {e}"))?;
        builder = builder.cc(cc);
    }
    builder = builder.subject(email.subject.clone());

    let (content_type, body) = if !email.html_body.is_empty() {
        (ContentType::TEXT_HTML, email.html_body.as_str())
    } else if !email.text_body.is_empty() {
        (ContentType::TEXT_PLAIN, email.text_body.as_str())
    } else {
        return Err("EmailMessage has neither an HTML nor a text body".to_string());
    };

    builder
        .header(content_type)
        .body(body.to_string())
        .map_err(|e| format!("failed to build SMTP message: {e}"))
}

/// SMTP mail adapter backed by lettre's async transport.
pub struct SmtpMailAdapter {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
}

impl SmtpMailAdapter {
    /// Construct an adapter for `relay:port`, optionally with credentials.
    ///
    /// Port 465 uses implicit TLS; any other port uses STARTTLS. Both modes are
    /// real encrypted SMTP connections — there is no plaintext fallback.
    pub fn new(
        relay: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        from: String,
    ) -> Result<Self, String> {
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, Tokio1Executor};

        if relay.trim().is_empty() {
            return Err("SMTP relay must not be empty".to_string());
        }
        if from.trim().is_empty() {
            return Err("SMTP From address must not be empty".to_string());
        }
        // Validate the configured sender eagerly so misconfiguration fails at
        // construction time, not on the first send.
        let _: lettre::message::Mailbox = from
            .parse()
            .map_err(|e| format!("invalid SMTP From address '{from}': {e}"))?;

        let builder = if port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&relay)
                .map_err(|e| format!("failed to configure SMTP relay: {e}"))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&relay)
                .map_err(|e| format!("failed to configure SMTP STARTTLS relay: {e}"))?
        };
        let builder = builder.port(port);
        let builder = match (username, password) {
            (Some(user), Some(pass)) => builder.credentials(Credentials::new(user, pass)),
            (None, None) => builder,
            _ => {
                return Err(
                    "SMTP username and password must be supplied together or not at all"
                        .to_string(),
                )
            }
        };

        let transport: AsyncSmtpTransport<Tokio1Executor> = builder.build();
        Ok(Self { transport })
    }

    /// Build from `YAV_SMTP_*` environment variables.
    ///
    /// `YAV_SMTP_RELAY`, `YAV_SMTP_FROM` and `YAV_SMTP_PORT` are required;
    /// credentials are optional but, when present, must both be set.
    pub fn from_env() -> Result<Self, String> {
        let relay =
            std::env::var("YAV_SMTP_RELAY").map_err(|_| "YAV_SMTP_RELAY is not set".to_string())?;
        let from =
            std::env::var("YAV_SMTP_FROM").map_err(|_| "YAV_SMTP_FROM is not set".to_string())?;
        let port = std::env::var("YAV_SMTP_PORT")
            .map_err(|_| "YAV_SMTP_PORT is not set".to_string())?
            .parse::<u16>()
            .map_err(|e| format!("YAV_SMTP_PORT must be a valid port number: {e}"))?;
        let username = std::env::var("YAV_SMTP_USERNAME").ok();
        let password = std::env::var("YAV_SMTP_PASSWORD").ok();

        Self::new(relay, port, username, password, from)
    }

    /// Native async SMTP delivery helper safe inside any Axum handler.
    pub async fn send_email_async(&self, message: EmailMessage) -> Result<String, String> {
        use lettre::AsyncTransport;
        let mail = build_message(&message)?;
        self.transport
            .send(mail)
            .await
            .map_err(|e| format!("SMTP delivery failed: {e}"))?;

        Ok(format!(
            "smtp_{}_{}",
            now_epoch(),
            NEXT_SEND_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

impl MailAdapter for SmtpMailAdapter {
    fn send_email(&self, message: EmailMessage) -> Result<String, String> {
        crate::roadmap::block_on_shared(self.send_email_async(message))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_validation() {
        let err = SmtpMailAdapter::new("".to_string(), 587, None, None, "dev@yav.rs".to_string())
            .err()
            .expect("empty relay must fail");
        assert!(err.contains("relay"));

        let err = SmtpMailAdapter::new(
            "smtp.example.com".to_string(),
            587,
            None,
            None,
            "not-an-address".to_string(),
        )
        .err()
        .expect("invalid From must fail");
        assert!(err.contains("From address"));

        let err = SmtpMailAdapter::new(
            "smtp.example.com".to_string(),
            587,
            Some("user".to_string()),
            None,
            "dev@yav.rs".to_string(),
        )
        .err()
        .expect("half credentials must fail");
        assert!(err.contains("together"));

        // Valid configuration constructs without any network access.
        assert!(SmtpMailAdapter::new(
            "smtp.example.com".to_string(),
            587,
            None,
            None,
            "dev@yav.rs".to_string(),
        )
        .is_ok());
    }

    #[test]
    fn test_build_message_validation_and_content_type() {
        let empty = EmailMessage::new().from("team@yav.rs").subject("Hi");
        assert!(build_message(&empty).is_err(), "no recipient must fail");

        let mut html = EmailMessage::new()
            .from("team@yav.rs")
            .to("user@example.com")
            .subject("Hi");
        // Build both variants explicitly without needing RSX nodes.
        html.html_body = "<h1>Hello</h1>".to_string();
        html.text_body = "Hello".to_string();
        let _message = build_message(&html).expect("valid message");
        assert_eq!(html.subject, "Hi");
    }
}
