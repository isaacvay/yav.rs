//! Micro-Frontends & Server-Side Module Federation Gateway.
//!
//! Enables composing independent Yav services or upstream web applications
//! into a single unified layout with HTML fragment stitching and real HTTP
//! reverse proxying via `reqwest`.
//!
//! ## Features
//! - Route-based upstream resolution with prefix matching.
//! - Real HTTP proxying via [`FederationGateway::proxy_request`].
//! - Automatic slot stitching with fallback HTML on upstream failure.
//! - [`FederationGateway::fetch_and_stitch_slots`] fetches every slot's
//!   fragment concurrently via `join_all`, then stitches in declaration order.
//! - Opt-in cookie/auth header forwarding (off by default, see
//!   [`FederationRoute::forward_auth`]).
//! - Explicit upstream host allowlist and conservative fragment sanitization.
//!
//! ## Security
//! - `forward_auth` defaults to `false` so client credentials are never leaked
//!   to an upstream by accident.
//! - `proxy_request` refuses any upstream host not present in
//!   `allowed_upstream_hosts` (an empty allowlist is an explicit deny of
//!   everything; there is no implicit "allow all" default).
//! - Stitched HTML fragments are sanitized by [`sanitize_upstream_fragment`]
//!   unless `sanitize_fragments` is explicitly disabled.

use std::time::Duration;

/// Configuration for a federated upstream microservice route.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FederationRoute {
    /// URL path prefix handled by the microservice (e.g. `"/billing"` or `"/docs"`).
    pub prefix: String,
    /// Base upstream server URL (e.g. `"http://127.0.0.1:3001"`).
    pub upstream_url: String,
    /// Whether to strip the prefix when proxying to upstream (e.g. `/billing/invoices` -> `/invoices`).
    pub strip_prefix: bool,
    /// Whether to forward client cookies and authentication headers.
    ///
    /// Defaults to `false` (set in [`FederationGateway::route`]). **Security
    /// risk:** enabling this forwards the client's `Cookie` and `Authorization`
    /// headers to the upstream microservice, which may live in a different
    /// trust domain. Only opt in for upstreams you fully control and that are
    /// pinned in `allowed_upstream_hosts`; a misconfigured, redirected or
    /// compromised upstream can otherwise steal session credentials.
    pub forward_auth: bool,
    /// Fallback HTML displayed if the upstream microservice times out or fails.
    pub fallback_html: Option<String>,
}

/// Result of a federation proxy request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyResult {
    /// HTTP status code from the upstream.
    pub status: u16,
    /// Response body from the upstream.
    pub body: String,
    /// Content-Type header from the upstream.
    pub content_type: String,
}

/// Error returned when a federation proxy request fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FederationError {
    /// No matching route was found for the requested path.
    NoRoute,
    /// The upstream server returned an error or was unreachable.
    UpstreamError(String),
    /// The request timed out waiting for the upstream.
    Timeout,
    /// The upstream host is not present in `allowed_upstream_hosts` (or the
    /// allowlist is empty). The request is refused before any network I/O.
    HostNotAllowed(String),
}

impl std::fmt::Display for FederationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRoute => write!(f, "No federation route matched the request path"),
            Self::UpstreamError(msg) => write!(f, "Upstream error: {msg}"),
            Self::Timeout => write!(f, "Upstream request timed out"),
            Self::HostNotAllowed(msg) => write!(f, "Upstream host not allowed: {msg}"),
        }
    }
}

impl std::error::Error for FederationError {}

// ---------------------------------------------------------------------------
// Security helpers: case-insensitive ASCII string manipulation, no dependencies.
// ---------------------------------------------------------------------------

/// Case-insensitive ASCII substring search. Returns the absolute byte index of
/// the first match at or after `from`, or `None`.
fn find_ci(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || n.len() > h.len() {
        return None;
    }
    let last = h.len() - n.len();
    let mut i = from;
    while i <= last {
        if h[i..i + n.len()]
            .iter()
            .zip(n)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract a comparable host token from a URL or allowlist entry: lowercased,
/// with scheme, userinfo, path and port stripped. Handles IPv6 literals.
fn extract_host(url: &str) -> Option<String> {
    let rest = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => url,
    };
    let authority = rest
        .split('/')
        .next()?
        .split('?')
        .next()?
        .split('#')
        .next()?;
    let authority = authority.rsplit('@').next()?;
    let host = if authority.starts_with('[') {
        &authority[..authority.find(']')? + 1]
    } else {
        authority.split(':').next()?
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// Remove every case-insensitive `javascript:` occurrence from `input`.
fn strip_javascript_scheme(input: &str) -> String {
    let pattern = "javascript:";
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(i) = find_ci(input, pattern, cursor) {
        out.push_str(&input[cursor..i]);
        cursor = i + pattern.len();
    }
    out.push_str(&input[cursor..]);
    out
}

/// Conservatively strip active content from an upstream HTML fragment.
///
/// Case-insensitive and dependency-free. Removes:
/// - `<script ...>...</script>` blocks (unterminated ones drop the remainder),
/// - `<iframe ...>...</iframe>` blocks,
/// - stray `</script>` / `</iframe>` closing tags,
/// - inline `on*` event-handler attributes (`onclick`, `onerror`, ...),
/// - `javascript:` URL schemes (both in attributes and in text).
///
/// This is deliberately conservative defense-in-depth for untrusted upstream
/// fragments, **not** a full HTML sanitizer: unknown/unusual markup is left
/// intact. Callers should still serve a strict Content-Security-Policy.
pub fn sanitize_upstream_fragment(fragment: &str) -> String {
    let cleaned = yav_core::sanitize_html_strict(fragment);
    strip_javascript_scheme(&cleaned)
}

/// One slot to resolve in a parallel federation request.
///
/// `name` is the placeholder name in the parent shell (matched against
/// `<!-- FEDERATION_SLOT:name -->` or `data-yav-slot="name"`), `upstream_path`
/// is the path resolved against the gateway's registered routes, and
/// `timeout_secs` is the per-slot HTTP timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationSlot {
    pub name: String,
    pub upstream_path: String,
    pub timeout_secs: u64,
}

impl FederationSlot {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        upstream_path: impl Into<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            name: name.into(),
            upstream_path: upstream_path.into(),
            timeout_secs,
        }
    }
}

/// Gateway orchestrating routing and HTML composition across federated microservices.
#[derive(Debug, Clone)]
pub struct FederationGateway {
    routes: Vec<FederationRoute>,
    /// Explicit allowlist of upstream hosts. Empty = refuse every proxy request.
    allowed_upstream_hosts: Vec<String>,
    /// Whether to sanitize stitched upstream fragments (default `true`).
    sanitize_fragments: bool,
}

impl Default for FederationGateway {
    fn default() -> Self {
        Self {
            routes: Vec::new(),
            // Empty = explicit deny. Never default to "allow everything".
            allowed_upstream_hosts: Vec::new(),
            sanitize_fragments: true,
        }
    }
}

impl FederationGateway {
    /// Create a new empty `FederationGateway`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a federated microservice upstream route.
    ///
    /// Cookie/Authorization forwarding is **off** by default (see
    /// [`FederationRoute::forward_auth`]).
    #[must_use]
    pub fn route(mut self, prefix: impl Into<String>, upstream_url: impl Into<String>) -> Self {
        self.routes.push(FederationRoute {
            prefix: prefix.into(),
            upstream_url: upstream_url.into(),
            strip_prefix: false,
            forward_auth: false,
            fallback_html: None,
        });
        self
    }

    /// Set the explicit allowlist of upstream hosts permitted by the proxy.
    ///
    /// Entries are host names (optionally `host:port` or full URLs); only the
    /// host component is compared, case-insensitively. An empty list (the
    /// default) refuses **all** proxy requests — there is no implicit
    /// "allow everything" default.
    #[must_use]
    pub fn with_allowed_upstream_hosts(
        mut self,
        hosts: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed_upstream_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }

    /// Enable or disable conservative sanitization of stitched fragments.
    /// Enabled by default; disabling it is strongly discouraged.
    #[must_use]
    pub fn with_sanitize_fragments(mut self, enabled: bool) -> Self {
        self.sanitize_fragments = enabled;
        self
    }

    /// Set fallback HTML for an existing prefix in case upstream is offline.
    #[must_use]
    pub fn with_fallback(mut self, prefix: &str, fallback_html: impl Into<String>) -> Self {
        if let Some(r) = self.routes.iter_mut().find(|r| r.prefix == prefix) {
            r.fallback_html = Some(fallback_html.into());
        }
        self
    }

    /// Refuse to proxy unless the upstream host is explicitly allowlisted.
    fn ensure_host_allowed(&self, url: &str) -> Result<(), FederationError> {
        let host = extract_host(url).unwrap_or_default();
        if self.allowed_upstream_hosts.is_empty() {
            return Err(FederationError::HostNotAllowed(format!(
                "host '{host}' refused: allowed_upstream_hosts is empty (explicit deny). \
                 Configure FederationGateway::with_allowed_upstream_hosts([...]) to \
                 permit specific upstream hosts"
            )));
        }
        let permitted = self
            .allowed_upstream_hosts
            .iter()
            .any(|entry| extract_host(entry).is_some_and(|allowed| allowed == host));
        if permitted {
            Ok(())
        } else {
            Err(FederationError::HostNotAllowed(format!(
                "host '{host}' is not in allowed_upstream_hosts; refusing to proxy"
            )))
        }
    }

    /// Find matching upstream route for a requested request path.
    pub fn resolve(&self, path: &str) -> Option<&FederationRoute> {
        let norm = if path.starts_with('/') { path } else { "/" };
        self.routes.iter().find(|r| {
            if norm == r.prefix {
                true
            } else if norm.starts_with(&r.prefix) {
                let rest = &norm[r.prefix.len()..];
                rest.starts_with('/')
            } else {
                false
            }
        })
    }

    /// Compute target upstream URL for a request path.
    pub fn target_url(&self, route: &FederationRoute, request_path: &str) -> String {
        let base = route.upstream_url.trim_end_matches('/');
        let path = if route.strip_prefix {
            request_path
                .strip_prefix(&route.prefix)
                .unwrap_or(request_path)
        } else {
            request_path
        };
        let clean_path = if path.starts_with('/') { path } else { "/" };
        format!("{base}{clean_path}")
    }

    /// Seamlessly stitch an upstream microservice HTML fragment into a parent
    /// document shell.
    ///
    /// Replaces `<!-- FEDERATION_SLOT:name -->` or `<div id="slot-name">` with
    /// the upstream fragment. Direct calls always sanitize the fragment with
    /// [`sanitize_upstream_fragment`] (secure default); the gateway-level
    /// `sanitize_fragments` flag controls [`Self::fetch_and_stitch_slot`].
    pub fn stitch_slot(parent_shell: &str, slot_name: &str, upstream_fragment: &str) -> String {
        Self::stitch_slot_impl(parent_shell, slot_name, upstream_fragment, true)
    }

    fn stitch_slot_impl(
        parent_shell: &str,
        slot_name: &str,
        upstream_fragment: &str,
        sanitize: bool,
    ) -> String {
        let fragment = if sanitize {
            sanitize_upstream_fragment(upstream_fragment)
        } else {
            upstream_fragment.to_string()
        };
        let comment_placeholder = format!("<!-- FEDERATION_SLOT:{slot_name} -->");
        if parent_shell.contains(&comment_placeholder) {
            return parent_shell.replace(&comment_placeholder, &fragment);
        }

        let container_placeholder = format!("data-yav-slot=\"{slot_name}\">");
        if let Some(idx) = parent_shell.find(&container_placeholder) {
            let insert_point = idx + container_placeholder.len();
            let mut out = String::with_capacity(parent_shell.len() + fragment.len());
            out.push_str(&parent_shell[..insert_point]);
            out.push_str(&fragment);
            out.push_str(&parent_shell[insert_point..]);
            return out;
        }

        format!("{parent_shell}\n<!-- Appended Federated Slot: {slot_name} -->\n{fragment}")
    }

    /// Merge deduplicated safe `<head>` `<link>`/`<meta>` tags from upstream documents
    /// into `parent_shell` alongside their sanitized `<body>` fragments at `slot_name`.
    pub fn stitch_with_head(parent_shell: &str, slots: &[(&str, &str)]) -> String {
        use std::collections::HashSet;
        let mut seen_head_lines = HashSet::new();
        let mut extra_head = Vec::new();
        let mut out = parent_shell.to_string();

        for (slot_name, full_doc) in slots {
            let (head, body) = Self::parse_upstream_document(full_doc);
            for raw_line in head.lines() {
                let t = raw_line.trim();
                let lower = t.to_ascii_lowercase();
                if (lower.starts_with("<link ") || lower.starts_with("<meta "))
                    && !lower.contains("javascript:")
                    && !lower.contains("on")
                    && seen_head_lines.insert(t.to_string())
                {
                    extra_head.push(t.to_string());
                }
            }
            out = Self::stitch_slot(&out, slot_name, &body);
        }

        if !extra_head.is_empty() {
            let head_block = extra_head.join("\n");
            if let Some(pos) = out.to_ascii_lowercase().find("</head>") {
                let mut merged = String::with_capacity(out.len() + head_block.len() + 2);
                merged.push_str(&out[..pos]);
                merged.push('\n');
                merged.push_str(&head_block);
                merged.push('\n');
                merged.push_str(&out[pos..]);
                return merged;
            }
        }
        out
    }

    /// Extract separate `<head>` assets and `<body>` contents from an upstream HTML document.
    pub fn parse_upstream_document(full_html: &str) -> (String, String) {
        let mut head = String::new();
        let mut body = String::new();

        if let Some(h_start) = full_html.find("<head>") {
            if let Some(h_end) = full_html[h_start..].find("</head>") {
                head = full_html[h_start + 6..h_start + h_end].trim().to_string();
            }
        }

        if let Some(b_start) = full_html.find("<body>") {
            if let Some(b_end) = full_html[b_start..].find("</body>") {
                body = full_html[b_start + 6..b_start + b_end].trim().to_string();
            }
        } else {
            body = full_html.to_string();
        }

        (head, body)
    }

    /// Proxy an incoming request to the matching upstream microservice via HTTP.
    ///
    /// Uses `reqwest` to perform a real HTTP request to the upstream server,
    /// forwarding cookies and auth headers only when `forward_auth` is enabled.
    /// The upstream host is validated against `allowed_upstream_hosts`
    /// **before** any network I/O (an empty allowlist refuses everything).
    ///
    /// # Arguments
    /// * `request_path` - The URL path of the incoming request (e.g. `/billing/invoices`).
    /// * `method` - The HTTP method (`GET`, `POST`, etc.).
    /// * `cookie_header` - Optional Cookie header value to forward.
    /// * `authorization` - Optional Authorization header value to forward.
    /// * `body` - Optional request body for POST/PUT requests.
    /// * `timeout_secs` - Request timeout in seconds.
    ///
    /// # Errors
    /// Returns `FederationError::NoRoute` if no route matches,
    /// `FederationError::HostNotAllowed` if the upstream host is not allowlisted,
    /// `FederationError::UpstreamError` if the HTTP request fails,
    /// `FederationError::Timeout` if the upstream doesn't respond in time.
    pub async fn proxy_request(
        &self,
        request_path: &str,
        method: &str,
        cookie_header: Option<&str>,
        authorization: Option<&str>,
        body: Option<&[u8]>,
        timeout_secs: u64,
    ) -> Result<ProxyResult, FederationError> {
        let route = self.resolve(request_path).ok_or(FederationError::NoRoute)?;
        self.ensure_host_allowed(&route.upstream_url)?;
        let target_url = self.target_url(route, request_path);

        static FEDERATION_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = FEDERATION_CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build()
                .expect("failed to build federation reqwest client")
        });

        let req_method = method.to_ascii_uppercase();
        let mut builder = match req_method.as_str() {
            "GET" => client.get(&target_url),
            "POST" => client.post(&target_url),
            "PUT" => client.put(&target_url),
            "DELETE" => client.delete(&target_url),
            "PATCH" => client.patch(&target_url),
            _ => client.get(&target_url),
        }
        .timeout(Duration::from_secs(timeout_secs));

        if route.forward_auth {
            if let Some(cookie) = cookie_header {
                builder = builder.header("Cookie", cookie);
            }
            if let Some(auth) = authorization {
                builder = builder.header("Authorization", auth);
            }
        }

        if let Some(b) = body {
            builder = builder.body(b.to_vec());
        }

        let resp = builder.send().await.map_err(|e| {
            if e.is_timeout() {
                FederationError::Timeout
            } else {
                FederationError::UpstreamError(e.to_string())
            }
        })?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();
        let resp_body = resp
            .text()
            .await
            .map_err(|e| FederationError::UpstreamError(e.to_string()))?;

        Ok(ProxyResult {
            status,
            body: resp_body,
            content_type,
        })
    }

    /// Fetch one upstream fragment, applying the route's fallback on failure.
    ///
    /// Kept separate from stitching so the parallel API can fetch every slot
    /// before touching the shell: stitching is an ordered, pure string fold.
    async fn fetch_fragment(
        &self,
        upstream_path: &str,
        timeout_secs: u64,
    ) -> Result<String, FederationError> {
        let route = self
            .resolve(upstream_path)
            .ok_or(FederationError::NoRoute)?;
        // Clone before the await so no borrow of `self.routes` is held across it.
        let fallback = route.fallback_html.clone();

        match self
            .proxy_request(upstream_path, "GET", None, None, None, timeout_secs)
            .await
        {
            Ok(result) if result.status >= 400 => fallback.ok_or_else(|| {
                FederationError::UpstreamError(format!(
                    "Upstream returned HTTP status {}",
                    result.status
                ))
            }),
            Ok(result) => Ok(result.body),
            Err(e) => fallback.ok_or(e),
        }
    }

    /// Fetch an HTML fragment from an upstream microservice and stitch it
    /// into a parent document at the named slot position.
    ///
    /// The fetched fragment is sanitized with [`sanitize_upstream_fragment`]
    /// when `sanitize_fragments` is enabled (the default). If the upstream is
    /// unreachable or its host is not allowlisted, falls back to
    /// `fallback_html` if configured, otherwise returns an error.
    pub async fn fetch_and_stitch_slot(
        &self,
        parent_shell: &str,
        slot_name: &str,
        upstream_path: &str,
        timeout_secs: u64,
    ) -> Result<String, FederationError> {
        let fragment = self.fetch_fragment(upstream_path, timeout_secs).await?;
        Ok(Self::stitch_slot_impl(
            parent_shell,
            slot_name,
            &fragment,
            self.sanitize_fragments,
        ))
    }

    /// Fetch every slot's fragment **concurrently** and stitch them into the
    /// parent shell in declaration order.
    ///
    /// `join_all` drives all upstream requests on the same task; a slow slot no
    /// longer delays the others. Stitching still happens sequentially after all
    /// fragments arrive, because each replacement folds over the previous
    /// result. Sanitization and per-route fallbacks behave exactly as in
    /// [`Self::fetch_and_stitch_slot`]. The first error in declaration order is
    /// returned; fragments already fetched are discarded in that case.
    pub async fn fetch_and_stitch_slots(
        &self,
        parent_shell: &str,
        slots: &[FederationSlot],
    ) -> Result<String, FederationError> {
        let fetches = slots
            .iter()
            .map(|slot| self.fetch_fragment(&slot.upstream_path, slot.timeout_secs));
        let fragments = futures_util::future::join_all(fetches).await;

        let mut out = parent_shell.to_string();
        for (slot, fragment) in slots.iter().zip(fragments) {
            let fragment = fragment?;
            out = Self::stitch_slot_impl(&out, &slot.name, &fragment, self.sanitize_fragments);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: a gateway with a configured allowlist. The default is an
    /// explicit deny, so any test that exercises the proxy must opt in.
    fn gateway_with_hosts(prefix: &str, url: &str, hosts: &[&str]) -> FederationGateway {
        FederationGateway::new()
            .route(prefix, url)
            .with_allowed_upstream_hosts(hosts.iter().copied())
    }

    #[tokio::test]
    async fn test_proxy_request_no_route_returns_error() {
        let gateway = FederationGateway::new();
        let result = gateway
            .proxy_request("/unknown", "GET", None, None, None, 5)
            .await;
        assert_eq!(result, Err(FederationError::NoRoute));
    }

    #[tokio::test]
    async fn test_fetch_and_stitch_uses_fallback_on_failure() {
        let gateway = gateway_with_hosts("/billing", "http://127.0.0.1:1", &["127.0.0.1"])
            .with_fallback("/billing", "<div>Fallback Billing</div>");

        let shell = r#"<main><!-- FEDERATION_SLOT:content --></main>"#;
        let result = gateway
            .fetch_and_stitch_slot(shell, "content", "/billing/invoices", 2)
            .await
            .expect("should use fallback");

        assert!(result.contains("Fallback Billing"));
        assert!(!result.contains("FEDERATION_SLOT"));
    }

    #[tokio::test]
    async fn test_fetch_and_stitch_slots_uses_fallbacks_and_keeps_order() {
        // Both upstreams point at a closed port, so every fetch fails and the
        // configured fallback is used. This exercises the parallel API without
        // any network dependency.
        let gateway = FederationGateway::new()
            .route("/billing", "http://127.0.0.1:1/billing")
            .route("/catalog", "http://127.0.0.1:1/catalog")
            .with_allowed_upstream_hosts(["127.0.0.1"])
            .with_fallback("/billing", "<div>Billing fallback</div>")
            .with_fallback("/catalog", "<div>Catalog fallback</div>");

        let shell = "<main><!-- FEDERATION_SLOT:top --><!-- FEDERATION_SLOT:bottom --></main>";
        let slots = [
            FederationSlot::new("top", "/billing/invoices", 2),
            FederationSlot::new("bottom", "/catalog/items", 2),
        ];

        let out = gateway
            .fetch_and_stitch_slots(shell, &slots)
            .await
            .expect("fallbacks should cover both failed fetches");

        assert!(out.contains("Billing fallback"));
        assert!(out.contains("Catalog fallback"));
        assert!(!out.contains("FEDERATION_SLOT:"));
        assert!(
            out.find("Billing fallback").unwrap() < out.find("Catalog fallback").unwrap(),
            "slots must be stitched in declaration order"
        );
    }

    #[tokio::test]
    async fn test_fetch_and_stitch_slots_fetches_concurrently() {
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::sync::Barrier;
        use tokio::time::timeout;

        // The upstream accepts two connections but does not answer either until
        // both requests have arrived. A sequential implementation would wait
        // forever on the first request and time out; a concurrent one lets both
        // reach the barrier and completes.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().unwrap();
        let barrier = Arc::new(Barrier::new(2));

        tokio::spawn({
            let barrier = Arc::clone(&barrier);
            async move {
                for _ in 0..2 {
                    if let Ok((mut socket, _)) = listener.accept().await {
                        let b = Arc::clone(&barrier);
                        tokio::spawn(async move {
                            let mut buf = [0u8; 2048];
                            let _ = socket.read(&mut buf).await;
                            b.wait().await;
                            let body = "<div>upstream-ok</div>";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        });
                    }
                }
            }
        });

        let base = format!("http://{addr}");
        let gateway = FederationGateway::new()
            .route("/a", base.clone())
            .route("/b", base)
            .with_allowed_upstream_hosts(["127.0.0.1"]);

        let slots = [
            FederationSlot::new("left", "/a/one", 5),
            FederationSlot::new("right", "/b/two", 5),
        ];

        let result = timeout(
            Duration::from_secs(4),
            gateway.fetch_and_stitch_slots(
                "<main><!-- FEDERATION_SLOT:left --><!-- FEDERATION_SLOT:right --></main>",
                &slots,
            ),
        )
        .await
        .expect("slots were fetched sequentially: the barrier never completed")
        .expect("both upstream requests should succeed");

        assert_eq!(result.matches("upstream-ok").count(), 2);
        assert!(!result.contains("FEDERATION_SLOT:"));
    }

    #[tokio::test]
    async fn test_fetch_and_stitch_slots_sanitizes_fragments() {
        let gateway = FederationGateway::new()
            .route("/unsafe", "http://127.0.0.1:1/unsafe")
            .with_allowed_upstream_hosts(["127.0.0.1"])
            .with_fallback(
                "/unsafe",
                "<div>safe</div><script>alert(1)</script><iframe src=\"x\"></iframe>",
            );

        let slots = [FederationSlot::new("only", "/unsafe/item", 2)];
        let out = gateway
            .fetch_and_stitch_slots("<main><!-- FEDERATION_SLOT:only --></main>", &slots)
            .await
            .expect("fallback should be used");

        assert!(out.contains("safe"));
        assert!(!out.contains("alert(1)"));
        assert!(!out.contains("<iframe"));
    }

    #[test]
    fn test_federation_routing_resolution() {
        let gateway = FederationGateway::new()
            .route("/billing", "http://billing-svc:3001")
            .route("/admin", "http://admin-svc:3002");

        let route = gateway
            .resolve("/billing/invoices/99")
            .expect("Should resolve");
        assert_eq!(route.prefix, "/billing");
        assert_eq!(
            gateway.target_url(route, "/billing/invoices/99"),
            "http://billing-svc:3001/billing/invoices/99"
        );

        assert!(gateway.resolve("/unknown/path").is_none());
    }

    #[test]
    fn test_federation_slot_stitching() {
        let shell = r#"<!DOCTYPE html><html><body><nav>Header</nav><main><!-- FEDERATION_SLOT:content --></main></body></html>"#;
        let fragment = r#"<div class="billing-page"><h1>Invoices</h1></div>"#;

        let stitched = FederationGateway::stitch_slot(shell, "content", fragment);
        assert!(stitched.contains(r#"<div class="billing-page"><h1>Invoices</h1></div>"#));
        assert!(!stitched.contains("<!-- FEDERATION_SLOT:content -->"));
    }

    // --- security regressions -------------------------------------------

    #[test]
    fn test_forward_auth_defaults_to_false() {
        let gateway = FederationGateway::new().route("/billing", "http://billing-svc:3001");
        let route = gateway.resolve("/billing").expect("should resolve");
        assert!(!route.forward_auth);
    }

    #[tokio::test]
    async fn test_unauthorized_upstream_host_is_rejected() {
        // Empty allowlist = explicit deny of everything, with a clear message.
        let deny_all = FederationGateway::new().route("/x", "http://upstream.example.com");
        let err = deny_all
            .proxy_request("/x/p", "GET", None, None, None, 5)
            .await
            .unwrap_err();
        assert!(matches!(err, FederationError::HostNotAllowed(_)));
        assert!(err.to_string().contains("allowed_upstream_hosts"));

        // A configured allowlist still refuses an unlisted host.
        let gateway = gateway_with_hosts("/x", "http://evil.example.com", &["good.example.com"]);
        let err = gateway
            .proxy_request("/x/p", "GET", None, None, None, 5)
            .await
            .unwrap_err();
        assert!(matches!(err, FederationError::HostNotAllowed(_)));
    }

    #[tokio::test]
    async fn test_allowlisted_host_passes_the_allowlist_guard() {
        // Host is allowed: the request proceeds to (failing) network I/O and
        // is NOT rejected by the allowlist guard.
        let gateway = gateway_with_hosts("/x", "http://127.0.0.1:1", &["127.0.0.1"]);
        let err = gateway
            .proxy_request("/x/p", "GET", None, None, None, 2)
            .await
            .unwrap_err();
        assert!(!matches!(err, FederationError::HostNotAllowed(_)));
    }

    #[test]
    fn test_injected_script_is_removed_from_stitched_fragment() {
        let fragment = r#"<div>ok</div><script>alert(1)</script>"#;
        let stitched = FederationGateway::stitch_slot(
            "<main><!-- FEDERATION_SLOT:content --></main>",
            "content",
            fragment,
        );
        assert!(stitched.contains("<div>ok</div>"));
        assert!(!stitched.to_ascii_lowercase().contains("<script"));
        assert!(!stitched.contains("alert(1)"));
    }

    #[test]
    fn test_sanitize_strips_iframe_handlers_javascript_and_mixed_case_scripts() {
        let fragment = concat!(
            "<div onclick=\"steal()\" ONERROR=boom class=\"keep\">Hi</div>",
            "<iframe src=\"https://evil.example\"></iframe>",
            "<a href=\"JaVaScRiPt:alert(1)\">link</a>",
            "<SCRIPT SRC=\"x.js\">alert(2)</SCRIPT>",
            "javascript:tail"
        );
        let clean = sanitize_upstream_fragment(fragment);
        let lower = clean.to_ascii_lowercase();
        assert!(!lower.contains("<iframe"));
        assert!(!lower.contains("<script"));
        assert!(!lower.contains("onclick"));
        assert!(!lower.contains("onerror"));
        assert!(!lower.contains("javascript:"));
        assert!(!lower.contains("alert(1)"));
        assert!(!lower.contains("alert(2)"));
        assert!(clean.contains("class=\"keep\""));
        assert!(clean.contains(">Hi</div>"));
    }

    #[test]
    fn test_sanitize_fragments_flag_defaults_true_and_can_be_disabled() {
        assert!(FederationGateway::new().sanitize_fragments);
        let gateway = FederationGateway::new().with_sanitize_fragments(false);
        assert!(!gateway.sanitize_fragments);
        // Direct stitch_slot always sanitizes regardless of the flag.
        let stitched = FederationGateway::stitch_slot("<p>SLOT</p>", "s", "<script>x()</script>");
        assert!(!stitched.to_ascii_lowercase().contains("<script"));
    }
}
