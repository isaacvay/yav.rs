//! Full-Stack E2E `TestClient` & `DomAssert` Component Testing Harness.
//!
//! Provides Playwright / `@testing-library` ergonomics in pure Rust without launching
//! a headless browser:
//! - [`TestClient`] dispatches synthetic HTTP requests directly through [`YavApp`] / [`axum::Router`]
//!   while automatically persisting session cookies (`yav_session`), flash cookies (`yav_flash`),
//!   and CSRF tokens across multi-step user flows.
//! - [`DomAssert`] parses rendered HTML or [`yav_core::Node`] trees and enables expressive
//!   assertions by `#id`, `.class`, `[attr="val"]`, or tag name.

use crate::app::YavApp;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use yav_core::{render_to_string, Node};

/// Stateful E2E test client that preserves cookies and headers across requests.
#[derive(Clone)]
pub struct TestClient {
    router: Router,
    cookies: Arc<Mutex<HashMap<String, String>>>,
}

impl TestClient {
    /// Create a new `TestClient` from a configured [`YavApp`].
    #[must_use]
    pub fn from_app(app: YavApp) -> Self {
        Self {
            router: app.into_router(),
            cookies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new `TestClient` from an [`axum::Router`].
    #[must_use]
    pub fn from_router(router: Router) -> Self {
        Self {
            router,
            cookies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Manually set a cookie in the client's cookie jar.
    pub fn set_cookie(&self, name: impl Into<String>, value: impl Into<String>) {
        let mut jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        jar.insert(name.into(), value.into());
    }

    /// Read a cookie value from the client's cookie jar.
    #[must_use]
    pub fn cookie(&self, name: &str) -> Option<String> {
        let jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        jar.get(name).cloned()
    }

    fn build_cookie_header(&self) -> Option<String> {
        let jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        if jar.is_empty() {
            return None;
        }
        let pairs: Vec<String> = jar.iter().map(|(k, v)| format!("{k}={v}")).collect();
        Some(pairs.join("; "))
    }

    fn store_response_cookies(&self, headers: &axum::http::HeaderMap) {
        let mut jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        for val in headers.get_all(header::SET_COOKIE) {
            if let Ok(raw) = val.to_str() {
                if let Some(first_part) = raw.split(';').next() {
                    if let Some((k, v)) = first_part.split_once('=') {
                        let key = k.trim().to_string();
                        let value = v.trim().to_string();
                        if value.is_empty() || raw.contains("Max-Age=0") {
                            jar.remove(&key);
                        } else {
                            jar.insert(key, value);
                        }
                    }
                }
            }
        }
    }

    /// Execute an HTTP `GET` request.
    pub async fn get(&self, uri: &str) -> TestResponse {
        let mut req_builder = Request::builder().method(Method::GET).uri(uri);
        if let Some(cookie_hdr) = self.build_cookie_header() {
            req_builder = req_builder.header(header::COOKIE, cookie_hdr);
        }
        let req = req_builder.body(Body::empty()).expect("valid GET request");
        self.send(req).await
    }

    /// Execute an HTTP `POST` request with `application/x-www-form-urlencoded` fields.
    pub async fn post_form(&self, uri: &str, fields: &[(&str, &str)]) -> TestResponse {
        let encoded: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect();
        let body_str = encoded.join("&");

        let mut req_builder = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(cookie_hdr) = self.build_cookie_header() {
            req_builder = req_builder.header(header::COOKIE, cookie_hdr);
        }
        let req = req_builder
            .body(Body::from(body_str))
            .expect("valid POST form request");
        self.send(req).await
    }

    /// Execute an HTTP `POST` request with a JSON payload.
    pub async fn post_json<T: serde::Serialize>(&self, uri: &str, payload: &T) -> TestResponse {
        let json_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let mut req_builder = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(cookie_hdr) = self.build_cookie_header() {
            req_builder = req_builder.header(header::COOKIE, cookie_hdr);
        }
        let req = req_builder
            .body(Body::from(json_str))
            .expect("valid POST JSON request");
        self.send(req).await
    }

    /// Dispatch a raw [`axum::http::Request<Body>`] and capture a [`TestResponse`].
    pub async fn send(&self, req: Request<Body>) -> TestResponse {
        let resp = self
            .router
            .clone()
            .oneshot(req)
            .await
            .expect("router oneshot never fails");

        let status = resp.status();
        let headers = resp.headers().clone();
        self.store_response_cookies(&headers);

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap_or_default();
        let body_text = String::from_utf8_lossy(&bytes).into_owned();

        TestResponse {
            status,
            headers,
            body: body_text,
        }
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Captured HTTP response from [`TestClient`].
#[derive(Debug, Clone)]
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: String,
}

impl TestResponse {
    /// Assert that the HTTP status code equals `expected`.
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status, expected,
            "Expected HTTP status {expected}, got {} (body: {})",
            self.status, self.body
        );
        self
    }

    /// Assert that the response is a redirect (`3xx`) to `expected_location`.
    pub fn assert_redirect(&self, expected_location: &str) -> &Self {
        assert!(
            self.status.is_redirection(),
            "Expected 3xx redirect status, got {}",
            self.status
        );
        let loc = self
            .headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            loc, expected_location,
            "Expected redirect Location '{expected_location}', got '{loc}'"
        );
        self
    }

    /// Parse the response body as a [`DomAssert`] inspector for element assertions.
    #[must_use]
    pub fn dom(&self) -> DomAssert {
        DomAssert::from_html(&self.body)
    }
}

/// Lightweight parsed HTML element representation for selector matching.
#[derive(Debug, Clone)]
pub struct DomTag {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub inner_text: String,
    pub raw_outer: String,
}

/// DOM assertion engine supporting `#id`, `.class`, `[attr="val"]`, and tag selectors.
#[derive(Debug, Clone)]
pub struct DomAssert {
    html: String,
    tags: Vec<DomTag>,
}

impl DomAssert {
    /// Create a [`DomAssert`] directly from a [`yav_core::Node`] tree.
    #[must_use]
    pub fn from_node(node: &Node) -> Self {
        Self::from_html(&render_to_string(node))
    }

    /// Create a [`DomAssert`] from a rendered HTML string.
    #[must_use]
    pub fn from_html(html: &str) -> Self {
        let tags = parse_html_tags(html);
        Self {
            html: html.to_string(),
            tags,
        }
    }

    /// Find all elements matching a simple selector (`#id`, `.class`, `[attr="val"]`, `tag`, or `tag.class`).
    #[must_use]
    pub fn query_all(&self, selector: &str) -> Vec<&DomTag> {
        let sel = selector.trim();
        self.tags
            .iter()
            .filter(|el| matches_selector(el, sel))
            .collect()
    }

    /// Find the first element matching `selector`.
    #[must_use]
    pub fn query(&self, selector: &str) -> Option<&DomTag> {
        self.query_all(selector).into_iter().next()
    }

    /// Assert that at least one element matches `selector`.
    pub fn assert_exists(&self, selector: &str) -> &Self {
        assert!(
            self.query(selector).is_some(),
            "Expected element matching selector '{selector}' to exist in HTML:\n{}",
            self.html
        );
        self
    }

    /// Assert that no element matches `selector`.
    pub fn assert_not_exists(&self, selector: &str) -> &Self {
        assert!(
            self.query(selector).is_none(),
            "Expected no element matching selector '{selector}', but found one in HTML:\n{}",
            self.html
        );
        self
    }

    /// Assert that the first element matching `selector` contains `expected_text`.
    pub fn assert_text(&self, selector: &str, expected_text: &str) -> &Self {
        let el = self.query(selector).unwrap_or_else(|| {
            panic!(
                "Element '{selector}' not found when asserting text '{expected_text}' in HTML:\n{}",
                self.html
            )
        });
        assert!(
            el.inner_text.contains(expected_text),
            "Expected '{selector}' text to contain '{expected_text}', got '{}'",
            el.inner_text
        );
        self
    }

    /// Assert that the first element matching `selector` has attribute `attr_name` equal to `expected_val`.
    pub fn assert_attr(&self, selector: &str, attr_name: &str, expected_val: &str) -> &Self {
        let el = self.query(selector).unwrap_or_else(|| {
            panic!(
                "Element '{selector}' not found when asserting attribute '{attr_name}' in HTML:\n{}",
                self.html
            )
        });
        let actual = el
            .attributes
            .get(attr_name)
            .map(String::as_str)
            .unwrap_or("");
        assert_eq!(
            actual, expected_val,
            "Expected '{selector}' attribute '{attr_name}' to be '{expected_val}', got '{actual}'"
        );
        self
    }
}

fn matches_selector(el: &DomTag, sel: &str) -> bool {
    if let Some(id) = sel.strip_prefix('#') {
        return el.attributes.get("id").is_some_and(|v| v == id);
    }
    if let Some(cls) = sel.strip_prefix('.') {
        return el
            .attributes
            .get("class")
            .is_some_and(|c| c.split_whitespace().any(|w| w == cls));
    }
    if sel.starts_with('[') && sel.ends_with(']') {
        let inner = &sel[1..sel.len() - 1];
        if let Some((k, v)) = inner.split_once('=') {
            let clean_v = v.trim().trim_matches('"').trim_matches('\'');
            return el.attributes.get(k.trim()).is_some_and(|av| av == clean_v);
        }
        return el.attributes.contains_key(inner.trim());
    }
    if let Some((tag_part, cls_part)) = sel.split_once('.') {
        return el.tag.eq_ignore_ascii_case(tag_part)
            && el
                .attributes
                .get("class")
                .is_some_and(|c| c.split_whitespace().any(|w| w == cls_part));
    }
    if let Some((tag_part, id_part)) = sel.split_once('#') {
        return el.tag.eq_ignore_ascii_case(tag_part)
            && el.attributes.get("id").is_some_and(|v| v == id_part);
    }
    el.tag.eq_ignore_ascii_case(sel)
}

fn parse_html_tags(html: &str) -> Vec<DomTag> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if i + 1 < bytes.len() && (bytes[i + 1] == b'/' || bytes[i + 1] == b'!') {
                if let Some(end) = html[i..].find('>') {
                    i += end + 1;
                    continue;
                }
                break;
            }
            if let Some(tag_end_rel) = html[i..].find('>') {
                let open_tag_slice = &html[i + 1..i + tag_end_rel];
                let clean_open = open_tag_slice.trim_end_matches('/').trim();
                let (tag_name, attrs) = parse_tag_and_attrs(clean_open);
                if !tag_name.is_empty() {
                    let content_start = i + tag_end_rel + 1;
                    let close_needle = format!("</{tag_name}>");
                    let inner_text = if open_tag_slice.ends_with('/') {
                        String::new()
                    } else if let Some(close_rel) = html[content_start..].find(&close_needle) {
                        strip_tags(&html[content_start..content_start + close_rel])
                    } else {
                        String::new()
                    };
                    out.push(DomTag {
                        tag: tag_name,
                        attributes: attrs,
                        inner_text,
                        raw_outer: html[i..=i + tag_end_rel].to_string(),
                    });
                }
                i += tag_end_rel + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn parse_tag_and_attrs(open_content: &str) -> (String, HashMap<String, String>) {
    let mut attrs = HashMap::new();
    let mut chars = open_content.chars().peekable();
    let mut tag = String::new();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            break;
        }
        tag.push(c);
        chars.next();
    }

    let rest = chars.collect::<String>();
    let mut idx = 0;
    let rbytes = rest.as_bytes();
    while idx < rbytes.len() {
        while idx < rbytes.len() && rbytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= rbytes.len() {
            break;
        }
        let key_start = idx;
        while idx < rbytes.len() && !rbytes[idx].is_ascii_whitespace() && rbytes[idx] != b'=' {
            idx += 1;
        }
        let key = rest[key_start..idx].trim().to_string();
        if key.is_empty() {
            break;
        }
        while idx < rbytes.len() && rbytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx < rbytes.len() && rbytes[idx] == b'=' {
            idx += 1;
            while idx < rbytes.len() && rbytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            if idx < rbytes.len() && (rbytes[idx] == b'"' || rbytes[idx] == b'\'') {
                let quote = rbytes[idx];
                idx += 1;
                let val_start = idx;
                while idx < rbytes.len() && rbytes[idx] != quote {
                    idx += 1;
                }
                let val = rest[val_start..idx].to_string();
                if idx < rbytes.len() {
                    idx += 1;
                }
                attrs.insert(key, val);
            } else {
                let val_start = idx;
                while idx < rbytes.len() && !rbytes[idx].is_ascii_whitespace() {
                    idx += 1;
                }
                attrs.insert(key, rest[val_start..idx].to_string());
            }
        } else {
            attrs.insert(key, "true".to_string());
        }
    }

    (tag, attrs)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::{html_response, redirect};
    use crate::session::Session;
    use yav_core::{Element, IntoNode};

    #[tokio::test]
    async fn test_e2e_client_cookie_persistence_and_dom_assertions() {
        let app = YavApp::new()
            .get("/login", |_req| async {
                html_response(
                    StatusCode::OK,
                    r#"<form id="login-form" action="/login" method="POST">
                        <input id="email-input" name="email" type="email" required />
                        <button class="btn-submit" type="submit">Se connecter</button>
                    </form>"#,
                )
            })
            .post("/login", |req: Request<Body>| async move {
                if let Some(session) = req.extensions().get::<Session>() {
                    session.set("user", "alice@yav.rs");
                }
                redirect("/dashboard")
            })
            .get("/dashboard", |req: Request<Body>| async move {
                let user = req
                    .extensions()
                    .get::<Session>()
                    .and_then(|s| s.get("user"))
                    .unwrap_or_else(|| "guest".to_string());
                html_response(
                    StatusCode::OK,
                    format!(
                        r#"<div id="welcome-banner" data-user="{user}">Bienvenue {user}</div>"#
                    ),
                )
            });

        let client = TestClient::from_app(app);

        // Step 1: GET /login and inspect DOM
        let login_res = client.get("/login").await;
        login_res
            .assert_status(StatusCode::OK)
            .dom()
            .assert_exists("#login-form")
            .assert_attr("#email-input", "type", "email")
            .assert_text(".btn-submit", "Se connecter");

        // Step 2: POST /login -> sets session cookie and redirects
        let post_res = client
            .post_form("/login", &[("email", "alice@yav.rs")])
            .await;
        post_res.assert_redirect("/dashboard");

        // Step 3: GET /dashboard -> automatically sends persisted session cookie!
        let dash_res = client.get("/dashboard").await;
        dash_res
            .assert_status(StatusCode::OK)
            .dom()
            .assert_exists("#welcome-banner")
            .assert_attr("#welcome-banner", "data-user", "alice@yav.rs")
            .assert_text("#welcome-banner", "Bienvenue alice@yav.rs");
    }

    #[test]
    fn test_dom_assert_from_node() {
        let node = Element::new("section")
            .attr("id", "hero")
            .attr("class", "bg-slate-950 p-8")
            .child(Element::new("h1").child(Node::text("Yav Framework")))
            .into_node();

        DomAssert::from_node(&node)
            .assert_exists("#hero")
            .assert_exists(".bg-slate-950")
            .assert_text("h1", "Yav Framework");
    }
}
