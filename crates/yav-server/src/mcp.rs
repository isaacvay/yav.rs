//! Native Model Context Protocol (MCP) Server (JSON-RPC 2.0).
//!
//! Exposes a Yav application's tools ([`crate::ai::ToolRegistry`]), data resources,
//! and prompt templates as a standard MCP server (`2024-11-05` specification) so AI
//! agents (Claude Desktop, Cursor, Antigravity, custom LLM orchestrators) can inspect
//! and interact with the application over HTTP (`POST /_yav/mcp`).

use crate::ai::ToolRegistry;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// MCP Resource descriptor (`resources/list` & `resources/read`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Unique resource URI (e.g. `"yav://schema/database"`, `"yav://docs/routes"`).
    pub uri: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// MIME type (e.g. `"application/json"`, `"text/markdown"`).
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// MCP Prompt template descriptor (`prompts/list` & `prompts/get`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    /// Prompt identifier.
    pub name: String,
    /// Prompt description.
    pub description: String,
    /// Prompt template body with `{arg}` placeholders.
    #[serde(skip)]
    pub template: String,
}

/// JSON-RPC 2.0 incoming request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 outgoing response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

type ResourceReaderFn = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

/// Model Context Protocol (MCP) server registry and JSON-RPC dispatcher.
#[derive(Clone, Default)]
pub struct McpServer {
    pub name: String,
    pub version: String,
    pub tools: ToolRegistry,
    resources: Arc<RwLock<Vec<(McpResource, ResourceReaderFn)>>>,
    prompts: Arc<RwLock<HashMap<String, McpPrompt>>>,
}

impl McpServer {
    /// Create a new MCP server with the given server name and version.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: ToolRegistry::new(),
            resources: Arc::new(RwLock::new(Vec::new())),
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Attach an existing [`ToolRegistry`] to this MCP server.
    #[must_use]
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    /// Register a readable MCP resource (`resources/list` & `resources/read`).
    pub fn register_resource<F>(&self, resource: McpResource, reader: F)
    where
        F: Fn(&str) -> Result<String, String> + Send + Sync + 'static,
    {
        if let Ok(mut lock) = self.resources.write() {
            lock.push((resource, Arc::new(reader)));
        }
    }

    /// Register a reusable MCP prompt template (`prompts/list` & `prompts/get`).
    pub fn register_prompt(&self, prompt: McpPrompt) {
        if let Ok(mut lock) = self.prompts.write() {
            lock.insert(prompt.name.clone(), prompt);
        }
    }

    /// Handle a single JSON-RPC 2.0 MCP request and return the corresponding response.
    #[must_use]
    pub fn handle_rpc(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone().unwrap_or(Value::Null);
        let params = req.params.clone().unwrap_or_else(|| json!({}));

        match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": self.name,
                        "version": self.version
                    },
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    }
                })),
                error: None,
            },
            "tools/list" => {
                let specs = self.tools.list_tools();
                let mcp_tools: Vec<Value> = specs
                    .into_iter()
                    .map(|t| {
                        json!({
                            "name": t["name"],
                            "description": t["description"],
                            "inputSchema": t["parameters_schema"]
                        })
                    })
                    .collect();
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "tools": mcp_tools })),
                    error: None,
                }
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match self.tools.invoke(tool_name, args) {
                    Ok(output) => {
                        let text = if let Some(s) = output.as_str() {
                            s.to_string()
                        } else {
                            output.to_string()
                        };
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "content": [{ "type": "text", "text": text }],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    Err(err_msg) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "content": [{ "type": "text", "text": err_msg }],
                            "isError": true
                        })),
                        error: None,
                    },
                }
            }
            "resources/list" => {
                let list: Vec<McpResource> = self
                    .resources
                    .read()
                    .map(|r| r.iter().map(|(res, _)| res.clone()).collect())
                    .unwrap_or_default();
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "resources": list })),
                    error: None,
                }
            }
            "resources/read" => {
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                let found = self.resources.read().ok().and_then(|list| {
                    list.iter()
                        .find(|(r, _)| r.uri == uri)
                        .map(|(r, f)| (r.clone(), f.clone()))
                });
                match found {
                    Some((res, reader)) => match reader(uri) {
                        Ok(text) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "contents": [{
                                    "uri": res.uri,
                                    "mimeType": res.mime_type,
                                    "text": text
                                }]
                            })),
                            error: None,
                        },
                        Err(e) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: e,
                            }),
                        },
                    },
                    None => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: format!("Resource not found: {uri}"),
                        }),
                    },
                }
            }
            "prompts/list" => {
                let list: Vec<Value> = self
                    .prompts
                    .read()
                    .map(|p| {
                        p.values()
                            .map(|pr| {
                                json!({
                                    "name": pr.name,
                                    "description": pr.description
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "prompts": list })),
                    error: None,
                }
            }
            "prompts/get" => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").and_then(|v| v.as_object());
                let prompt_opt = self.prompts.read().ok().and_then(|m| m.get(name).cloned());

                match prompt_opt {
                    Some(pr) => {
                        let mut rendered = pr.template.clone();
                        if let Some(arg_map) = args {
                            for (k, v) in arg_map {
                                let val_str = v.as_str().unwrap_or("");
                                rendered = rendered.replace(&format!("{{{k}}}"), val_str);
                            }
                        }
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "description": pr.description,
                                "messages": [{
                                    "role": "user",
                                    "content": { "type": "text", "text": rendered }
                                }]
                            })),
                            error: None,
                        }
                    }
                    None => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: format!("Prompt not found: {name}"),
                        }),
                    },
                }
            }
            "notifications/initialized" | "ping" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({})),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                }),
            },
        }
    }

    /// Create an Axum HTTP `POST` handler for mounting this MCP server (e.g. at `/_yav/mcp`).
    pub fn into_axum_handler(
        self,
    ) -> impl Fn(
        axum::extract::Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
           + Clone
           + Send
           + Sync
           + 'static {
        let server = Arc::new(self);
        move |req: axum::extract::Request| {
            let srv = server.clone();
            Box::pin(async move {
                let bytes = match axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024).await {
                    Ok(b) => b,
                    Err(_) => return StatusCode::BAD_REQUEST.into_response(),
                };
                match serde_json::from_slice::<JsonRpcRequest>(&bytes) {
                    Ok(rpc_req) => {
                        let resp = srv.handle_rpc(&rpc_req);
                        Json(resp).into_response()
                    }
                    Err(_) => StatusCode::BAD_REQUEST.into_response(),
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_initialize_tools_and_resources() {
        let mut tools = ToolRegistry::new();
        tools.register(
            "get_order_status",
            "Fetch order status by ID",
            json!({
                "type": "object",
                "properties": { "order_id": { "type": "string" } }
            }),
            |args| {
                let id = args.get("order_id").and_then(|v| v.as_str()).unwrap_or("0");
                Ok(json!(format!("Order #{id} is SHIPPED")))
            },
        );

        let mcp = McpServer::new("yav-commerce-mcp", "1.0.0").with_tools(tools);
        mcp.register_resource(
            McpResource {
                uri: "yav://config/store".to_string(),
                name: "Store Configuration".to_string(),
                description: Some("Active store settings".to_string()),
                mime_type: "application/json".to_string(),
            },
            |_uri| Ok(r#"{"currency":"EUR","tax_rate":0.20}"#.to_string()),
        );

        // 1. Initialize
        let init_res = mcp.handle_rpc(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        });
        assert_eq!(
            init_res.result.unwrap()["serverInfo"]["name"],
            "yav-commerce-mcp"
        );

        // 2. Call tool
        let call_res = mcp.handle_rpc(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "get_order_status",
                "arguments": { "order_id": "9941" }
            })),
        });
        let text = call_res.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(text, "Order #9941 is SHIPPED");

        // 3. Read resource
        let res_read = mcp.handle_rpc(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": "yav://config/store" })),
        });
        assert!(res_read.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("EUR"));
    }
}
