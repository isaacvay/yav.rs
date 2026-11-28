//! Request handler utilities, HTML response helpers, and the [`HtmlPage`] document wrapper.
//!
//! Provides ergonomic helper functions for returning HTML, JSON, 302 redirects,
//! and 404 responses, along with the [`HtmlPage`] renderer.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::sync::OnceLock;
use yav_core::{render_to_string, Element, Node, PageMeta};

static ASSET_MANIFEST: OnceLock<Option<yav_build::assets::AssetManifest>> = OnceLock::new();

/// Resolves an asset path through `dist/asset-manifest.json` (or `public/asset-manifest.json`)
/// when running outside of `YAV_DEV` mode.
pub fn resolve_asset_path(href: &str) -> String {
    if std::env::var("YAV_DEV").is_ok() {
        return href.to_string();
    }
    let manifest_opt = ASSET_MANIFEST.get_or_init(|| {
        yav_build::assets::AssetManifest::load("dist/asset-manifest.json")
            .or_else(|_| yav_build::assets::AssetManifest::load("public/asset-manifest.json"))
            .ok()
    });
    if let Some(ref manifest) = manifest_opt {
        let resolved = manifest.resolve_url(href);
        // Verify target file exists if path is local, avoiding broken 404 links from stale manifests
        let clean = resolved.trim_start_matches('/');
        let stripped_public = clean.strip_prefix("public/").unwrap_or(clean);
        if std::path::Path::new(clean).exists()
            || std::path::Path::new("public")
                .join(stripped_public)
                .exists()
            || std::path::Path::new("dist").join(stripped_public).exists()
            || std::path::Path::new("dist/public")
                .join(stripped_public)
                .exists()
        {
            resolved
        } else {
            href.to_string()
        }
    } else {
        href.to_string()
    }
}

/// Construct an HTML response with the given status code and HTML body string.
///
/// Sets the `Content-Type` header to `text/html; charset=utf-8`.
///
/// # Examples
/// ```rust,no_run
/// use axum::http::StatusCode;
/// use yav_server::html_response;
///
/// let resp = html_response(StatusCode::OK, "<h1>Hello, Yav!</h1>");
/// assert_eq!(resp.status(), StatusCode::OK);
/// ```
pub fn html_response(status: StatusCode, body: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body.into(),
    )
        .into_response()
}

/// Construct a JSON response from any serializable value.
///
/// Sets the `Content-Type` header to `application/json`.
/// If serialization fails, returns a 500 Internal Server Error response.
///
/// # Examples
/// ```rust,no_run
/// use axum::http::StatusCode;
/// use serde::Serialize;
/// use yav_server::json_response;
///
/// #[derive(Serialize)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".into() };
/// let resp = json_response(StatusCode::OK, &user);
/// assert_eq!(resp.status(), StatusCode::OK);
/// ```
pub fn json_response<T: Serialize>(status: StatusCode, data: &T) -> Response {
    match serde_json::to_string(data) {
        Ok(json) => (status, [(header::CONTENT_TYPE, "application/json")], json).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            format!(
                r#"{{"error":"serialization_failed","message":{}}}"#,
                serde_json::Value::String(err.to_string())
            ),
        )
            .into_response(),
    }
}

/// Checks if a redirect target URL is free of dangerous schemes and header-injection
/// payloads (`javascript:`, `data:`, `vbscript:`, `//evil.com`, `/\evil.com`, CRLF).
///
/// Note: this deliberately does **not** decide whether a target is *trustworthy*.
/// An absolute `https://evil.com` passes this check; use
/// [`is_local_redirect_path`] or [`is_allowlisted_redirect`] to prevent open
/// redirects when the URL can be influenced by a client.
pub fn is_safe_redirect_target(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_control()) {
        return false;
    }
    // Browsers normalise backslashes into slashes in URLs (`/\evil.com`,
    // `https:\evil.com`, `/%5c evil.com`). Reject every form of ambiguity.
    if trimmed.contains('\\') || trimmed.to_ascii_lowercase().contains("%5c") {
        return false;
    }
    if trimmed.starts_with("//") {
        return false;
    }
    let normalized: String = trimmed
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if normalized.contains("%0d") || normalized.contains("%0a") || normalized.contains("%00") {
        return false;
    }
    // An absolute URL may only use http/https, and must have a proper authority.
    if let Some(colon) = normalized.find(':') {
        let first_slash = normalized.find('/').unwrap_or(normalized.len());
        if colon < first_slash {
            let scheme = &normalized[..colon];
            if (scheme != "http" && scheme != "https") || !normalized[colon + 1..].starts_with("//")
            {
                return false;
            }
            // Reject userinfo trickery (`https://trusted.com@evil.com/`).
            let authority = normalized[colon + 3..]
                .split('/')
                .next()
                .unwrap_or_default();
            if authority.contains('@') {
                return false;
            }
        }
    }
    true
}

/// Validate an absolute redirect target against an explicit origin allowlist.
///
/// `allowed_origins` entries look like `https://example.com` (an optional port is
/// allowed). Comparison is case-insensitive and ignores a trailing slash. An
/// empty allowlist rejects every external target.
pub fn is_allowlisted_redirect(url: &str, allowed_origins: &[String]) -> bool {
    if !is_safe_redirect_target(url) {
        return false;
    }
    let normalized = url.trim().to_ascii_lowercase();
    let Some((scheme, rest)) = normalized.split_once("://") else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() {
        return false;
    }
    allowed_origins.iter().any(|allowed| {
        let clean = allowed.trim().trim_end_matches('/').to_ascii_lowercase();
        let without_scheme = clean
            .split_once("://")
            .map(|(_, r)| r)
            .unwrap_or(clean.as_str());
        without_scheme == authority
    })
}

/// Checks if a redirect target is strictly a local relative path (starts with `/` and not `//` or `/\`).
pub fn is_local_redirect_path(path: &str) -> bool {
    let trimmed = path.trim();
    if !is_safe_redirect_target(trimmed) || !trimmed.starts_with('/') {
        return false;
    }
    if trimmed.len() > 1 {
        let second = trimmed.as_bytes()[1];
        if second == b'/' || second == b'\\' || second == b' ' {
            return false;
        }
    }
    true
}

/// Construct a 302 Found redirect response to a **local** application path.
///
/// Only same-origin paths (`/...`) are accepted. Absolute URLs, protocol-relative
/// URLs, dangerous schemes and header-injection payloads are rejected with
/// `400 Bad Request`, which prevents user-controlled values from turning this
/// into an open redirect. Use [`redirect_allowlisted`] when an external redirect
/// is genuinely required.
///
/// # Examples
/// ```rust,no_run
/// use axum::http::StatusCode;
/// use yav_server::redirect;
///
/// let resp = redirect("/dashboard");
/// assert_eq!(resp.status(), StatusCode::FOUND);
/// ```
pub fn redirect(url: &str) -> Response {
    if !is_local_redirect_path(url) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Invalid or unsafe redirect URL",
        )
            .into_response();
    }
    match HeaderValue::try_from(url.trim()) {
        Ok(loc) => (StatusCode::FOUND, [(header::LOCATION, loc)]).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Invalid redirect URL",
        )
            .into_response(),
    }
}

/// Construct a 302 Found redirect to an external URL, but only when its origin
/// appears in `allowed_origins` (e.g. an OAuth provider / payment gateway).
///
/// This keeps external redirects possible without reintroducing an open redirect.
pub fn redirect_allowlisted(url: &str, allowed_origins: &[String]) -> Response {
    if !is_allowlisted_redirect(url, allowed_origins) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Redirect target is not in the allowed origin list",
        )
            .into_response();
    }
    match HeaderValue::try_from(url.trim()) {
        Ok(loc) => (StatusCode::FOUND, [(header::LOCATION, loc)]).into_response(),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Invalid redirect URL",
        )
            .into_response(),
    }
}

/// Construct a 302 Found redirect strictly restricted to local application paths (`/...`).
///
/// If the target is an external domain or invalid, safely redirects to `"/"`.
pub fn safe_redirect(path: &str) -> Response {
    if is_local_redirect_path(path) {
        redirect(path)
    } else {
        redirect("/")
    }
}

/// Construct a standard 404 Not Found response using the framework's error page renderer.
pub fn not_found() -> Response {
    crate::error::YavError::NotFound.into_response()
}

/// Helper to construct an XML response from a [`yav_core::Sitemap`].
pub fn sitemap_response(sitemap: &yav_core::Sitemap) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        sitemap.to_xml(),
    )
        .into_response()
}

/// Helper to construct a plain text response from a [`yav_core::RobotsTxt`].
pub fn robots_response(robots: &yav_core::RobotsTxt) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        robots.to_txt(),
    )
        .into_response()
}

/// An HTML page wrapper that renders a virtual DOM [`Node`] tree
/// inside a complete HTML5 document shell.
#[derive(Debug, Clone)]
pub struct HtmlPage {
    /// Document title inside `<title>`.
    pub title: String,
    /// Custom markup injected into `<head>` (e.g. stylesheets, meta tags).
    pub head_extra: String,
    /// Language attribute for the root `<html>` tag (default: `"en"`).
    pub lang: String,
    /// Whether `.lang(...)` was explicitly called on this `HtmlPage`.
    pub lang_explicit: bool,
    /// Root body virtual DOM node.
    pub body: Node,
    /// Scripts or extra markup injected right before `</body>`.
    pub body_end: String,
    /// HTTP status code for the response (default: `200 OK`).
    pub status: StatusCode,
}

impl HtmlPage {
    /// Create a new `HtmlPage` wrapping the given body content.
    pub fn new(body: impl Into<Node>) -> Self {
        let default_css = resolve_asset_path("/public/styles.css");
        Self {
            title: String::from("Yav App"),
            head_extra: format!(r#"<link rel="stylesheet" href="{default_css}" />"#),
            lang: String::from("en"),
            lang_explicit: false,
            body: body.into(),
            body_end: String::new(),
            status: StatusCode::OK,
        }
    }

    /// Set the page `<title>`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the HTML document language code (e.g. `"en"`, `"fr"`, `"es"`).
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self.lang_explicit = true;
        self
    }

    /// Set custom markup for the `<head>` section.
    pub fn head_extra(mut self, head_extra: impl Into<String>) -> Self {
        self.head_extra = head_extra.into();
        self
    }

    /// Append a stylesheet `<link>` tag to `<head>`.
    pub fn stylesheet(mut self, href: impl AsRef<str>) -> Self {
        let resolved = resolve_asset_path(href.as_ref());
        if let Some(escaped) = yav_core::escape_url(&resolved) {
            self.head_extra
                .push_str(&format!(r#"<link rel="stylesheet" href="{escaped}" />"#));
        }
        self
    }

    /// Append a favicon `<link>` tag to `<head>`.
    pub fn favicon(mut self, href: impl AsRef<str>) -> Self {
        let resolved = resolve_asset_path(href.as_ref());
        if let Some(escaped) = yav_core::escape_url(&resolved) {
            let mime = if resolved.ends_with(".svg") {
                r#" type="image/svg+xml""#
            } else {
                ""
            };
            self.head_extra
                .push_str(&format!(r#"<link rel="icon"{mime} href="{escaped}" />"#));
        }
        self
    }

    /// Append a `<script>` tag before `</body>`.
    pub fn script(mut self, src: impl AsRef<str>) -> Self {
        let resolved = resolve_asset_path(src.as_ref());
        if let Some(escaped) = yav_core::escape_url(&resolved) {
            self.body_end
                .push_str(&format!(r#"<script src="{escaped}"></script>"#));
        }
        self
    }

    /// Set content injected before `</body>`.
    pub fn body_end(mut self, body_end: impl Into<String>) -> Self {
        self.body_end = body_end.into();
        self
    }

    /// Set the HTTP status code.
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Construct an `HtmlPage` from [`PageMeta`] and a body node.
    pub fn from_meta(meta: &PageMeta, body: impl Into<Node>) -> Self {
        let mut page = Self::new(body);
        if let Some(ref title) = meta.title {
            page.title = title.clone();
        }
        if let Some(code) = meta.status {
            if let Ok(st) = StatusCode::from_u16(code) {
                page.status = st;
            }
        }

        let mut head = meta.render_head_tags();
        for css in &meta.css {
            let resolved = resolve_asset_path(css);
            if let Some(escaped) = yav_core::escape_url(&resolved) {
                head.push_str(&format!(r#"<link rel="stylesheet" href="{escaped}" />"#));
            }
        }
        page.head_extra = head;

        let mut end = String::new();
        for script in &meta.scripts {
            let resolved = resolve_asset_path(script);
            if let Some(escaped) = yav_core::escape_url(&resolved) {
                end.push_str(&format!(r#"<script src="{escaped}"></script>"#));
            }
        }
        page.body_end = end;

        page
    }

    /// Render the entire document to an HTML string.
    pub fn render(&self) -> String {
        let transformed_body: Option<Node> = yav_core::use_context::<yav_core::SsrLayoutChain>()
            .map(|chain| (chain.0)(self.body.clone()));
        let body_ref: &Node = transformed_body.as_ref().unwrap_or(&self.body);

        let effective_lang = if self.lang_explicit {
            self.lang.clone()
        } else {
            yav_core::use_context::<String>().unwrap_or_else(|| self.lang.clone())
        };

        let mut body_end = self.body_end.clone();
        if std::env::var("YAV_DEV").is_ok() {
            let reload_port: u16 = std::env::var("YAV_RELOAD_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(35729);
            body_end.push_str(&format!(
                r#"<script>(function(){{var s=document.createElement('script');s.src=window.location.protocol+'//'+window.location.hostname+':{reload_port}/reload.js';document.head.appendChild(s);}})();</script>"#,
            ));
        }

        // Check if body_ref is already a root <html> element or full raw HTML document
        let is_html_root_element = match body_ref {
            Node::Element(el) => el.tag.eq_ignore_ascii_case("html"),
            _ => false,
        };

        let raw_doc: Option<&str> = match body_ref {
            Node::Raw(s) => Some(s.as_str()),
            Node::Static(s) => Some(s),
            _ => None,
        };
        let html = if is_html_root_element {
            let mut rendered_body = String::new();
            rendered_body.push_str("<!DOCTYPE html>\n");
            rendered_body.push_str(&yav_core::render_to_string(body_ref));
            if body_end.is_empty() {
                rendered_body
            } else if let Some(pos) = rendered_body.rfind("</body>") {
                let mut full = rendered_body;
                full.insert_str(pos, &body_end);
                full
            } else {
                let mut full = rendered_body;
                full.push_str(&body_end);
                full
            }
        } else if let Some(raw_doc) = raw_doc {
            let trimmed = raw_doc.trim_start();
            if trimmed.starts_with("<!DOCTYPE")
                || trimmed.starts_with("<!doctype")
                || trimmed.starts_with("<html")
            {
                let rendered_body = raw_doc.to_string();
                if body_end.is_empty() {
                    rendered_body
                } else if let Some(pos) = rendered_body.rfind("</body>") {
                    let mut full = rendered_body;
                    full.insert_str(pos, &body_end);
                    full
                } else {
                    let mut full = rendered_body;
                    full.push_str(&body_end);
                    full
                }
            } else {
                yav_core::render_node_document_with_lang(
                    &self.title,
                    &self.head_extra,
                    body_ref,
                    &body_end,
                    &effective_lang,
                )
            }
        } else {
            yav_core::render_node_document_with_lang(
                &self.title,
                &self.head_extra,
                body_ref,
                &body_end,
                &effective_lang,
            )
        };

        if let Some(csp) = yav_core::use_context::<crate::middleware::CspNonce>() {
            if !csp.0.is_empty() {
                return inject_csp_nonce_into_scripts(&html, &csp.0);
            }
        }
        html
    }
}

fn find_closing_gt_respecting_quotes(s: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            return Some(i);
        }
    }
    None
}

fn has_nonce_attribute(attrs: &str) -> bool {
    let bytes = attrs.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let name = &attrs[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let has_eq = i < bytes.len() && bytes[i] == b'=';
        if name.eq_ignore_ascii_case("nonce") && has_eq {
            return true;
        }
        if has_eq {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
        }
    }
    false
}

/// Injects `nonce="..."` into `<script` opening tags that do not already carry a `nonce=` attribute.
///
/// The scan is *HTML/script-aware*: skips HTML comments (`<!-- ... -->`), rawtext
/// non-script elements (`<textarea>`, `<title>`, `<style>`), parses `<script ...>`
/// attributes while respecting quoted `>`, checks exact attribute name boundaries
/// (avoiding false positives on `data-nonce=`), and copies `<script>` bodies verbatim.
pub fn inject_csp_nonce_into_scripts(html: &str, nonce: &str) -> String {
    let escaped_nonce = yav_core::escape_html(nonce);
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len() + 64);
    let mut cursor = 0usize;

    while let Some(rel) = lower[cursor..].find('<') {
        let idx = cursor + rel;
        out.push_str(&html[cursor..idx]);

        let rest_lower = &lower[idx..];
        if rest_lower.starts_with("<!--") {
            let end = rest_lower
                .find("-->")
                .map(|p| idx + p + 3)
                .unwrap_or(html.len());
            out.push_str(&html[idx..end]);
            cursor = end;
            continue;
        }

        if let Some(after_tag) = rest_lower.strip_prefix("<script") {
            if !matches!(
                after_tag.chars().next(),
                Some(' ') | Some('>') | Some('\t') | Some('\n') | Some('\r') | Some('/')
            ) {
                out.push_str(&html[idx..idx + 7]);
                cursor = idx + 7;
                continue;
            }

            let after = &html[idx + 7..];
            let (tag_end, has_gt) = match find_closing_gt_respecting_quotes(after) {
                Some(rel_end) => (idx + 7 + rel_end, true),
                None => (html.len(), false),
            };
            let attrs = &html[idx + 7..tag_end];

            out.push_str(&html[idx..idx + 7]);
            if !has_nonce_attribute(attrs) {
                out.push_str(&format!(r#" nonce="{escaped_nonce}""#));
            }
            out.push_str(attrs);
            if has_gt {
                out.push('>');
            }

            let content_start = if has_gt { tag_end + 1 } else { tag_end };
            if content_start >= html.len() {
                cursor = html.len();
                break;
            }

            if let Some(close_rel) = lower[content_start..].find("</script") {
                let close_idx = content_start + close_rel;
                let close_end = lower[close_idx..]
                    .find('>')
                    .map(|r| close_idx + r + 1)
                    .unwrap_or(html.len());
                out.push_str(&html[content_start..close_end]);
                cursor = close_end;
            } else {
                out.push_str(&html[content_start..]);
                cursor = html.len();
            }
            continue;
        }

        // Skip rawtext non-script containers verbatim so `<script>` inside `<textarea>`/`<title>`/`<style>` stays untouched.
        let raw_tag = ["<textarea", "<title", "<style"].into_iter().find(|&tag| {
            rest_lower.strip_prefix(tag).is_some_and(|after| {
                matches!(
                    after.chars().next(),
                    Some(' ') | Some('>') | Some('\t') | Some('\n') | Some('\r') | Some('/')
                )
            })
        });
        if let Some(tag) = raw_tag {
            let close_pat = format!("</{}", &tag[1..]);
            if let Some(open_gt) = find_closing_gt_respecting_quotes(&html[idx + tag.len()..]) {
                let body_start = idx + tag.len() + open_gt + 1;
                if let Some(close_rel) = lower[body_start..].find(&close_pat) {
                    let close_idx = body_start + close_rel;
                    let close_end = lower[close_idx..]
                        .find('>')
                        .map(|r| close_idx + r + 1)
                        .unwrap_or(html.len());
                    out.push_str(&html[idx..close_end]);
                    cursor = close_end;
                    continue;
                }
            }
        }

        out.push('<');
        cursor = idx + 1;
    }

    out.push_str(&html[cursor..]);
    out
}

impl IntoResponse for HtmlPage {
    fn into_response(self) -> Response {
        let status = self.status;
        let html = self.render();
        html_response(status, html)
    }
}

impl From<Node> for HtmlPage {
    fn from(node: Node) -> Self {
        Self::new(node)
    }
}

impl From<Element> for HtmlPage {
    fn from(element: Element) -> Self {
        Self::new(Node::Element(element))
    }
}

/// A partial HTML response for HTMX swaps, AJAX requests, or micro-frontends.
///
/// Unlike [`HtmlPage`], `HtmlFragment` renders ONLY the specified [`Node`] tree
/// without `<!DOCTYPE html>`, `<html>`, or `<head>` tags.
///
/// Also provides builders for HTMX response headers (`HX-Trigger`, `HX-Redirect`, `HX-Push-Url`, `HX-Reswap`, `HX-Retarget`).
#[derive(Debug, Clone)]
pub struct HtmlFragment {
    /// Virtual DOM content to render.
    pub content: Node,
    /// HTTP status code (default: 200 OK).
    pub status: StatusCode,
    /// Event name to trigger on client via `HX-Trigger`.
    pub hx_trigger: Option<String>,
    /// Client-side redirect URL via `HX-Redirect`.
    pub hx_redirect: Option<String>,
    /// History push URL via `HX-Push-Url`.
    pub hx_push_url: Option<String>,
    /// Client-side swap style via `HX-Reswap`.
    pub hx_reswap: Option<String>,
    /// Client-side target element via `HX-Retarget`.
    pub hx_retarget: Option<String>,
}

impl HtmlFragment {
    /// Create a new partial HTML fragment from any type convertible into [`Node`].
    pub fn new(content: impl Into<Node>) -> Self {
        Self {
            content: content.into(),
            status: StatusCode::OK,
            hx_trigger: None,
            hx_redirect: None,
            hx_push_url: None,
            hx_reswap: None,
            hx_retarget: None,
        }
    }

    /// Set HTTP status code (default 200 OK).
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Trigger client-side events via `HX-Trigger`.
    pub fn hx_trigger(mut self, event: impl Into<String>) -> Self {
        self.hx_trigger = Some(event.into());
        self
    }

    /// Trigger a client-side redirect via `HX-Redirect`.
    pub fn hx_redirect(mut self, url: impl Into<String>) -> Self {
        self.hx_redirect = Some(url.into());
        self
    }

    /// Push a new URL into browser history via `HX-Push-Url`.
    pub fn hx_push_url(mut self, url: impl Into<String>) -> Self {
        self.hx_push_url = Some(url.into());
        self
    }

    /// Override the swap strategy via `HX-Reswap`.
    pub fn hx_reswap(mut self, swap: impl Into<String>) -> Self {
        self.hx_reswap = Some(swap.into());
        self
    }

    /// Override the swap target via `HX-Retarget`.
    pub fn hx_retarget(mut self, target: impl Into<String>) -> Self {
        self.hx_retarget = Some(target.into());
        self
    }
}

impl IntoResponse for HtmlFragment {
    fn into_response(self) -> Response {
        let html = render_to_string(&self.content);
        let mut resp = (
            self.status,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response();

        let headers = resp.headers_mut();
        if let Some(trigger) = self.hx_trigger {
            if let Ok(val) = HeaderValue::from_str(&trigger) {
                headers.insert("HX-Trigger", val);
            }
        }
        if let Some(redirect) = self.hx_redirect {
            // HTMX navigation must stay on this origin; otherwise a reflected
            // value could ship visitors to an attacker-controlled site.
            if is_local_redirect_path(&redirect) {
                if let Ok(val) = HeaderValue::from_str(redirect.trim()) {
                    headers.insert("HX-Redirect", val);
                }
            }
        }
        if let Some(push_url) = self.hx_push_url {
            // `false` is the documented HTMX value for "do not push a URL".
            let allowed =
                push_url.trim().eq_ignore_ascii_case("false") || is_local_redirect_path(&push_url);
            if allowed {
                if let Ok(val) = HeaderValue::from_str(push_url.trim()) {
                    headers.insert("HX-Push-Url", val);
                }
            }
        }
        if let Some(reswap) = self.hx_reswap {
            if let Ok(val) = HeaderValue::from_str(&reswap) {
                headers.insert("HX-Reswap", val);
            }
        }
        if let Some(retarget) = self.hx_retarget {
            if let Ok(val) = HeaderValue::from_str(&retarget) {
                headers.insert("HX-Retarget", val);
            }
        }

        resp
    }
}

impl From<Node> for HtmlFragment {
    fn from(node: Node) -> Self {
        Self::new(node)
    }
}

impl From<Element> for HtmlFragment {
    fn from(element: Element) -> Self {
        Self::new(Node::Element(element))
    }
}

/// Axum request extractor for HTMX header metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HtmxRequest {
    /// True if the request was initiated by HTMX (`HX-Request: "true"`).
    pub is_htmx: bool,
    /// The ID or selector of the target element (`HX-Target`).
    pub target: Option<String>,
    /// The current URL of the browser (`HX-Current-URL`).
    pub current_url: Option<String>,
    /// The user response to an `hx-prompt`.
    pub prompt: Option<String>,
}

impl<S> axum::extract::FromRequestParts<S> for HtmxRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let is_htmx = parts
            .headers
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            == Some("true");

        let target = parts
            .headers
            .get("HX-Target")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let current_url = parts
            .headers
            .get("HX-Current-URL")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let prompt = parts
            .headers
            .get("HX-Prompt")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok(HtmxRequest {
            is_htmx,
            target,
            current_url,
            prompt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::ActionResponse;

    #[derive(Serialize)]
    struct TestData {
        name: String,
        count: u32,
    }

    #[tokio::test]
    async fn html_response_has_correct_content_type() {
        let resp = html_response(StatusCode::OK, "<h1>Hello</h1>");

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(ct, "text/html; charset=utf-8");

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert_eq!(body_bytes.as_ref(), b"<h1>Hello</h1>");
    }

    #[tokio::test]
    async fn json_response_serializes_properly() {
        let data = TestData {
            name: "Alice".to_string(),
            count: 42,
        };

        let resp = json_response(StatusCode::CREATED, &data);

        assert_eq!(resp.status(), StatusCode::CREATED);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(ct, "application/json");

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body_str = String::from_utf8(body_bytes.to_vec()).expect("utf8");

        assert!(body_str.contains(r#""name":"Alice""#));
        assert!(body_str.contains(r#""count":42"#));
    }

    #[test]
    fn redirect_has_correct_status_and_location() {
        let resp = redirect("/auth/login");

        assert_eq!(resp.status(), StatusCode::FOUND);
        let loc = resp
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(loc, "/auth/login");
    }

    #[tokio::test]
    async fn not_found_helper_returns_404() {
        let resp = not_found();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(ct, "text/html; charset=utf-8");
    }

    #[tokio::test]
    async fn html_page_renders_document_shell() {
        let node = Node::Element(Element::new("main").child(Node::text("Welcome to Yav")));
        let page = HtmlPage::new(node)
            .title("Home Page")
            .stylesheet("/assets/styles.css")
            .script("/assets/client.js");

        let html = page.render();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Home Page</title>"));
        assert!(html.contains(r#"<link rel="stylesheet" href="/assets/styles.css" />"#));
        assert!(html.contains(r#"<script src="/assets/client.js"></script>"#));
        assert!(html.contains("<main>Welcome to Yav</main>"));

        let resp = page.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn html_page_from_meta() {
        let meta = PageMeta {
            title: Some("Dashboard".to_string()),
            description: Some("Overview of activity".to_string()),
            css: vec!["/dash.css".to_string()],
            scripts: vec!["/dash.js".to_string()],
            status: Some(200),
            ..Default::default()
        };

        let page = HtmlPage::from_meta(&meta, Node::text("Dashboard Content"));
        let html = page.render();

        assert!(html.contains("<title>Dashboard</title>"));
        assert!(html.contains(r#"<meta name="description" content="Overview of activity" />"#));
        assert!(html.contains(r#"<link rel="stylesheet" href="/dash.css" />"#));
        assert!(html.contains(r#"<script src="/dash.js"></script>"#));
        assert!(html.contains("Dashboard Content"));
    }

    #[test]
    fn html_page_avoids_double_wrapping_complete_document() {
        let complete_doc = "<!DOCTYPE html><html lang=\"fr\"><head><title>Root</title></head><body><div id=\"app\">Content</div></body></html>";
        let page = HtmlPage::new(Node::raw(complete_doc)).script("/extra.js");
        let html = page.render();

        // Must NOT contain duplicate doctypes or nested html tags
        assert_eq!(html.matches("<!DOCTYPE html>").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("</html>").count(), 1);
        assert!(html.contains(r#"<script src="/extra.js"></script></body>"#));
    }

    #[test]
    fn test_redirect_blocks_dangerous_schemes_and_protocol_relative() {
        let js_resp = redirect("javascript:alert(1)");
        assert_eq!(js_resp.status(), StatusCode::BAD_REQUEST);

        let proto_resp = redirect("//evil.com/phish");
        assert_eq!(proto_resp.status(), StatusCode::BAD_REQUEST);

        let safe_external = safe_redirect("https://evil.com/phish");
        assert_eq!(safe_external.status(), StatusCode::FOUND);
        assert_eq!(safe_external.headers().get(header::LOCATION).unwrap(), "/");

        let safe_local = safe_redirect("/dashboard/settings");
        assert_eq!(safe_local.status(), StatusCode::FOUND);
        assert_eq!(
            safe_local.headers().get(header::LOCATION).unwrap(),
            "/dashboard/settings"
        );
    }

    #[test]
    fn test_redirect_is_not_an_open_redirect() {
        for target in [
            "https://evil.com/phish",
            "http://evil.com",
            "\\\\evil.com",
            "/\\evil.com",
            "https:/\\evil.com",
            "https://trusted.example@evil.com/",
            "/%5cevil.com",
            "mailto:attacker@example.com",
        ] {
            let resp = redirect(target);
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "external/ambiguous target must be rejected: {target}"
            );
        }
        assert_eq!(redirect("/dashboard?next=/ok").status(), StatusCode::FOUND);
    }

    #[test]
    fn test_redirect_allowlisted_requires_explicit_origin() {
        let allowed = vec!["https://accounts.example.com".to_string()];
        let ok = redirect_allowlisted("https://accounts.example.com/oauth/cb", &allowed);
        assert_eq!(ok.status(), StatusCode::FOUND);

        let blocked = redirect_allowlisted("https://evil.com/oauth/cb", &allowed);
        assert_eq!(blocked.status(), StatusCode::BAD_REQUEST);

        let spoofed =
            redirect_allowlisted("https://accounts.example.com@evil.com/oauth/cb", &allowed);
        assert_eq!(spoofed.status(), StatusCode::BAD_REQUEST);

        let empty = redirect_allowlisted("https://accounts.example.com/", &[]);
        assert_eq!(empty.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_form_action_redirect_blocks_external_targets() {
        let resp = ActionResponse::redirect("https://evil.com/phish").into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let allowed = ["https://pay.example.com"];
        let ok =
            ActionResponse::redirect_allowlisted("https://pay.example.com/checkout/done", allowed)
                .into_response();
        assert_eq!(ok.status(), StatusCode::SEE_OTHER);
    }

    #[test]
    fn test_html_page_script_and_stylesheet_block_javascript_urls() {
        let page = HtmlPage::new(Node::text("Body"))
            .stylesheet("javascript:alert(1)")
            .script("javascript:alert(2)")
            .favicon("data:text/html,<script>alert(3)</script>");
        let html = page.render();
        assert!(!html.contains("javascript:alert"));
        assert!(!html.contains("data:text/html"));
    }

    #[test]
    fn test_html_page_injects_csp_nonce_from_ssr_context() {
        let html = yav_core::context::provide_context(
            crate::middleware::CspNonce("nonce-xyz-999".to_string()),
            || {
                HtmlPage::new(Node::text("Secure Body"))
                    .script("/app.js")
                    .render()
            },
        );
        assert!(html.contains(r#"<script nonce="nonce-xyz-999" src="/app.js"></script>"#));
        assert!(html.contains(r#"<script nonce="nonce-xyz-999" id="yav-micro-actions">"#));
    }

    #[test]
    fn test_nonce_injection_does_not_corrupt_script_bodies() {
        let html = r#"<script>var x = "<script>alert(1)</script>";</script><script src="/a.js"></script><script NONCE="keep">ok()</script>"#;
        let out = inject_csp_nonce_into_scripts(html, "n0nce");
        // The literal inside the JS string must remain byte-for-byte identical.
        assert!(out.contains(r#"var x = "<script>alert(1)</script>""#));
        assert_eq!(out.matches(r#"nonce="n0nce""#).count(), 2);
        assert!(out.contains(r#"<script NONCE="keep">"#));
    }

    #[test]
    fn test_nonce_injection_handles_uppercase_and_lookalike_tags() {
        let html = r#"<SCRIPT>foo()</SCRIPT><scripture>not a tag</scripture>"#;
        let out = inject_csp_nonce_into_scripts(html, "N");
        assert!(out.contains(r#"<SCRIPT nonce="N">foo()</SCRIPT>"#));
        assert!(out.contains("<scripture>not a tag</scripture>"));
    }
}
