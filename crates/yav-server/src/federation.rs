//! Micro-Frontends & Server-Side Module Federation Gateway.
//!
//! Enables composing independent Yav services or upstream web applications
//! into a single unified layout with HTML fragment stitching and cookie forwarding.

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
    pub forward_auth: bool,
    /// Fallback HTML displayed if the upstream microservice times out or fails.
    pub fallback_html: Option<String>,
}

/// Gateway orchestrating routing and HTML composition across federated microservices.
#[derive(Debug, Clone, Default)]
pub struct FederationGateway {
    routes: Vec<FederationRoute>,
}

impl FederationGateway {
    /// Create a new empty `FederationGateway`.
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Register a federated microservice upstream route.
    #[must_use]
    pub fn route(mut self, prefix: impl Into<String>, upstream_url: impl Into<String>) -> Self {
        self.routes.push(FederationRoute {
            prefix: prefix.into(),
            upstream_url: upstream_url.into(),
            strip_prefix: false,
            forward_auth: true,
            fallback_html: None,
        });
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

    /// Seamlessly stitches an upstream microservice HTML fragment into a parent document shell.
    ///
    /// Replaces `<!-- FEDERATION_SLOT:name -->` or `<div id="slot-name">` with the upstream fragment.
    pub fn stitch_slot(parent_shell: &str, slot_name: &str, upstream_fragment: &str) -> String {
        let comment_placeholder = format!("<!-- FEDERATION_SLOT:{slot_name} -->");
        if parent_shell.contains(&comment_placeholder) {
            return parent_shell.replace(&comment_placeholder, upstream_fragment);
        }

        let container_placeholder = format!("data-yav-slot=\"{slot_name}\">");
        if let Some(idx) = parent_shell.find(&container_placeholder) {
            let insert_point = idx + container_placeholder.len();
            let mut out = String::with_capacity(parent_shell.len() + upstream_fragment.len());
            out.push_str(&parent_shell[..insert_point]);
            out.push_str(upstream_fragment);
            out.push_str(&parent_shell[insert_point..]);
            return out;
        }

        format!(
            "{parent_shell}\n<!-- Appended Federated Slot: {slot_name} -->\n{upstream_fragment}"
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
