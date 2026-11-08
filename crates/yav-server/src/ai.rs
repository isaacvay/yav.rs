//! Generative AI, Streaming UI (`streamUI`), Tool Calling & Hybrid RAG Vector Search (`yav::ai` — Vercel AI SDK equivalent).
//!
//! Provides first-class AI engineering primitives for Rust web applications:
//! - [`Conversation`] & [`ChatMessage`]: Structured multi-turn prompt management (`System`, `User`, `Assistant`, `Tool`).
//! - [`ToolRegistry`] & [`AiTool`]: Typed tool/function definitions with JSON Schema generation and dispatch.
//! - [`stream_ui_fragment`] & [`stream_text_sse`]: Server-Sent Events streaming of both LLM text tokens and live RSX [`Node`] UI components (`streamUI`).
//! - [`VectorIndex`] & [`reciprocal_rank_fusion`]: Dense cosine-similarity vector store and hybrid BM25 + Vector RAG fusion.

use crate::sse::{sse_channel, sse_response, SseMsg};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use yav_core::{render_to_string, Node};

// ─────────────────────────────────────────────────────────────────────────────
// 1. Chat Messages & Conversation History
// ─────────────────────────────────────────────────────────────────────────────

/// Role of a participant in an AI conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a multi-turn LLM conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_call_id: None,
        }
    }
}

/// Manages a multi-turn LLM conversation with token budget estimation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system(prompt: impl Into<String>) -> Self {
        Self {
            messages: vec![ChatMessage::system(prompt)],
        }
    }

    pub fn push(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    /// Estimate token usage (~4 characters per token heuristic).
    pub fn estimated_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.content.len().div_ceil(4) + 4)
            .sum()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. AI Tool Calling Registry (`ToolRegistry`)
// ─────────────────────────────────────────────────────────────────────────────

type ToolHandlerFn =
    Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;

/// Metadata and execution handler for an LLM-callable tool.
#[derive(Clone)]
pub struct AiTool {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    handler: ToolHandlerFn,
}

/// Registry of typed functions callable by an AI model or agent.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, AiTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a callable tool with its JSON Schema parameters and handler closure.
    pub fn register<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters_schema: serde_json::Value,
        handler: F,
    ) where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
    {
        let n = name.into();
        self.tools.insert(
            n.clone(),
            AiTool {
                name: n,
                description: description.into(),
                parameters_schema,
                handler: Arc::new(handler),
            },
        );
    }

    /// Execute a registered tool by name with JSON arguments.
    pub fn call(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("Unknown AI tool: '{name}'"))?;
        (tool.handler)(args)
    }

    /// Invoke a tool by name (alias for `call`).
    pub fn invoke(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        self.call(name, args)
    }

    /// Return metadata for all registered tools.
    pub fn list_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters_schema": t.parameters_schema,
                })
            })
            .collect()
    }

    /// Export all registered tool definitions in OpenAI / Anthropic `tools` JSON array format.
    pub fn to_openai_tools_json(&self) -> serde_json::Value {
        let list: Vec<serde_json::Value> = self
            .tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters_schema,
                    }
                })
            })
            .collect();
        serde_json::Value::Array(list)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Generative UI (`streamUI`) & Token Streaming over SSE
// ─────────────────────────────────────────────────────────────────────────────

/// Format an RSX [`Node`] into a Generative UI SSE message (`event: yav:ui`) that swaps
/// live server-rendered UI components into the chat stream (Vercel AI RSC `streamUI` equivalent).
pub fn stream_ui_fragment(target_selector: &str, node: &Node) -> SseMsg {
    let html = render_to_string(node);
    let payload = serde_json::json!({
        "target": target_selector,
        "html": html,
    });
    SseMsg::new(payload.to_string()).event("yav:ui")
}

/// Stream a sequence of text chunks as Server-Sent Events (`event: yav:token`) followed by `event: done`.
pub fn stream_text_sse(chunks: Vec<String>) -> Response {
    let (tx, stream) = sse_channel(64);
    tokio::spawn(async move {
        for chunk in chunks {
            if tx
                .send(SseMsg::new(chunk).event("yav:token"))
                .await
                .is_err()
            {
                break;
            }
        }
        let _ = tx.send(SseMsg::new("[DONE]").event("done")).await;
    });
    sse_response(stream)
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Dense Vector Index & Hybrid RAG (Reciprocal Rank Fusion)
// ─────────────────────────────────────────────────────────────────────────────

/// A vector embedding record stored in [`VectorIndex`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

/// In-memory dense vector store using Cosine Similarity for semantic RAG retrieval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorIndex {
    records: Vec<VectorRecord>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a document embedding vector.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        embedding: Vec<f32>,
        metadata: HashMap<String, String>,
    ) {
        self.records.push(VectorRecord {
            id: id.into(),
            embedding,
            metadata,
        });
    }

    /// Search the top-`k` most semantically similar records by cosine similarity.
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .records
            .iter()
            .map(|rec| {
                let sim = cosine_similarity(query_embedding, &rec.embedding);
                (rec.id.clone(), sim)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k.max(1));
        scored
    }
}

/// Compute cosine similarity between two vectors (`-1.0..=1.0`).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom <= f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

/// Combine lexical BM25 rankings (`SearchIndex`) and semantic vector rankings (`VectorIndex`)
/// using Reciprocal Rank Fusion ($\text{RRF}(d) = \sum \frac{1}{k + \text{rank}(d)}$).
pub fn reciprocal_rank_fusion(
    bm25_doc_ids: &[String],
    vector_doc_ids: &[String],
    k_constant: usize,
) -> Vec<(String, f64)> {
    let k = k_constant.max(1) as f64;
    let mut rrf_scores: HashMap<String, f64> = HashMap::new();

    for (rank, doc_id) in bm25_doc_ids.iter().enumerate() {
        *rrf_scores.entry(doc_id.clone()).or_insert(0.0) += 1.0 / (k + (rank + 1) as f64);
    }
    for (rank, doc_id) in vector_doc_ids.iter().enumerate() {
        *rrf_scores.entry(doc_id.clone()).or_insert(0.0) += 1.0 / (k + (rank + 1) as f64);
    }

    let mut merged: Vec<(String, f64)> = rrf_scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_tools_vector_search_and_rrf_fusion() {
        let mut tools = ToolRegistry::new();
        tools.register(
            "get_weather",
            "Fetch current weather for a city",
            serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}}),
            |args| {
                let city = args.get("city").and_then(|v| v.as_str()).unwrap_or("Paris");
                Ok(serde_json::json!({ "city": city, "temp_c": 22 }))
            },
        );

        let res = tools
            .call("get_weather", serde_json::json!({"city": "Tokyo"}))
            .unwrap();
        assert_eq!(res["city"], "Tokyo");
        assert_eq!(res["temp_c"], 22);

        let mut vec_idx = VectorIndex::new();
        vec_idx.insert("doc_rust", vec![0.9, 0.1, 0.0], HashMap::new());
        vec_idx.insert("doc_cooking", vec![0.0, 0.1, 0.9], HashMap::new());

        let top = vec_idx.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(top[0].0, "doc_rust");

        let fused = reciprocal_rank_fusion(
            &["doc_a".to_string(), "doc_b".to_string()],
            &["doc_b".to_string(), "doc_c".to_string()],
            60,
        );
        // doc_b appears in both lists so it ranks #1
        assert_eq!(fused[0].0, "doc_b");
    }
}
