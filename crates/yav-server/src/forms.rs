//! Form parsing, validation helpers, and action responses.
//!
//! Provides ergonomic utilities for handling form submissions, collecting
//! field validation errors, and rendering error messages inline with inputs.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::BTreeMap;
use yav_core::Node;

/// A map of field validation errors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormErrors {
    errors: BTreeMap<String, Vec<String>>,
}

impl FormErrors {
    /// Create a new empty error collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error message for a specific field.
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.errors
            .entry(field.into())
            .or_default()
            .push(message.into());
        self
    }

    /// Check if any errors have been recorded.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all error messages for a specific field.
    pub fn get(&self, field: &str) -> Option<&[String]> {
        self.errors.get(field).map(|v| v.as_slice())
    }

    /// Get the first error message for a field, if any.
    pub fn first(&self, field: &str) -> Option<&str> {
        self.get(field).and_then(|v| v.first().map(|s| s.as_str()))
    }

    /// Get all errors grouped by field.
    pub fn all(&self) -> &BTreeMap<String, Vec<String>> {
        &self.errors
    }

    /// Validate that a field string is non-empty after trimming whitespace.
    pub fn require(&mut self, field: &str, value: &str, message: impl Into<String>) -> &mut Self {
        if value.trim().is_empty() {
            self.add(field, message);
        }
        self
    }

    /// Validate that a field meets a minimum length.
    pub fn min_length(
        &mut self,
        field: &str,
        value: &str,
        min: usize,
        message: impl Into<String>,
    ) -> &mut Self {
        if value.trim().len() < min {
            self.add(field, message);
        }
        self
    }

    /// Validate that a field matches a basic valid email format.
    pub fn email(&mut self, field: &str, value: &str, message: impl Into<String>) -> &mut Self {
        let trimmed = value.trim();
        if !trimmed.contains('@') || !trimmed.contains('.') || trimmed.len() < 5 {
            self.add(field, message);
        }
        self
    }

    /// Validate anti-CSRF token using constant-time comparison. Adds an error if validation fails.
    pub fn csrf(
        &mut self,
        expected: &str,
        submitted: &str,
        message: impl Into<String>,
    ) -> &mut Self {
        if !verify_csrf_token(expected, submitted) {
            self.add(CSRF_FIELD_NAME, message);
        }
        self
    }

    /// Validate an uploaded file against maximum size and allowed extensions.
    pub fn validate_file(
        &mut self,
        file: &UploadedFile,
        max_bytes: usize,
        allowed_extensions: &[&str],
        size_error_msg: impl Into<String>,
        ext_error_msg: impl Into<String>,
    ) -> &mut Self {
        file.validate_max_size(max_bytes, self, size_error_msg);
        file.validate_extension(allowed_extensions, self, ext_error_msg);
        self
    }
}

/// An uploaded file extracted from a multipart form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadedFile {
    /// Field name in the form submission.
    pub field_name: String,
    /// Original client filename (e.g., `"avatar.png"`).
    pub file_name: Option<String>,
    /// Content-Type header (e.g., `"image/png"`).
    pub content_type: Option<String>,
    /// Raw byte payload of the uploaded file.
    pub bytes: Vec<u8>,
}

impl UploadedFile {
    /// Create a new uploaded file.
    pub fn new(
        field_name: impl Into<String>,
        file_name: Option<String>,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            field_name: field_name.into(),
            file_name,
            content_type,
            bytes,
        }
    }

    /// Size of the uploaded payload in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` if the uploaded file is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Extracts file extension from `file_name` in lowercase (e.g. `"png"`).
    pub fn extension(&self) -> Option<String> {
        self.file_name.as_ref().and_then(|name| {
            std::path::Path::new(name)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
        })
    }

    /// Validates that the file size does not exceed `max_bytes`.
    pub fn validate_max_size(
        &self,
        max_bytes: usize,
        errors: &mut FormErrors,
        message: impl Into<String>,
    ) -> bool {
        if self.len() > max_bytes {
            errors.add(&self.field_name, message);
            false
        } else {
            true
        }
    }

    /// Validates that the file extension is in an allowed list of extensions.
    pub fn validate_extension(
        &self,
        allowed: &[&str],
        errors: &mut FormErrors,
        message: impl Into<String>,
    ) -> bool {
        let ext = self.extension().unwrap_or_default();
        if !allowed.iter().any(|&a| a.eq_ignore_ascii_case(&ext)) {
            errors.add(&self.field_name, message);
            false
        } else {
            true
        }
    }
}

/// Canonical HTML form input name for anti-CSRF token.
pub const CSRF_FIELD_NAME: &str = "_csrf";

/// Canonical HTTP header name for anti-CSRF token in AJAX/HTMX submissions.
pub const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// Render a hidden anti-CSRF input tag suitable for inclusion inside an HTML form.
pub fn csrf_field(token: impl Into<String>) -> Node {
    Node::Element(
        yav_core::Element::new("input")
            .attr("type", "hidden")
            .attr("name", CSRF_FIELD_NAME)
            .attr("value", token.into()),
    )
}

/// Constant-time string equality check to prevent timing attacks.
pub fn constant_time_eq_str(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Validate a submitted anti-CSRF token against the expected token.
pub fn verify_csrf_token(expected: &str, submitted: &str) -> bool {
    if expected.is_empty() || submitted.is_empty() {
        return false;
    }
    constant_time_eq_str(expected, submitted)
}

/// Standard response from a form submission or server action.
pub enum ActionResponse {
    /// Redirect to another page upon successful submission (Post/Redirect/Get pattern).
    Redirect(String),
    /// Re-render with validation errors.
    Errors(FormErrors),
    /// Return custom rendered HTML.
    Html(Node),
}

impl ActionResponse {
    /// Construct a redirect response to the given target URL.
    pub fn redirect(url: impl Into<String>) -> Self {
        Self::Redirect(url.into())
    }

    /// Construct a validation error response.
    pub fn errors(errors: FormErrors) -> Self {
        Self::Errors(errors)
    }

    /// Construct an HTML fragment response (ideal for HTMX swap).
    pub fn html(node: impl yav_core::IntoNode) -> Self {
        Self::Html(node.into_node())
    }
}

/// Convenient result alias for server actions returning either an [`ActionResponse`] or [`FormErrors`].
pub type ActionResult = Result<ActionResponse, FormErrors>;

impl IntoResponse for ActionResponse {
    fn into_response(self) -> Response {
        match self {
            ActionResponse::Redirect(url) => match HeaderValue::try_from(&url) {
                Ok(loc) => (StatusCode::SEE_OTHER, [(header::LOCATION, loc)]).into_response(),
                Err(_) => (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    "Invalid redirect URL",
                )
                    .into_response(),
            },
            ActionResponse::Errors(errors) => {
                let json = serde_json::to_string(&errors.all()).unwrap_or_default();
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                    json,
                )
                    .into_response()
            }
            ActionResponse::Html(node) => {
                let html = yav_core::render_to_string(&node);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    html,
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_errors() {
        let mut errors = FormErrors::new();
        assert!(!errors.has_errors());

        errors.add("email", "Email requis");
        errors.add("email", "Format invalide");
        errors.add("password", "Trop court");

        assert!(errors.has_errors());
        assert_eq!(errors.first("email"), Some("Email requis"));
        assert_eq!(errors.get("email").unwrap().len(), 2);
        assert_eq!(errors.first("password"), Some("Trop court"));
        assert_eq!(errors.first("username"), None);
    }

    #[test]
    fn test_form_validation_helpers() {
        let mut errors = FormErrors::new();
        errors.require("name", "", "Name is required");
        errors.require("title", "  ", "Title is required");
        errors.require("valid", "hello", "Should not trigger");

        errors.min_length("password", "pass", 8, "Password must be at least 8 chars");
        errors.email("email", "not-an-email", "Invalid email address");

        assert!(errors.has_errors());
        assert_eq!(errors.first("name"), Some("Name is required"));
        assert_eq!(errors.first("title"), Some("Title is required"));
        assert_eq!(errors.first("valid"), None);
        assert_eq!(
            errors.first("password"),
            Some("Password must be at least 8 chars")
        );
        assert_eq!(errors.first("email"), Some("Invalid email address"));
    }

    #[test]
    fn test_csrf_field_and_verification() {
        let token = "random_csrf_token_abc123";
        let node = csrf_field(token);
        let html = yav_core::render_to_string(&node);
        assert!(html.contains(r#"type="hidden""#));
        assert!(html.contains(r#"name="_csrf""#));
        assert!(html.contains(&format!(r#"value="{token}""#)));

        assert!(verify_csrf_token(token, token));
        assert!(!verify_csrf_token(token, "wrong_token"));
        assert!(!verify_csrf_token("", ""));

        let mut errors = FormErrors::new();
        errors.csrf(token, "wrong_token", "CSRF token mismatch");
        assert!(errors.has_errors());
        assert_eq!(errors.first(CSRF_FIELD_NAME), Some("CSRF token mismatch"));

        let mut valid_errors = FormErrors::new();
        valid_errors.csrf(token, token, "Should not fail");
        assert!(!valid_errors.has_errors());
    }

    #[test]
    fn test_action_response_redirect_safe() {
        let resp = ActionResponse::redirect("/dashboard").into_response();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/dashboard");

        // Header injection / invalid URL characters must not panic and must return 400 Bad Request
        let bad_resp =
            ActionResponse::redirect("https://example.com/\r\nSet-Cookie: evil=1").into_response();
        assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_uploaded_file_validation() {
        let file = UploadedFile::new(
            "avatar",
            Some("my_photo.PNG".into()),
            Some("image/png".into()),
            vec![0u8; 1024 * 50], // 50 KB
        );

        assert_eq!(file.extension(), Some("png".to_string()));
        assert_eq!(file.len(), 50 * 1024);

        let mut errors = FormErrors::new();
        // Allowed: png, jpg, max 100 KB -> passes
        errors.validate_file(
            &file,
            100 * 1024,
            &["png", "jpg"],
            "File too large",
            "Invalid file type",
        );
        assert!(!errors.has_errors());

        // Max 20 KB -> fails size check
        errors.validate_file(
            &file,
            20 * 1024,
            &["png", "jpg"],
            "File exceeds 20KB",
            "Invalid file type",
        );
        assert!(errors.has_errors());
        assert_eq!(errors.first("avatar"), Some("File exceeds 20KB"));

        // Allowed: pdf only -> fails extension check
        let mut ext_errors = FormErrors::new();
        ext_errors.validate_file(
            &file,
            100 * 1024,
            &["pdf"],
            "File too large",
            "Only PDF allowed",
        );
        assert!(ext_errors.has_errors());
        assert_eq!(ext_errors.first("avatar"), Some("Only PDF allowed"));
    }
}
