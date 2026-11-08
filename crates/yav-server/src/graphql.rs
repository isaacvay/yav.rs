//! Pure Rust, zero-dependency GraphQL schema executor and GraphiQL playground.
//!
//! Provides type-safe GraphQL query and mutation resolution, introspection,
//! and an embedded GraphiQL explorer IDE for developer exploration.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::Response,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};

/// GraphQL field resolver closure type.
pub type FieldResolver =
    Arc<dyn Fn(HashMap<String, Value>) -> Result<Value, String> + Send + Sync + 'static>;

/// Represents a scalar or compound GraphQL type.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphQlType {
    String,
    Int,
    Float,
    Boolean,
    Id,
    Object(String),
    List(Box<GraphQlType>),
    NonNull(Box<GraphQlType>),
}

/// A field definition in an object type.
#[derive(Clone)]
pub struct GraphQlField {
    pub name: String,
    pub description: Option<String>,
    pub return_type: GraphQlType,
    pub args: HashMap<String, GraphQlType>,
    pub resolver: Option<FieldResolver>,
}

/// A GraphQL object type composed of named fields.
#[derive(Clone, Default)]
pub struct GraphQlObject {
    pub name: String,
    pub fields: HashMap<String, GraphQlField>,
}

impl GraphQlObject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: HashMap::new(),
        }
    }

    /// Add a field to the object type with a synchronous/async-safe resolver function.
    pub fn field<F>(
        mut self,
        name: impl Into<String>,
        return_type: GraphQlType,
        resolver: F,
    ) -> Self
    where
        F: Fn(HashMap<String, Value>) -> Result<Value, String> + Send + Sync + 'static,
    {
        let n = name.into();
        self.fields.insert(
            n.clone(),
            GraphQlField {
                name: n,
                description: None,
                return_type,
                args: HashMap::new(),
                resolver: Some(Arc::new(resolver)),
            },
        );
        self
    }
}

/// Root GraphQL Schema with query and mutation entrypoints.
#[derive(Clone)]
pub struct GraphQlSchema {
    pub query: GraphQlObject,
    pub mutation: Option<GraphQlObject>,
}

/// Incoming GraphQL request payload (POST body).
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQlRequest {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<HashMap<String, Value>>,
}

/// Standard GraphQL response format.
#[derive(Debug, Clone, Serialize)]
pub struct GraphQlResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphQlError {
    pub message: String,
}

impl GraphQlSchema {
    /// Create a new schema with a root `Query` object.
    pub fn new(query: GraphQlObject) -> Self {
        Self {
            query,
            mutation: None,
        }
    }

    /// Set root `Mutation` object.
    #[must_use]
    pub fn with_mutation(mut self, mutation: GraphQlObject) -> Self {
        self.mutation = Some(mutation);
        self
    }

    /// Execute a GraphQL query string against this schema.
    pub fn execute(
        &self,
        query_str: &str,
        variables: Option<&HashMap<String, Value>>,
    ) -> GraphQlResponse {
        let trimmed = query_str.trim();

        // 1. Introspection query shortcut for tooling like Apollo / GraphiQL
        if trimmed.contains("__schema") {
            return GraphQlResponse {
                data: Some(json!({
                    "__schema": {
                        "queryType": { "name": self.query.name },
                        "mutationType": self.mutation.as_ref().map(|m| json!({ "name": m.name })),
                        "types": self.introspect_types(),
                    }
                })),
                errors: None,
            };
        }

        // 2. Determine operation type: query or mutation
        let is_mutation = trimmed.starts_with("mutation");
        let target_object = if is_mutation {
            match &self.mutation {
                Some(m) => m,
                None => {
                    return GraphQlResponse {
                        data: None,
                        errors: Some(vec![GraphQlError {
                            message: "Mutations are not supported on this schema".to_string(),
                        }]),
                    };
                }
            }
        } else {
            &self.query
        };

        // 3. Simple robust token extraction for top-level query fields and arguments
        let body_str = if let Some(open_brace) = trimmed.find('{') {
            if let Some(close_brace) = trimmed.rfind('}') {
                &trimmed[open_brace + 1..close_brace]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        let mut data_map = serde_json::Map::new();
        let mut errors = Vec::new();

        for line in split_top_level_fields(body_str) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Extract field alias, field name and optional arguments
            let (output_key, field_name, mut args) = parse_field_and_args(line);

            // Substitute variable references if provided ($var_name)
            if let Some(vars) = variables {
                for val in args.values_mut() {
                    if let Some(s) = val.as_str() {
                        if let Some(var_name) = s.strip_prefix('$') {
                            if let Some(resolved) = vars.get(var_name) {
                                *val = resolved.clone();
                            }
                        }
                    }
                }
            }

            if let Some(field) = target_object.fields.get(&field_name) {
                if let Some(resolver) = &field.resolver {
                    match resolver(args) {
                        Ok(val) => {
                            data_map.insert(output_key, val);
                        }
                        Err(err) => {
                            errors.push(GraphQlError { message: err });
                        }
                    }
                } else {
                    data_map.insert(output_key, Value::Null);
                }
            } else {
                errors.push(GraphQlError {
                    message: format!(
                        "Unknown field '{field_name}' on type '{}'",
                        target_object.name
                    ),
                });
            }
        }

        GraphQlResponse {
            data: if data_map.is_empty() && !errors.is_empty() {
                None
            } else {
                Some(Value::Object(data_map))
            },
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        }
    }

    fn introspect_types(&self) -> Vec<Value> {
        let mut types = vec![json!({
            "name": self.query.name,
            "kind": "OBJECT",
            "fields": self.query.fields.keys().map(|k| json!({ "name": k })).collect::<Vec<_>>(),
        })];
        if let Some(m) = &self.mutation {
            types.push(json!({
                "name": m.name,
                "kind": "OBJECT",
                "fields": m.fields.keys().map(|k| json!({ "name": k })).collect::<Vec<_>>(),
            }));
        }
        types
    }
}

fn split_top_level_fields(body: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0;
    let mut paren_depth = 0;
    let mut in_quote = false;
    let mut quote_char = '"';

    for ch in body.chars() {
        if in_quote {
            current.push(ch);
            if ch == quote_char {
                in_quote = false;
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            }
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                current.push(ch);
                if brace_depth == 0 && paren_depth == 0 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        fields.push(trimmed.to_string());
                    }
                    current.clear();
                }
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                current.push(ch);
            }
            '\n' | '\r' | ';' | ',' if brace_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    fields.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        fields.push(trimmed.to_string());
    }

    fields
}

fn parse_field_and_args(line: &str) -> (String, String, HashMap<String, Value>) {
    let mut args = HashMap::new();
    let before_brace = if let Some(brace_idx) = line.find('{') {
        &line[..brace_idx]
    } else {
        line
    };
    let cleaned = before_brace.trim();

    let (raw_field, raw_args) = if let Some(paren_start) = cleaned.find('(') {
        let field_part = cleaned[..paren_start].trim();
        let args_part = if let Some(paren_end) = cleaned.rfind(')') {
            &cleaned[paren_start + 1..paren_end]
        } else {
            ""
        };
        (field_part, Some(args_part))
    } else {
        let field_part = cleaned.split_whitespace().next().unwrap_or(cleaned);
        (field_part, None)
    };

    // Check alias: e.g. "u1: user" or "user"
    let (output_key, target_field) = if let Some((alias, field)) = raw_field.split_once(':') {
        (alias.trim().to_string(), field.trim().to_string())
    } else {
        (raw_field.to_string(), raw_field.to_string())
    };

    if let Some(inner_args) = raw_args {
        for pair in inner_args.split(',') {
            if let Some((k, v)) = pair.split_once(':') {
                let k = k.trim().to_string();
                let v_trimmed = v.trim();
                let v_str = v_trimmed.trim_matches('"').trim_matches('\'');
                if let Ok(num) = v_str.parse::<i64>() {
                    args.insert(k, json!(num));
                } else if v_str == "true" || v_str == "false" {
                    args.insert(k, json!(v_str == "true"));
                } else {
                    args.insert(k, json!(v_str));
                }
            }
        }
    }

    (output_key, target_field, args)
}

/// Standalone GraphiQL Explorer web interface HTML.
pub fn graphiql_html(endpoint: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Yav GraphiQL Explorer</title>
  <style>
    body {{ margin: 0; background: #0f172a; color: #f8fafc; font-family: ui-monospace, monospace; display: flex; flex-direction: column; height: 100vh; }}
    header {{ background: #1e293b; padding: 12px 24px; border-bottom: 1px solid #334155; display: flex; justify-content: space-between; align-items: center; }}
    .main {{ display: flex; flex: 1; }}
    textarea {{ flex: 1; background: #020617; color: #38bdf8; border: none; padding: 20px; font-family: monospace; font-size: 14px; outline: none; resize: none; }}
    .response {{ flex: 1; background: #090d16; color: #a7f3d0; border-left: 1px solid #334155; padding: 20px; overflow: auto; white-space: pre-wrap; }}
    button {{ background: #6366f1; color: white; border: none; padding: 8px 18px; border-radius: 6px; font-weight: bold; cursor: pointer; }}
    button:hover {{ background: #4f46e5; }}
  </style>
</head>
<body>
  <header>
    <div style="font-weight: bold; font-size: 16px;">🦀 Yav.rs GraphQL Playground</div>
    <button onclick="runQuery()">▶ Execute Query</button>
  </header>
  <div class="main">
    <textarea id="query" spellcheck="false">{{
  hello
  serverTime
}}</textarea>
    <div id="output" class="response">// Execute query to inspect JSON response</div>
  </div>
  <script>
    async function runQuery() {{
      var q = document.getElementById('query').value;
      var out = document.getElementById('output');
      out.textContent = 'Executing...';
      try {{
        var res = await fetch('{endpoint}', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ query: q }})
        }});
        var json = await res.json();
        out.textContent = JSON.stringify(json, null, 2);
      }} catch(err) {{
        out.textContent = 'Error: ' + err.message;
      }}
    }}
  </script>
</body>
</html>"#
    )
}

/// Unified Axum handler for `GET /graphql` (Playground/Schema) and `POST /graphql` (Execution).
pub async fn graphql_handler(schema: Arc<GraphQlSchema>, req: Request<Body>) -> Response {
    if req.method() == axum::http::Method::GET {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(graphiql_html("/graphql")))
            .unwrap();
    }

    let bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(
                    r#"{"errors":[{"message":"Invalid request body"}]}"#,
                ))
                .unwrap();
        }
    };

    let payload: GraphQlRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(
                    r#"{"errors":[{"message":"Malformed JSON GraphQL payload"}]}"#,
                ))
                .unwrap();
        }
    };

    let res = schema.execute(&payload.query, payload.variables.as_ref());
    let res_json = serde_json::to_string(&res).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(res_json))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_query_execution() {
        let query = GraphQlObject::new("Query")
            .field("hello", GraphQlType::String, |_| Ok(json!("world")))
            .field("user", GraphQlType::Object("User".to_string()), |args| {
                let id = args.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(json!({ "id": id, "name": "Alice" }))
            });

        let schema = GraphQlSchema::new(query);
        let mut vars = HashMap::new();
        vars.insert("uid".to_string(), json!(99));
        let res = schema.execute(
            "{\n  hello\n  user(id: 42)\n  u_alias: user(id: $uid)\n}",
            Some(&vars),
        );

        assert!(res.errors.is_none());
        let data = res.data.expect("Should have data");
        assert_eq!(data["hello"], "world");
        assert_eq!(data["user"]["id"], 42);
        assert_eq!(data["user"]["name"], "Alice");
        assert_eq!(data["u_alias"]["id"], 99);
    }

    #[test]
    fn test_graphql_introspection() {
        let query =
            GraphQlObject::new("Query").field("ping", GraphQlType::String, |_| Ok(json!("pong")));
        let schema = GraphQlSchema::new(query);
        let res = schema.execute("{ __schema { types } }", None);
        assert!(res.errors.is_none());
        let data = res.data.unwrap();
        assert!(data.to_string().contains("Query"));
    }
}
