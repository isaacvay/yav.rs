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

/// Result of parsing a `multipart/form-data` body: text fields **and** binary
/// file parts. Use this extractor when a form actually uploads files; the
/// legacy [`FormData::from_multipart_bytes`] only exposes the text view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultipartForm {
    /// Text fields (all parts, including a lossy UTF-8 view of binary parts).
    pub fields: FormData,
    /// Parts that carried a `filename=` parameter, with their raw bytes.
    pub files: Vec<UploadedFile>,
}

impl MultipartForm {
    /// Parse a complete multipart body. Rejects malformed boundaries, headers
    /// and parts instead of silently returning an empty/partial form.
    pub fn from_bytes(bytes: &[u8], boundary: &str) -> Result<Self, &'static str> {
        parse_multipart_bytes(bytes, boundary)
    }

    /// First uploaded file for a given field name, if any.
    pub fn file(&self, field: &str) -> Option<&UploadedFile> {
        self.files.iter().find(|f| f.field_name == field)
    }

    /// All uploaded files for a given field name.
    pub fn files_for<'a>(&'a self, field: &'a str) -> impl Iterator<Item = &'a UploadedFile> {
        self.files.iter().filter(move |f| f.field_name == field)
    }

    /// Borrow the text fields as [`FormData`].
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key)
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

/// RFC 2046 boundary limits. A caller-controlled boundary must not contain
/// line breaks (header/body smuggling) or be arbitrarily long.
const MAX_BOUNDARY_LEN: usize = 70;
const MAX_MULTIPART_PARTS: usize = 256;

fn validate_multipart_boundary(boundary: &str) -> Result<(), &'static str> {
    if boundary.is_empty() || boundary.len() > MAX_BOUNDARY_LEN {
        return Err("Invalid multipart boundary length");
    }
    // RFC 2046 bcharsnospace / bchars, plus the common characters browsers use.
    if !boundary.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'\'' | b'(' | b')' | b'+' | b'_' | b',' | b'-' | b'.' | b'/' | b':' | b'=' | b'?'
            )
    }) {
        return Err("Invalid multipart boundary characters");
    }
    Ok(())
}

/// Strip directory components and control characters from a client-supplied
/// filename. Never let `../../etc/passwd` reach a filesystem API downstream.
fn sanitize_upload_filename(name: &str) -> Option<String> {
    let replaced = name.replace('\\', "/");
    let base = replaced.rsplit('/').next().unwrap_or_default();
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != '"')
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        None
    } else {
        Some(cleaned)
    }
}

/// Read a quoted (or token) parameter from a `Content-Disposition` value.
fn header_param(value: &str, wanted: &str) -> Option<String> {
    for segment in value.split(';').skip(1) {
        let segment = segment.trim();
        let Some((key, raw)) = segment.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !key.eq_ignore_ascii_case(wanted) {
            // RFC 5987 extended parameters, e.g. `filename*=UTF-8''avatar.png`.
            if key.eq_ignore_ascii_case(&format!("{wanted}*")) {
                let raw = raw.trim();
                if let Some((_, encoded)) = raw.rsplit_once("''") {
                    let decoded = percent_decode_str(encoded);
                    if !decoded.is_empty() {
                        return sanitize_upload_filename(&decoded);
                    }
                }
            }
            continue;
        }
        let raw = raw.trim();
        if !raw.starts_with('"') {
            return Some(raw.to_string());
        }
        let mut out = String::new();
        let mut chars = raw[1..].chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                }
                _ => out.push(c),
            }
        }
        return Some(out);
    }
    None
}

fn percent_decode_str(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_multipart_part(part: &[u8], form: &mut MultipartForm) -> Result<(), &'static str> {
    let (sep_pos, sep_len) = find_subslice(part, b"\r\n\r\n")
        .map(|p| (p, 4))
        .or_else(|| find_subslice(part, b"\n\n").map(|p| (p, 2)))
        .ok_or("Malformed multipart part: missing header separator")?;

    let headers = std::str::from_utf8(&part[..sep_pos])
        .map_err(|_| "Malformed multipart part: non-UTF-8 headers")?;
    let body = &part[sep_pos + sep_len..];

    let mut name: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;

    for line in headers.lines() {
        let Some((key, value)) = line.split_once(':') else {
            return Err("Malformed multipart part header");
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "content-disposition" => {
                if !value.to_ascii_lowercase().starts_with("form-data") {
                    return Err("Unsupported multipart Content-Disposition");
                }
                name = header_param(value, "name");
                filename = header_param(value, "filename").and_then(|f| {
                    let sanitized = sanitize_upload_filename(&f);
                    // An empty filename means "no file selected"; keep it as an
                    // empty upload so callers can decide what to do.
                    sanitized
                });
            }
            "content-type" => content_type = Some(value.to_string()),
            _ => {}
        }
    }

    let name = name.ok_or("Multipart part is missing its field name")?;
    // Keep the historical text view: even binary parts are visible (lossily)
    // through FormData so existing callers keep working.
    form.fields
        .0
        .insert(name.clone(), String::from_utf8_lossy(body).into_owned());

    if filename.is_some() {
        form.files.push(UploadedFile::new(
            name,
            filename,
            content_type,
            body.to_vec(),
        ));
    }
    Ok(())
}

/// Parse a `multipart/form-data` body into text fields **and** real binary
/// [`UploadedFile`] parts. Unlike the previous ad-hoc string splitting, this
/// follows RFC 2046 delimiters (CRLF-prefixed boundaries, transport padding,
/// closing `--`), parses each part's headers and never mangles file bytes.
pub fn parse_multipart_bytes(bytes: &[u8], boundary: &str) -> Result<MultipartForm, &'static str> {
    validate_multipart_boundary(boundary)?;
    let delim = format!("--{boundary}");
    let delim_bytes = delim.as_bytes();
    let needle = format!("\r\n{delim}");

    let mut positions = Vec::new();
    if bytes.starts_with(delim_bytes) {
        positions.push(0);
    }
    let mut search = 0;
    while let Some(rel) = find_subslice(&bytes[search..], needle.as_bytes()) {
        let at = search + rel + 2; // skip the CRLF, point at "--"
        positions.push(at);
        search = at + delim_bytes.len();
        if positions.len() > MAX_MULTIPART_PARTS {
            return Err("Too many multipart parts");
        }
    }
    if positions.is_empty() {
        return Err("Multipart boundary not found in body");
    }

    let mut form = MultipartForm::default();
    for (i, &boundary_pos) in positions.iter().enumerate() {
        let start = boundary_pos + delim_bytes.len();
        if bytes.get(start..start + 2) == Some(b"--") {
            break; // closing boundary
        }
        let mut p = start;
        while matches!(bytes.get(p), Some(b' ' | b'\t')) {
            p += 1; // RFC 2046 transport padding
        }
        if bytes.get(p..p + 2) == Some(b"\r\n") {
            p += 2;
        } else if bytes.get(p) == Some(&b'\n') {
            p += 1;
        } else {
            return Err("Malformed multipart boundary line");
        }

        let end = positions.get(i + 1).copied().unwrap_or(bytes.len());
        let mut body_end = end;
        if end >= 2 && bytes.get(end - 2..end) == Some(b"\r\n") {
            body_end -= 2;
        } else if bytes.get(end - 1) == Some(&b'\n') {
            body_end -= 1;
        }
        let part = bytes
            .get(p..body_end)
            .ok_or("Malformed multipart part boundaries")?;
        parse_multipart_part(part, &mut form)?;
    }
    Ok(form)
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

/// Resolve the secret used to derive a CSRF token.
///
/// In production a missing `SESSION_SECRET` / `YAV_SECRET` is an **explicit
/// error** instead of a silent fallback to the well-known development constant.
fn configured_csrf_secret() -> Result<Vec<u8>, String> {
    if let Ok(s) = std::env::var("SESSION_SECRET").or_else(|_| std::env::var("YAV_SECRET")) {
        if !s.is_empty() {
            return Ok(s.into_bytes());
        }
    }
    if crate::session::is_production_mode() {
        return Err(
            "CSRF: no SESSION_SECRET/YAV_SECRET configured in production; refusing to derive a \
             token from DEFAULT_DEV_SESSION_SECRET. Honour the configured \
             crate::session::SessionSecret by calling \
             CsrfToken::from_session_with_secret from the CSRF middleware (middleware.rs)."
                .to_string(),
        );
    }
    Ok(crate::session::DEFAULT_DEV_SESSION_SECRET.to_vec())
}

impl CsrfToken {
    /// Retrieve the existing CSRF token from the user's [`Session`](crate::session::Session),
    /// or generate and store a new 64-character hex token seeded from OS CSPRNG entropy if none exists.
    ///
    /// # Production / configuration limitation
    /// This helper only receives a `&Session`, so it cannot see request
    /// extensions or application state: the only secret it can reach is
    /// `SESSION_SECRET` / `YAV_SECRET` from the environment. A secret set via
    /// `YavApp::session_secret()` (the `crate::session::SessionSecret`
    /// extension) is therefore **not** visible here.
    ///
    /// To avoid silently degrading defense-in-depth, a missing secret in
    /// production (`crate::session::is_production_mode()`) is now an explicit
    /// error: it is surfaced (logged) and the call **fails closed** — no token
    /// is ever bound to the session, so the CSRF middleware rejects every
    /// state-changing request until a real secret is configured. It is
    /// **never** a silent fallback to `DEFAULT_DEV_SESSION_SECRET`.
    ///
    /// The final wiring must be done in `middleware.rs` (outside this file's
    /// scope): the CSRF middleware should read the `SessionSecret` extension
    /// and call [`Self::from_session_with_secret`] instead of relying on this
    /// environment-only helper.
    pub fn from_session(session: &crate::session::Session) -> Self {
        // An already-bound token needs no secret at all: keep it so existing
        // sessions are not broken by the production guard.
        if let Some(existing) = session.get("_csrf_token") {
            if !existing.is_empty() {
                return Self(existing);
            }
        }
        let secret = match configured_csrf_secret() {
            Ok(secret) => secret,
            Err(err) => {
                // Explicit error + fail closed. Crucially, nothing is written
                // to the session, so the CSRF middleware has no token to
                // accept. We still return an unpredictable, unbound value so
                // callers holding a `CsrfToken` never see a silently weak or
                // empty token.
                tracing::error!("{}", err);
                return Self(Self::random_seed());
            }
        };
        Self::from_session_with_secret(session, &secret)
    }

    /// Generate (or reuse) a CSRF token using an explicitly supplied secret.
    ///
    /// This is the secure entry point for callers that can reach request
    /// extensions / application state: pass
    /// `crate::session::SessionSecret::as_bytes()` so a secret configured with
    /// `YavApp::session_secret()` is honoured instead of the environment.
    /// Unlike [`Self::from_session`], this method never needs a silent fallback.
    pub fn from_session_with_secret(session: &crate::session::Session, secret: &[u8]) -> Self {
        if let Some(existing) = session.get("_csrf_token") {
            if !existing.is_empty() {
                return Self(existing);
            }
        }
        let seed = Self::random_seed();
        let token = crate::session::compute_hmac_sha256(seed.as_bytes(), secret);
        session.set("_csrf_token", token.clone());
        Self(token)
    }

    fn random_seed() -> String {
        let mut random_bytes = [0u8; 32];
        if getrandom::getrandom(&mut random_bytes).is_ok() {
            let mut hex = String::with_capacity(64);
            for b in random_bytes {
                use std::fmt::Write;
                let _ = write!(&mut hex, "{:02x}", b);
            }
            hex
        } else {
            tracing::error!("OS CSPRNG getrandom failed while generating CSRF seed");
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let seq = CSRF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let pid = std::process::id();
            format!("yav-csrf-{nanos}-{seq}-{pid}")
        }
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
    /// Redirect to a local page upon successful submission (Post/Redirect/Get pattern).
    Redirect(String),
    /// Redirect to an external URL whose origin is explicitly allowlisted.
    ///
    /// This preserves external redirects (OAuth, payment gateways) without
    /// reintroducing an open-redirect vulnerability.
    RedirectAllowlisted(String, Vec<String>),
    /// Re-render with validation errors.
    Errors(FormErrors),
    /// Return custom rendered HTML.
    Html(Node),
}

impl ActionResponse {
    /// Construct a redirect response to a local application path.
    pub fn redirect(url: impl Into<String>) -> Self {
        Self::Redirect(url.into())
    }

    /// Construct a redirect to an external URL, permitted only if its origin is
    /// present in `allowed_origins` (e.g. `["https://accounts.example.com"]`).
    pub fn redirect_allowlisted(
        url: impl Into<String>,
        allowed_origins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::RedirectAllowlisted(
            url.into(),
            allowed_origins.into_iter().map(Into::into).collect(),
        )
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
                // Form actions commonly interpolate `?next=<user value>`. Only
                // local paths are accepted so this cannot become an open redirect.
                if !crate::handler::is_local_redirect_path(&url) {
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
            ActionResponse::RedirectAllowlisted(url, allowed_origins) => {
                if !crate::handler::is_allowlisted_redirect(&url, &allowed_origins) {
                    return (
                        StatusCode::BAD_REQUEST,
                        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                        "Redirect target is not in the allowed origin list",
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

    #[test]
    fn test_csrf_from_session_with_secret_uses_configured_secret() {
        let s1 = crate::session::Session::new();
        let s2 = crate::session::Session::new();
        let secret = b"explicit-configured-secret-32-bytes!!";

        let t1 = CsrfToken::from_session_with_secret(&s1, secret);
        // The token stored in the session is exactly the returned token.
        assert_eq!(t1.as_str(), s1.get("_csrf_token").unwrap().as_str());
        assert!(!t1.as_str().is_empty());
        assert!(t1.verify_token(t1.as_str()));

        // A different secret yields a different token (the configured secret
        // genuinely contributes to the HMAC, unlike a hard-coded fallback).
        let t2 = CsrfToken::from_session_with_secret(&s2, b"a-totally-different-secret-000000");
        assert_ne!(t1.as_str(), t2.as_str());

        // Reuse: a second call returns the already-bound token unchanged.
        let again = CsrfToken::from_session_with_secret(&s1, secret);
        assert_eq!(t1.as_str(), again.as_str());
    }

    #[test]
    fn test_configured_csrf_secret_is_explicit_and_never_panics() {
        // Robust to any CI environment (production env may or may not be set):
        // the resolver either yields a non-empty secret or an explicit error.
        match configured_csrf_secret() {
            Ok(secret) => assert!(!secret.is_empty()),
            Err(err) => assert!(err.contains("production")),
        }
    }

    #[test]
    fn test_csrf_from_session_is_consistent_with_stored_token() {
        let session = crate::session::Session::new();
        let token = CsrfToken::from_session(&session);
        // Never panics, and never binds a token derived from a silent weak
        // fallback. If a real token was bound it must equal the returned one;
        // otherwise the call failed closed (production, no configured secret)
        // and stored nothing for the middleware to accept.
        assert!(!token.as_str().is_empty());
        if let Some(stored) = session.get("_csrf_token") {
            assert_eq!(token.as_str(), stored.as_str());
        }
    }
}
