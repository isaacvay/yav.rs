//! OpenAPI 3.1 schema generator and embedded API documentation pages.
//!
//! Generates compliant OpenAPI JSON descriptions for hybrid full-stack
//! applications and serves **offline, dependency-free** reference pages. The
//! pages are static HTML (no CDN bundles, no JavaScript); mount a real Scalar
//! or Swagger UI application yourself if you need an interactive explorer.

use std::collections::HashMap;

use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// An API endpoint documentation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub summary: String,
    pub tags: Vec<String>,
}

/// OpenAPI 3.1 Specification builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub title: String,
    pub version: String,
    pub description: String,
    pub endpoints: Vec<ApiEndpoint>,
    #[serde(default)]
    pub schemas: HashMap<String, serde_json::Value>,
}

impl OpenApiSpec {
    /// Create a new OpenAPI specification for an application.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: String::new(),
            endpoints: Vec::new(),
            schemas: HashMap::new(),
        }
    }

    /// Set API description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Register a JSON Schema object under `components.schemas[name]` (e.g. from `#[derive(ApiSchema)]`).
    pub fn schema_raw(mut self, name: impl Into<String>, raw_json_schema: &str) -> Self {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_json_schema) {
            self.schemas.insert(name.into(), val);
        }
        self
    }

    /// Register an API endpoint.
    pub fn endpoint(
        mut self,
        path: impl Into<String>,
        method: impl Into<String>,
        summary: impl Into<String>,
        tag: impl Into<String>,
    ) -> Self {
        self.endpoints.push(ApiEndpoint {
            path: path.into(),
            method: method.into().to_uppercase(),
            summary: summary.into(),
            tags: vec![tag.into()],
        });
        self
    }

    /// Render standard OpenAPI 3.1 JSON format.
    pub fn to_json(&self) -> String {
        let mut paths_map: HashMap<String, serde_json::Value> = HashMap::new();

        for ep in &self.endpoints {
            let method_key = ep.method.to_lowercase();
            let op = serde_json::json!({
                "summary": ep.summary,
                "tags": ep.tags,
                "responses": {
                    "200": {
                        "description": "Succès"
                    }
                }
            });

            paths_map
                .entry(ep.path.clone())
                .or_insert_with(|| serde_json::json!({}))
                .as_object_mut()
                .map(|obj| obj.insert(method_key, op));
        }

        let doc = if self.schemas.is_empty() {
            serde_json::json!({
                "openapi": "3.1.0",
                "info": {
                    "title": self.title,
                    "version": self.version,
                    "description": self.description,
                },
                "paths": paths_map
            })
        } else {
            serde_json::json!({
                "openapi": "3.1.0",
                "info": {
                    "title": self.title,
                    "version": self.version,
                    "description": self.description,
                },
                "paths": paths_map,
                "components": {
                    "schemas": self.schemas
                }
            })
        };

        serde_json::to_string_pretty(&doc).unwrap_or_default()
    }

    /// Convert into an Axum [`Response`] with `Content-Type: application/json`.
    pub fn into_response(&self) -> Response {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            self.to_json(),
        )
            .into_response()
    }
}

/// Build a dependency-free, offline API reference page.
///
/// The previous implementation claimed to be "self-contained" while loading
/// Scalar from `cdn.jsdelivr.net` and Swagger UI from `unpkg.com`. Those CDN
/// bundles require network access and are blocked by a strict CSP. This page
/// loads no external script, stylesheet or font and uses no JavaScript at all.
fn offline_reference_html(title: &str, spec_url: &str, flavor: &str) -> String {
    let escaped_title = yav_core::escape_html(title);
    let escaped_flavor = yav_core::escape_html(flavor);
    let escaped_url = yav_core::escape_attribute(spec_url);
    let url_text = yav_core::escape_html(spec_url);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{escaped_title}</title>
<style>
:root {{ color-scheme: light dark; }}
body {{ font: 16px/1.6 system-ui, sans-serif; margin: 0; padding: 2rem; }}
main {{ max-width: 48rem; margin: 0 auto; }}
code {{ background: rgba(127,127,127,.15); padding: .1rem .3rem; border-radius: .25rem; }}
</style>
</head>
<body>
<main>
<h1>{escaped_title}</h1>
<p>Offline API reference ({escaped_flavor}). This page is genuinely self-contained: no external scripts, stylesheets or fonts, and no JavaScript, so it works offline and behind a strict Content-Security-Policy.</p>
<p>OpenAPI 3.1 document: <a href="{escaped_url}"><code>{url_text}</code></a></p>
<p>For an interactive explorer, mount a viewer of your choice yourself; the framework deliberately does not fetch third-party CDN bundles.</p>
</main>
</body>
</html>"#
    )
}

fn render_offline_reference(title: &str, spec_url: &str, flavor: &str) -> Response {
    let html = offline_reference_html(title, spec_url, flavor);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(html),
    )
        .into_response()
}

/// Render the offline Scalar-flavoured API reference page.
///
/// This is a static HTML page, not the upstream Scalar JS application.
pub fn scalar_ui_handler(title: &str, spec_url: &str) -> Response {
    render_offline_reference(title, spec_url, "Scalar layout")
}

/// Render the offline Swagger-flavoured API reference page.
///
/// This is a static HTML page, not the upstream Swagger UI JS application.
pub fn swagger_ui_handler(title: &str, spec_url: &str) -> Response {
    render_offline_reference(title, spec_url, "Swagger layout")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = OpenApiSpec::new("Yav API", "1.0.0")
            .description("Service API pour application Yav")
            .endpoint("/api/v1/users", "get", "Liste des utilisateurs", "Users")
            .endpoint("/api/v1/users", "post", "Créer un utilisateur", "Users");

        let json = spec.to_json();
        assert!(json.contains("openapi"));
        assert!(json.contains("3.1.0"));
        assert!(json.contains("Yav API"));
        assert!(json.contains("/api/v1/users"));
        assert!(json.contains("Liste des utilisateurs"));
    }

    #[test]
    fn test_reference_pages_are_truly_self_contained() {
        let scalar = offline_reference_html("My API", "/openapi.json", "Scalar layout");
        let swagger = offline_reference_html("My API", "/openapi.json", "Swagger layout");
        for html in [&scalar, &swagger] {
            assert!(html.contains("<title>My API</title>"));
            assert!(html.contains(r#"href="/openapi.json""#));
            assert!(!html.contains("<script"));
            assert!(!html.contains("cdn.jsdelivr.net"));
            assert!(!html.contains("unpkg.com"));
            assert!(!html.contains("https://"));
            assert!(!html.contains("http://"));
        }
    }

    #[test]
    fn test_reference_page_escapes_title_and_spec_url() {
        let html = offline_reference_html(
            "<script>alert(1)</script>",
            "\"/onmouseover=alert(1)",
            "Scalar layout",
        );
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(!html.contains(r#""/onmouseover"#));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&quot;"));
    }
}
