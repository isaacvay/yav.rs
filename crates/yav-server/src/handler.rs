//! Request handler utilities, HTML response helpers, and the [`HtmlPage`] document wrapper.
//!
//! Provides ergonomic helper functions for returning HTML, JSON, 302 redirects,
//! and 404 responses, along with the [`HtmlPage`] renderer.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use yav_core::{document_shell_with_lang, render_to_string, Element, Node, PageMeta};

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

/// Construct a 302 Found redirect response to the given target URL.
///
/// Sets the `Location` header to the provided URL.
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
    match HeaderValue::try_from(url) {
        Ok(loc) => (StatusCode::FOUND, [(header::LOCATION, loc)]).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Invalid redirect URL",
        )
            .into_response(),
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
        Self {
            title: String::from("Yav App"),
            head_extra: String::from(r#"<link rel="stylesheet" href="/public/styles.css" />"#),
            lang: String::from("en"),
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
        self
    }

    /// Set custom markup for the `<head>` section.
    pub fn head_extra(mut self, head_extra: impl Into<String>) -> Self {
        self.head_extra = head_extra.into();
        self
    }

    /// Append a stylesheet `<link>` tag to `<head>`.
    pub fn stylesheet(mut self, href: impl AsRef<str>) -> Self {
        let escaped = yav_core::escape_attribute(href.as_ref());
        self.head_extra
            .push_str(&format!(r#"<link rel="stylesheet" href="{}" />"#, escaped));
        self
    }

    /// Append a favicon `<link>` tag to `<head>`.
    pub fn favicon(mut self, href: impl AsRef<str>) -> Self {
        let escaped = yav_core::escape_attribute(href.as_ref());
        let mime = if href.as_ref().ends_with(".svg") {
            r#" type="image/svg+xml""#
        } else {
            ""
        };
        self.head_extra
            .push_str(&format!(r#"<link rel="icon"{mime} href="{escaped}" />"#));
        self
    }

    /// Append a `<script>` tag before `</body>`.
    pub fn script(mut self, src: impl AsRef<str>) -> Self {
        let escaped = yav_core::escape_attribute(src.as_ref());
        self.body_end
            .push_str(&format!(r#"<script src="{}"></script>"#, escaped));
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
            let escaped = yav_core::escape_attribute(css);
            head.push_str(&format!(r#"<link rel="stylesheet" href="{}" />"#, escaped));
        }
        page.head_extra = head;

        let mut end = String::new();
        for script in &meta.scripts {
            let escaped = yav_core::escape_attribute(script);
            end.push_str(&format!(r#"<script src="{}"></script>"#, escaped));
        }
        page.body_end = end;

        page
    }

    /// Render the entire document to an HTML string.
    pub fn render(&self) -> String {
        let rendered_body = render_to_string(&self.body);
        let mut body_end = self.body_end.clone();
        if std::env::var("YAV_DEV").is_ok() {
            let reload_port =
                std::env::var("YAV_RELOAD_PORT").unwrap_or_else(|_| "35729".to_string());
            body_end.push_str(&format!(
                r#"<script>(function(){{var s=document.createElement('script');s.src=window.location.protocol+'//'+window.location.hostname+':{}/reload.js';document.head.appendChild(s);}})();</script>"#,
                reload_port
            ));
        }

        let trimmed = rendered_body.trim_start();
        if trimmed.starts_with("<!DOCTYPE")
            || trimmed.starts_with("<!doctype")
            || trimmed.starts_with("<html")
        {
            // Already a complete HTML document: inject body_end before </body> without double-wrapping
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
            document_shell_with_lang(
                &self.title,
                &self.head_extra,
                &rendered_body,
                &body_end,
                &self.lang,
            )
        }
    }
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
            if let Ok(val) = HeaderValue::from_str(&redirect) {
                headers.insert("HX-Redirect", val);
            }
        }
        if let Some(push_url) = self.hx_push_url {
            if let Ok(val) = HeaderValue::from_str(&push_url) {
                headers.insert("HX-Push-Url", val);
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
}
