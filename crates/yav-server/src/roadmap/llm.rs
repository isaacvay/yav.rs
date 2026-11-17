//! Real OpenAI-compatible HTTP LLM client (feature `http-llm`).
//!
//! Talks to `/v1/chat/completions` and `/v1/embeddings` with `reqwest`.
//! There is no embedded model and no fabricated completion or embedding: a
//! missing API key, a non-success HTTP status, or a malformed response is an
//! explicit error. Each request carries a fresh `uuid` request id.

#![cfg(feature = "http-llm")]

use serde::{Deserialize, Serialize};

use crate::adapters::LlmAdapter;
use crate::ai::ChatMessage;

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// HTTP client for an OpenAI-compatible chat and embeddings API.
pub struct HttpLlmAdapter {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl HttpLlmAdapter {
    /// Create a client for `base_url` using `api_key`.
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Result<Self, String> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err("YAV LLM API key must not be empty".to_string());
        }
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err("YAV LLM base URL must not be empty".to_string());
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("failed to build dedicated Tokio runtime: {e}"))?;
        Ok(Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
            runtime,
        })
    }

    /// Build from `YAV_LLM_API_KEY` and `YAV_LLM_BASE_URL`.
    ///
    /// The API key is mandatory; a missing `YAV_LLM_BASE_URL` falls back to
    /// OpenAI's public endpoint.
    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("YAV_LLM_API_KEY")
            .map_err(|_| "YAV_LLM_API_KEY is not set; no LLM request will be sent".to_string())?;
        let base_url =
            std::env::var("YAV_LLM_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::new(api_key, base_url)
    }

    /// The configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn request_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Execute `future` on the adapter's runtime and decode a JSON response.
    fn post_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: String,
        body: serde_json::Value,
    ) -> Result<T, String> {
        let request_id = Self::request_id();
        let api_key = self.api_key.clone();
        let client = self.client.clone();

        let outcome = self.runtime.block_on(async move {
            let response = client
                .post(&url)
                .bearer_auth(&api_key)
                .header("Content-Type", "application/json")
                .header("X-Request-Id", &request_id)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("LLM request to '{url}' failed: {e}"))?;

            let status = response.status();
            let text = response
                .text()
                .await
                .map_err(|e| format!("failed to read LLM response body: {e}"))?;
            if !status.is_success() {
                return Err(format!(
                    "LLM API returned HTTP {status} (request {request_id}): {}",
                    text.chars().take(500).collect::<String>()
                ));
            }
            serde_json::from_str::<T>(&text)
                .map_err(|e| format!("invalid LLM JSON response (request {request_id}): {e}"))
        });

        outcome
    }
}

impl LlmAdapter for HttpLlmAdapter {
    fn complete(&self, model: &str, messages: &[ChatMessage]) -> Result<String, String> {
        let body = serde_json::to_value(ChatRequest { model, messages })
            .map_err(|e| format!("failed to serialize chat request: {e}"))?;
        let response: ChatResponse =
            self.post_json(format!("{}/v1/chat/completions", self.base_url), body)?;
        response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .ok_or_else(|| "LLM response contained no completion choice".to_string())
    }

    fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>, String> {
        let body = serde_json::to_value(EmbeddingRequest { model, input: text })
            .map_err(|e| format!("failed to serialize embedding request: {e}"))?;
        let response: EmbeddingResponse =
            self.post_json(format!("{}/v1/embeddings", self.base_url), body)?;
        response
            .data
            .into_iter()
            .next()
            .map(|entry| entry.embedding)
            .ok_or_else(|| "LLM response contained no embedding".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_requires_key_and_normalizes_url() {
        let err = HttpLlmAdapter::new("", "https://api.openai.com")
            .err()
            .expect("no key");
        assert!(err.contains("API key"));

        let err = HttpLlmAdapter::new("sk-test", "").err().expect("no url");
        assert!(err.contains("base URL"));

        let adapter = HttpLlmAdapter::new("sk-test", "https://llm.local/").unwrap();
        assert_eq!(adapter.base_url(), "https://llm.local");
    }

    #[test]
    fn test_request_payload_serialization() {
        let messages = vec![ChatMessage::user("Hello")];
        let body = serde_json::to_value(ChatRequest {
            model: "gpt-4o",
            messages: &messages,
        })
        .unwrap();
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hello");

        let embed_body = serde_json::to_value(EmbeddingRequest {
            model: "text-embedding-3-small",
            input: "hi",
        })
        .unwrap();
        assert_eq!(embed_body["input"], "hi");
    }
}
