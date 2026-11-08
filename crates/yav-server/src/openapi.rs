//! OpenAPI 3.1 schema generator and embedded interactive API documentation viewers.
//!
//! Generates compliant OpenAPI JSON descriptions for hybrid full-stack applications
//! and serves ready-to-test Scalar and Swagger UI documentation interfaces.

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

/// Render a self-contained modern Scalar API Reference documentation page.
pub fn scalar_ui_handler(title: &str, spec_url: &str) -> Response {
    let escaped_title = yav_core::escape_html(title);
    let escaped_spec_url = yav_core::escape_attribute(spec_url);
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <title>{escaped_title}</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
  <script
    id="api-reference"
    data-url="{escaped_spec_url}"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(html),
    )
        .into_response()
}

/// Render a classic Swagger UI documentation page.
pub fn swagger_ui_handler(title: &str, spec_url: &str) -> Response {
    let escaped_title = yav_core::escape_html(title);
    let spec_url_json = serde_json::to_string(spec_url).unwrap_or_else(|_| "\"\"".to_string());
    let safe_spec_url_json = spec_url_json.replace("</", "<\\/");
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <title>{escaped_title}</title>
  <meta charset="utf-8" />
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({{
      url: {safe_spec_url_json},
      dom_id: '#swagger-ui',
    }});
  </script>
</body>
</html>"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(html),
    )
        .into_response()
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
}
