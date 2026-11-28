//! Lightweight GraphQL schema executor and a self-contained playground page.
//!
//! Provides a minimal but functional GraphQL query and mutation resolution engine
//! with top-level field selection, aliasing, variable substitution, and introspection.
//!
//! ## Capabilities
//! - Top-level query and mutation field resolution with typed resolvers.
//! - Field aliasing (`alias: fieldName`).
//! - Variable substitution (`$varName`).
//! - Basic introspection (`__schema { types }`).
//! - A small hand-written HTML playground page (not GraphiQL 3.x, no CDN
//!   bundles, no external JavaScript).
//!
//! ## Limitations
//! This is a lightweight engine designed for simple schemas. It does **not** support:
//! - Nested selection sets (field sub-selections).
//! - GraphQL fragments or directives.
//! - Full type validation.
//!
//! For complex schemas, consider using `async-graphql` alongside your Yav app.
//! The `GraphQlSchema` builder API is designed to be compatible with a future
//! migration to a full GraphQL engine.

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

    /// Execute a GraphQL query string against this schema (supporting nested field projection,
    /// aliases, `$variables`, named `fragment` spreads, `@include`/`@skip` directives, and introspection).
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

        // Extract top-level `fragment Name on Type { ... }` blocks
        let mut fragments: HashMap<String, String> = HashMap::new();
        let mut op_query = String::new();
        let mut rest = trimmed;
        while let Some(frag_pos) = rest.find("fragment ") {
            op_query.push_str(&rest[..frag_pos]);
            let after_kw = rest[frag_pos + "fragment ".len()..].trim_start();
            if let Some(brace_open) = after_kw.find('{') {
                let header = after_kw[..brace_open].trim();
                let frag_name = header.split_whitespace().next().unwrap_or("").to_string();
                let mut depth = 0usize;
                let mut end_brace = None;
                for (i, ch) in after_kw[brace_open..].char_indices() {
                    match ch {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end_brace = Some(brace_open + i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(end_idx) = end_brace {
                    if !frag_name.is_empty() {
                        fragments.insert(
                            frag_name,
                            after_kw[brace_open + 1..end_idx].trim().to_string(),
                        );
                    }
                    rest = &after_kw[end_idx + 1..];
                    continue;
                }
            }
            op_query.push_str(&rest[frag_pos..]);
            rest = "";
            break;
        }
        op_query.push_str(rest);
        let op_trimmed = op_query.trim();

        // 2. Determine operation type: query or mutation
        let is_mutation = op_trimmed.starts_with("mutation");
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

        // 3. Extract top-level operation body `{ ... }` and expand fragments
        let body_str = if let Some(open_brace) = op_trimmed.find('{') {
            if let Some(close_brace) = op_trimmed.rfind('}') {
                &op_trimmed[open_brace + 1..close_brace]
            } else {
                op_trimmed
            }
        } else {
            op_trimmed
        };
        let expanded_body = expand_fragments(body_str, &fragments);

        let mut data_map = serde_json::Map::new();
        let mut errors = Vec::new();

        for line in split_top_level_fields(&expanded_body) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if !should_include_field(line, variables) {
                continue;
            }

            // Extract field alias, field name, arguments and optional nested `{ ... }` selection
            let (output_key, field_name, mut args, nested_body) = parse_field_full(line);

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
                            let projected = if let Some(nb) = nested_body {
                                project_selection_set(val, nb, variables, &fragments)
                            } else {
                                val
                            };
                            data_map.insert(output_key, projected);
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

fn split_top_level_args(args_str: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;

    for (i, &b) in args_str.as_bytes().iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(q) = quote {
            if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'"' | b'\'' => quote = Some(b),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                let s = args_str[start..i].trim();
                if !s.is_empty() {
                    parts.push(s);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = args_str[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn expand_fragments(body: &str, fragments: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        if body[i..].starts_with("...") {
            let after = &body[i + 3..];
            let trimmed = after.trim_start();
            // Skip inline fragment syntax if not named
            if !trimmed.starts_with("on ") && !trimmed.starts_with('{') {
                let name_len = trimmed
                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(trimmed.len());
                if name_len > 0 {
                    let frag_name = &trimmed[..name_len];
                    if let Some(frag_body) = fragments.get(frag_name) {
                        out.push_str(frag_body);
                        let consumed = (after.len() - trimmed.len()) + name_len;
                        i += 3 + consumed;
                        continue;
                    }
                }
            }
        }
        let ch = body[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn project_selection_set(
    val: Value,
    selection_body: &str,
    variables: Option<&HashMap<String, Value>>,
    fragments: &HashMap<String, String>,
) -> Value {
    match val {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| project_selection_set(item, selection_body, variables, fragments))
                .collect(),
        ),
        Value::Object(obj) => {
            let expanded = expand_fragments(selection_body, fragments);
            let mut projected = serde_json::Map::new();
            for sub in split_top_level_fields(&expanded) {
                let sub = sub.trim();
                if sub.is_empty() || sub.starts_with('#') {
                    continue;
                }
                if !should_include_field(sub, variables) {
                    continue;
                }
                let (out_key, target_field, _, nested_body) = parse_field_full(sub);
                if let Some(child_val) = obj.get(&target_field).cloned() {
                    let final_val = if let Some(nb) = nested_body {
                        project_selection_set(child_val, nb, variables, fragments)
                    } else {
                        child_val
                    };
                    projected.insert(out_key, final_val);
                }
            }
            Value::Object(projected)
        }
        other => other,
    }
}

fn should_include_field(line: &str, variables: Option<&HashMap<String, Value>>) -> bool {
    let header = line.split('{').next().unwrap_or(line);
    if let Some(pos) = header.find("@include(") {
        if let Some(end) = header[pos + 9..].find(')') {
            let expr = header[pos + 9..pos + 9 + end].trim();
            if let Some(val_str) = expr.strip_prefix("if:").map(str::trim) {
                return resolve_bool_directive(val_str, variables, true);
            }
        }
    }
    if let Some(pos) = header.find("@skip(") {
        if let Some(end) = header[pos + 6..].find(')') {
            let expr = header[pos + 6..pos + 6 + end].trim();
            if let Some(val_str) = expr.strip_prefix("if:").map(str::trim) {
                return !resolve_bool_directive(val_str, variables, false);
            }
        }
    }
    true
}

fn resolve_bool_directive(
    token: &str,
    variables: Option<&HashMap<String, Value>>,
    default: bool,
) -> bool {
    if let Some(var_name) = token.strip_prefix('$') {
        return variables
            .and_then(|v| v.get(var_name))
            .and_then(|v| v.as_bool())
            .unwrap_or(default);
    }
    match token {
        "true" => true,
        "false" => false,
        _ => default,
    }
}

fn parse_field_full(line: &str) -> (String, String, HashMap<String, Value>, Option<&str>) {
    let nested_body = line.find('{').and_then(|open| {
        line.rfind('}')
            .filter(|&close| close > open)
            .map(|close| &line[open + 1..close])
    });
    let (out_key, field, args) = parse_field_and_args(line);
    (out_key, field, args, nested_body)
}

fn parse_field_and_args(line: &str) -> (String, String, HashMap<String, Value>) {
    let mut args = HashMap::new();
    let before_brace = if let Some(brace_idx) = line.find('{') {
        &line[..brace_idx]
    } else {
        line
    };
    // Strip trailing directives (`@include(...)`, `@skip(...)`) before parsing field/args
    let without_directives = if let Some(at_idx) = before_brace.find('@') {
        before_brace[..at_idx].trim()
    } else {
        before_brace.trim()
    };

    let (raw_field, raw_args) = if let Some(paren_start) = without_directives.find('(') {
        let field_part = without_directives[..paren_start].trim();
        let args_part = if let Some(paren_end) = without_directives.rfind(')') {
            &without_directives[paren_start + 1..paren_end]
        } else {
            ""
        };
        (field_part, Some(args_part))
    } else {
        let field_part = without_directives
            .split_whitespace()
            .next()
            .unwrap_or(without_directives);
        (field_part, None)
    };

    // Check alias: e.g. "u1: user" or "user"
    let (output_key, target_field) = if let Some((alias, field)) = raw_field.split_once(':') {
        (alias.trim().to_string(), field.trim().to_string())
    } else {
        (raw_field.to_string(), raw_field.to_string())
    };

    if let Some(inner_args) = raw_args {
        for pair in split_top_level_args(inner_args) {
            if let Some((k, v)) = pair.split_once(':') {
                let k = k.trim().to_string();
                let v_trimmed = v.trim();
                let is_quoted = (v_trimmed.starts_with('"') && v_trimmed.ends_with('"'))
                    || (v_trimmed.starts_with('\'') && v_trimmed.ends_with('\''));
                let v_str = if is_quoted && v_trimmed.len() >= 2 {
                    &v_trimmed[1..v_trimmed.len() - 1]
                } else {
                    v_trimmed
                };
                if is_quoted {
                    args.insert(k, json!(v_str));
                } else if let Ok(num) = v_str.parse::<i64>() {
                    args.insert(k, json!(num));
                } else if let Ok(num) = v_str.parse::<f64>() {
                    args.insert(k, json!(num));
                } else if v_str == "true" || v_str == "false" {
                    args.insert(k, json!(v_str == "true"));
                } else if v_str == "null" {
                    args.insert(k, Value::Null);
                } else {
                    args.insert(k, json!(v_str));
                }
            }
        }
    }

    (output_key, target_field, args)
}

/// Self-contained playground page for the given GraphQL endpoint.
///
/// This is a small hand-written HTML page, not the GraphiQL 3.x IDE. The
/// endpoint is serialised as a JS/JSON string literal (with `<`, `>`, `&`,
/// `/` and U+2028/U+2029 escaped), so a caller-controlled endpoint cannot
/// break out of the inline `<script>` element.
pub fn graphiql_html(endpoint: &str) -> String {
    let endpoint_js = crate::streaming::escape_js_string_literal(endpoint);
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
    <button id="run-btn">▶ Execute Query</button>
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
        var res = await fetch({endpoint_js}, {{
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
    document.getElementById('run-btn').addEventListener('click', runQuery);
  </script>
</body>
</html>"#
    )
}

/// Unified Axum handler for `GET /graphql` (Playground/Schema) and `POST /graphql` (Execution).
pub async fn graphql_handler(schema: Arc<GraphQlSchema>, req: Request<Body>) -> Response {
    graphql_handler_at(schema, "/graphql", req).await
}

/// Unified Axum handler mounted at a custom `endpoint` path.
pub async fn graphql_handler_at(
    schema: Arc<GraphQlSchema>,
    endpoint: &str,
    req: Request<Body>,
) -> Response {
    if req.method() == axum::http::Method::GET {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(graphiql_html(endpoint)))
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    let bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(
                    r#"{"errors":[{"message":"Invalid request body"}]}"#,
                ))
                .unwrap_or_else(|_| Response::new(Body::empty()));
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
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }
    };

    let res = schema.execute(&payload.query, payload.variables.as_ref());
    let res_json = serde_json::to_string(&res).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(res_json))
        .unwrap_or_else(|_| Response::new(Body::empty()))
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

    #[test]
    fn test_playground_escapes_endpoint_against_script_breakout() {
        let html = graphiql_html("/graphql");
        assert!(html.contains("fetch(\"\\/graphql\""));
        assert!(!html.contains("cdn."));
        assert!(!html.contains("<script src="));

        let hostile = "</script><script>alert(1)</script>'; fetch('https://evil.example/?";
        let html = graphiql_html(hostile);
        assert!(!html.contains("</script><script>alert(1)"));
        assert!(!html.contains("<script>alert(1)"));
        // The whole endpoint is a single JSON string literal, so the injected
        // quotes and tags are inert data rather than executable JavaScript.
        assert!(html.contains("\\u003c\\/script\\u003e"));
        assert_eq!(html.matches("<script>").count(), 1);
    }
}
