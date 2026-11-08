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

static CSRF_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Extracted URL-encoded form fields (`application/x-www-form-urlencoded`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormData(pub std::collections::HashMap<String, String>);

impl FormData {
    /// Create a new empty `FormData` map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieve a form field value by name.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    /// Retrieve a form field value by name, or `""` if absent.
    pub fn get_or_empty(&self, key: &str) -> &str {
        self.get(key).unwrap_or("")
    }

    /// Retrieve a boolean checkbox/toggle field (`"true"`, `"1"`, `"on"`, `"yes"` -> `true`).
    pub fn get_bool(&self, key: &str) -> bool {
        matches!(
            self.get_or_empty(key).trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "on" | "yes"
        )
    }

    /// Parse a field into any type implementing [`std::str::FromStr`].
    pub fn get_parsed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get(key).and_then(|v| v.trim().parse::<T>().ok())
    }

    /// Parse and validate form fields into a typed struct implementing [`FromFormData`].
    pub fn parse_validated<T: FromFormData>(&self) -> Result<T, FormErrors> {
        T::from_form(self)
    }

    /// Parse a JSON object byte slice into [`FormData`].
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let val: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| "Invalid JSON in form body")?;
        let obj = val
            .as_object()
            .ok_or("Expected a JSON object for FormData")?;
        let mut map = std::collections::HashMap::with_capacity(obj.len());
        for (k, v) in obj {
            let s = match v {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            map.insert(k.clone(), s);
        }
        Ok(Self(map))
    }

    /// Parse a `multipart/form-data` payload with the given boundary into [`FormData`].
    pub fn from_multipart_bytes(bytes: &[u8], boundary: &str) -> Result<Self, &'static str> {
        let delim = format!("--{boundary}").into_bytes();
        let delim_len = delim.len();
        let mut map = std::collections::HashMap::new();

        let mut pos = 0;
        let mut part_indices = Vec::new();

        while let Some(idx) = bytes[pos..]
            .windows(delim_len)
            .position(|w| w == delim.as_slice())
        {
            part_indices.push(pos + idx);
            pos += idx + delim_len;
        }

        for i in 0..part_indices.len() {
            let start = part_indices[i] + delim_len;
            let end = if i + 1 < part_indices.len() {
                part_indices[i + 1]
            } else {
                bytes.len()
            };
            let mut part = &bytes[start..end];
            // Check for terminal boundary `--`
            if part.starts_with(b"--") {
                break;
            }
            // Strip leading CRLF or LF
            if part.starts_with(b"\r\n") {
                part = &part[2..];
            } else if part.starts_with(b"\n") {
                part = &part[1..];
            }

            // Find header-body separator (\r\n\r\n or \n\n)
            let header_sep_idx = part
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .map(|p| (p, 4))
                .or_else(|| part.windows(2).position(|w| w == b"\n\n").map(|p| (p, 2)));

            if let Some((sep_pos, sep_len)) = header_sep_idx {
                let header_bytes = &part[..sep_pos];
                let mut body_bytes = &part[sep_pos + sep_len..];

                // Strip trailing CRLF or LF before next delimiter
                if body_bytes.ends_with(b"\r\n") {
                    body_bytes = &body_bytes[..body_bytes.len() - 2];
                } else if body_bytes.ends_with(b"\n") {
                    body_bytes = &body_bytes[..body_bytes.len() - 1];
                }

                if let Ok(headers_str) = std::str::from_utf8(header_bytes) {
                    let mut field_name: Option<String> = None;
                    for line in headers_str.lines() {
                        let lower = line.to_ascii_lowercase();
                        if lower.starts_with("content-disposition:") {
                            if let Some(name_idx) = line.find("name=\"") {
                                let after = &line[name_idx + 6..];
                                if let Some(end_q) = after.find('"') {
                                    field_name = Some(after[..end_q].to_string());
                                }
                            }
                        }
                    }
                    if let Some(name) = field_name {
                        let val = match std::str::from_utf8(body_bytes) {
                            Ok(s) => s.to_string(),
                            Err(_) => String::from_utf8_lossy(body_bytes).into_owned(),
                        };
                        map.insert(name, val);
                    }
                }
            }
        }
        Ok(Self(map))
    }
}

/// Trait for types that can be parsed and validated from form submission data.
pub trait FromFormData: Sized {
    /// Parse and validate fields from [`FormData`], returning `Ok(Self)` or `Err(FormErrors)`.
    fn from_form(form: &FormData) -> Result<Self, FormErrors>;
}

impl<S> axum::extract::FromRequest<S> for FormData
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(
        req: axum::extract::Request,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024)
            .await
            .map_err(|_| (StatusCode::PAYLOAD_TOO_LARGE, "Form body too large"))?;

        let ct_lower = content_type.to_ascii_lowercase();
        if ct_lower.starts_with("application/json") {
            return Self::from_json_bytes(&bytes).map_err(|msg| (StatusCode::BAD_REQUEST, msg));
        }

        if ct_lower.starts_with("multipart/form-data") {
            if let Some(b_idx) = content_type.find("boundary=") {
                let boundary = content_type[b_idx + 9..]
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"');
                if !boundary.is_empty() {
                    return Self::from_multipart_bytes(&bytes, boundary)
                        .map_err(|msg| (StatusCode::BAD_REQUEST, msg));
                }
            }
        }

        let body_str = std::str::from_utf8(&bytes)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UTF-8 in form body"))?;
        Ok(FormData(yav_router::parse_query_string(body_str)))
    }
}

/// Session-bound anti-CSRF token manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Retrieve the existing CSRF token from the user's [`Session`](crate::session::Session),
    /// or generate and store a new 64-character hex token seeded from OS CSPRNG entropy if none exists.
    pub fn from_session(session: &crate::session::Session) -> Self {
        if let Some(existing) = session.get("_csrf_token") {
            if !existing.is_empty() {
                return Self(existing);
            }
        }
        let mut random_bytes = [0u8; 32];
        let seed = if getrandom::getrandom(&mut random_bytes).is_ok() {
            let mut hex = String::with_capacity(64);
            for b in random_bytes {
                use std::fmt::Write;
                let _ = write!(&mut hex, "{:02x}", b);
            }
            hex
        } else {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let seq = CSRF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let pid = std::process::id();
            format!("yav-csrf-{nanos}-{seq}-{pid}")
        };
        let secret = std::env::var("SESSION_SECRET")
            .or_else(|_| std::env::var("YAV_SECRET"))
            .map(|s| s.into_bytes())
            .unwrap_or_else(|_| crate::session::DEFAULT_DEV_SESSION_SECRET.to_vec());
        let token = crate::session::compute_hmac_sha256(seed.as_bytes(), &secret);
        session.set("_csrf_token", token.clone());
        Self(token)
    }

    /// Return the raw token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Render a hidden `<input type="hidden" name="_csrf" value="..." />` Virtual DOM node.
    pub fn hidden_input(&self) -> Node {
        csrf_field(&self.0)
    }

    /// Verify the `_csrf` field inside a submitted [`FormData`] in constant time.
    pub fn verify(&self, form: &FormData) -> bool {
        verify_csrf_token(&self.0, form.get_or_empty(CSRF_FIELD_NAME))
    }

    /// Verify a raw submitted token string in constant time.
    pub fn verify_token(&self, submitted: &str) -> bool {
        verify_csrf_token(&self.0, submitted)
    }
}

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
    crate::session::constant_time_eq(a.as_bytes(), b.as_bytes())
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
            ActionResponse::Redirect(url) => {
                if !crate::handler::is_safe_redirect_target(&url) {
                    return (
                        StatusCode::BAD_REQUEST,
                        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                        "Invalid or unsafe redirect URL",
                    )
                        .into_response();
                }
                match HeaderValue::try_from(url.trim()) {
                    Ok(loc) => (StatusCode::SEE_OTHER, [(header::LOCATION, loc)]).into_response(),
                    Err(_) => (
                        StatusCode::BAD_REQUEST,
                        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                        "Invalid redirect URL",
                    )
                        .into_response(),
                }
            }
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

    #[tokio::test]
    async fn test_form_data_from_json_and_multipart_requests() {
        use axum::extract::FromRequest;

        // 1. JSON payload
        let json_req = axum::extract::Request::builder()
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(
                r#"{"title":"Ship v2","priority":5,"published":true}"#,
            ))
            .unwrap();
        let form = FormData::from_request(json_req, &()).await.unwrap();
        assert_eq!(form.get("title"), Some("Ship v2"));
        assert_eq!(form.get_parsed::<u32>("priority"), Some(5));
        assert!(form.get_bool("published"));

        // 2. Multipart payload
        let mp_body = "--yavbound\r\nContent-Disposition: form-data; name=\"username\"\r\n\r\nalice\r\n--yavbound\r\nContent-Disposition: form-data; name=\"role\"\r\n\r\nadmin\r\n--yavbound--\r\n";
        let mp_req = axum::extract::Request::builder()
            .method("POST")
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=yavbound",
            )
            .body(axum::body::Body::from(mp_body))
            .unwrap();
        let mp_form = FormData::from_request(mp_req, &()).await.unwrap();
        assert_eq!(mp_form.get("username"), Some("alice"));
        assert_eq!(mp_form.get("role"), Some("admin"));

        // 3. Binary multipart payload
        let mut binary_mp = Vec::new();
        binary_mp.extend_from_slice(b"--binbound\r\nContent-Disposition: form-data; name=\"binary_file\"\r\n\r\n\xFF\xFE\xFD\xFC\r\n--binbound--\r\n");
        let bin_res = FormData::from_multipart_bytes(&binary_mp, "binbound").unwrap();
        assert!(bin_res.get("binary_file").is_some());
    }
}
