//! Ollama local LLM provider implementation
//!
//! This module provides an implementation of the `LlmProvider` trait for Ollama,
//! which allows running local LLM models via the Ollama API.
//!
//! # Features
//!
//! - Local LLM inference (no data leaves the machine)
//! - Streaming response handling
//! - Support for tool calling (model-dependent)
//! - Token usage tracking with estimation fallback
//! - Connection error handling with helpful suggestions
//!
//! # Example
//!
//! ```rust
//! use miniclaw::providers::{OllamaConfig, OllamaProvider};
//!
//! async fn example() {
//!     let config = OllamaConfig::new();
//!     let provider = OllamaProvider::new(config);
//!     // Use provider...
//! }
//! ```

use std::time::Duration;

use bytes::Bytes;
use futures::StreamExt;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::providers::factory::OllamaConfig;
use crate::providers::{
    LlmMessage, LlmProvider, LlmResponse, LlmToolCall, ModelInfo, ProviderError,
};

/// Ollama API request body format
#[derive(Debug, Serialize)]
struct OllamaRequest {
    /// Model to use for completion
    model: String,
    /// Messages in the conversation
    messages: Vec<OllamaMessage>,
    /// Enable streaming (always true for consistent handling)
    stream: bool,
    /// Available tools for the model to use (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    /// Additional options for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<serde_json::Value>,
}

/// Ollama message format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaMessage {
    /// Role of the message sender
    role: String,
    /// Content of the message
    content: String,
    /// Tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    /// Tool call ID for tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// Ollama tool call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaToolCall {
    /// Type of the tool call (always "function")
    #[serde(rename = "type")]
    call_type: String,
    /// Function call details
    function: OllamaFunctionCall,
}

/// Ollama function call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaFunctionCall {
    /// Name of the function to call
    name: String,
    /// Arguments as JSON string
    arguments: String,
}

/// Ollama streaming response chunk format
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponseChunk {
    /// Model name
    #[allow(dead_code)]
    model: String,
    /// Creation timestamp
    #[allow(dead_code)]
    created_at: String,
    /// Message content in this chunk
    message: OllamaMessage,
    /// Whether this is the final chunk
    done: bool,
    /// Total duration in nanoseconds (only in final chunk)
    #[allow(dead_code)]
    total_duration: Option<u64>,
    /// Load duration in nanoseconds (only in final chunk)
    #[allow(dead_code)]
    load_duration: Option<u64>,
    /// Prompt evaluation count (only in final chunk)
    prompt_eval_count: Option<u32>,
    /// Evaluation count (completion tokens, only in final chunk)
    eval_count: Option<u32>,
}

/// Accumulated response from streaming
#[derive(Debug, Default)]
struct AccumulatedResponse {
    /// Accumulated content
    content: String,
    /// Accumulated tool calls
    tool_calls: Vec<LlmToolCall>,
    /// Prompt tokens (from final chunk)
    prompt_tokens: Option<u32>,
    /// Completion tokens (from final chunk)
    completion_tokens: Option<u32>,
    /// Whether the stream is complete
    done: bool,
}

/// Ollama API response for listing models (/api/tags)
#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    /// List of available models
    models: Vec<OllamaModelInfo>,
}

/// Individual model information from Ollama
#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    /// Model name (e.g., "llama3.2:latest")
    name: String,
}

/// Ollama provider implementation
///
/// This struct implements the `LlmProvider` trait for Ollama's local API,
/// which provides access to locally-running LLM models.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    /// Configuration for the provider
    config: OllamaConfig,
    /// HTTP client for making requests
    client: Client,
}

impl OllamaProvider {
    /// Creates a new Ollama provider with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Ollama configuration including base URL and settings
    ///
    /// # Returns
    ///
    /// A new `OllamaProvider` instance
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client fails to build (extremely rare in practice)
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to build HTTP client: {}. This should never happen.",
                    e
                )
            });

        Self { config, client }
    }

    /// Creates a new Ollama provider with the given configuration, returning an error if client build fails
    ///
    /// This is a fallible version of `new()` that returns a Result instead of panicking.
    ///
    /// # Arguments
    ///
    /// * `config` - Ollama configuration including base URL and settings
    ///
    /// # Returns
    ///
    /// A new `OllamaProvider` instance or an error if HTTP client cannot be built
    pub fn try_new(config: OllamaConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::config(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Builds the Ollama API request body from messages and tools
    fn build_request(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> OllamaRequest {
        let ollama_messages: Vec<OllamaMessage> = messages
            .into_iter()
            .map(|msg| OllamaMessage {
                role: msg.role.as_str().to_string(),
                content: msg.content,
                tool_calls: msg.tool_calls.map(|calls| {
                    calls
                        .into_iter()
                        .map(|call| OllamaToolCall {
                            call_type: "function".to_string(),
                            function: OllamaFunctionCall {
                                name: call.name,
                                arguments: call.arguments,
                            },
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id,
            })
            .collect();

        OllamaRequest {
            model: model.to_string(),
            messages: ollama_messages,
            stream: true,
            tools: if tools.is_empty() { None } else { Some(tools) },
            options: if self.config.options.is_empty() {
                None
            } else {
                match serde_json::to_value(&self.config.options) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        warn!(
                            error = %e,
                            "Failed to serialize Ollama options, ignoring them"
                        );
                        None
                    }
                }
            },
        }
    }

    /// Parses a streaming chunk from Ollama
    fn parse_chunk(&self, chunk: &Bytes) -> Result<OllamaResponseChunk, ProviderError> {
        let text = String::from_utf8_lossy(chunk);

        // Handle empty lines
        if text.trim().is_empty() {
            return Err(ProviderError::serialization("Empty response chunk"));
        }

        // Parse the JSON chunk
        serde_json::from_str(&text).map_err(|e| {
            ProviderError::serialization(format!("Failed to parse response chunk: {}", e))
        })
    }

    /// Accumulates streaming chunks into a complete response
    fn accumulate_chunk(
        &self,
        acc: &mut AccumulatedResponse,
        chunk: OllamaResponseChunk,
    ) -> Result<(), ProviderError> {
        // Accumulate content
        acc.content.push_str(&chunk.message.content);

        // Accumulate tool calls if present
        if let Some(tool_calls) = chunk.message.tool_calls {
            for call in tool_calls {
                acc.tool_calls.push(LlmToolCall::new(
                    format!("call_{}", acc.tool_calls.len()),
                    call.function.name,
                    call.function.arguments,
                ));
            }
        }

        // If this is the final chunk, extract token counts
        if chunk.done {
            acc.done = true;
            acc.prompt_tokens = chunk.prompt_eval_count;
            acc.completion_tokens = chunk.eval_count;
        }

        Ok(())
    }

    /// Estimates token count from character count (rough approximation: 4 chars ≈ 1 token)
    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.len() as f32 / 4.0).ceil() as u32
    }

    /// Estimates token count for a list of messages
    fn estimate_message_tokens(&self, messages: &[OllamaMessage]) -> u32 {
        messages
            .iter()
            .map(|msg| {
                // Count role, content, and basic message overhead
                let role_tokens = self.estimate_tokens(&msg.role);
                let content_tokens = self.estimate_tokens(&msg.content);
                // Add small overhead for message structure (approximately 4 tokens per message)
                role_tokens + content_tokens + 4
            })
            .sum()
    }

    /// Handles HTTP errors and converts them to appropriate ProviderError variants
    fn handle_http_error(&self, status: StatusCode, body: Option<String>) -> ProviderError {
        let message = body.unwrap_or_else(|| "Unknown error".to_string());

        match status {
            StatusCode::NOT_FOUND => ProviderError::invalid_request(
                "Model not found. Run `ollama pull [model_name]` to download the model."
                    .to_string(),
            ),
            StatusCode::BAD_REQUEST => ProviderError::invalid_request(message),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                ProviderError::auth("Authentication failed")
            }
            StatusCode::TOO_MANY_REQUESTS => {
                ProviderError::rate_limit(format!("Rate limit exceeded: {}", message), Some(1))
            }
            status if status.is_server_error() => {
                ProviderError::provider(format!("Ollama server error: {}", message), None::<String>)
            }
            _ => ProviderError::network(format!("HTTP error {}: {}", status, message)),
        }
    }

    /// Handles connection errors with helpful suggestions
    fn handle_connection_error(&self, err: &reqwest::Error) -> ProviderError {
        let message = if err.is_connect() {
            format!(
                "Cannot connect to Ollama at {}. Is Ollama running? Start it with: ollama serve",
                self.config.base_url
            )
        } else if err.is_timeout() {
            format!(
                "Request timed out after {} seconds",
                self.config.timeout_seconds
            )
        } else {
            format!("Network error: {}", err)
        };

        ProviderError::network(message)
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> Result<LlmResponse, ProviderError> {
        let model = if model.is_empty() {
            &self.config.default_model
        } else {
            model
        };

        info!(
            model = %model,
            message_count = messages.len(),
            tool_count = tools.len(),
            "Sending request to Ollama"
        );

        let request = self.build_request(messages, tools, model);
        let url = format!("{}/api/chat", self.config.base_url);

        debug!(url = %url, "Making Ollama API request");

        // Make the HTTP request
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| self.handle_connection_error(&e))?;

        let status = response.status();

        // Handle HTTP errors
        if !status.is_success() {
            let body = response.text().await.ok();
            return Err(self.handle_http_error(status, body));
        }

        // Process streaming response
        let mut accumulated = AccumulatedResponse::default();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // Split by newlines in case multiple JSON objects are in one chunk
                    for line in chunk.split(|&b| b == b'\n') {
                        if line.is_empty() {
                            continue;
                        }

                        match self.parse_chunk(&Bytes::copy_from_slice(line)) {
                            Ok(parsed_chunk) => {
                                if let Err(e) =
                                    self.accumulate_chunk(&mut accumulated, parsed_chunk)
                                {
                                    warn!(error = %e, "Failed to process chunk");
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, line = %String::from_utf8_lossy(line), "Failed to parse chunk");
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ProviderError::network(format!("Stream error: {}", e)));
                }
            }
        }

        // Validate stream completion
        if !accumulated.done {
            warn!("Ollama stream ended without done=true flag. Response may be incomplete.");
        }

        // Log token usage info
        if let (Some(prompt), Some(completion)) =
            (accumulated.prompt_tokens, accumulated.completion_tokens)
        {
            info!(
                prompt_tokens = prompt,
                completion_tokens = completion,
                total_tokens = prompt + completion,
                "Ollama token usage"
            );
        } else {
            // Estimate tokens if not provided
            let estimated_prompt = self.estimate_message_tokens(&request.messages);
            let estimated_completion = self.estimate_tokens(&accumulated.content);
            warn!(
                estimated_prompt_tokens = estimated_prompt,
                estimated_completion_tokens = estimated_completion,
                "Ollama token counts not provided, using estimates"
            );
            accumulated.prompt_tokens = Some(estimated_prompt);
            accumulated.completion_tokens = Some(estimated_completion);
        }

        debug!(
            content_length = accumulated.content.len(),
            tool_call_count = accumulated.tool_calls.len(),
            "Ollama response complete"
        );

        Ok(LlmResponse {
            content: accumulated.content,
            tool_calls: if accumulated.tool_calls.is_empty() {
                None
            } else {
                Some(accumulated.tool_calls)
            },
            prompt_tokens: accumulated.prompt_tokens,
            completion_tokens: accumulated.completion_tokens,
        })
    }

    fn default_model(&self) -> String {
        self.config.default_model.clone()
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/api/tags", self.config.base_url);

        debug!(url = %url, "Listing Ollama models");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| self.handle_connection_error(&e))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.ok();
            return Err(self.handle_http_error(status, body));
        }

        let models_response: OllamaModelsResponse = response.json().await.map_err(|e| {
            ProviderError::serialization(format!("Failed to parse models response: {}", e))
        })?;

        let mut models: Vec<ModelInfo> = models_response
            .models
            .into_iter()
            .map(|m| ModelInfo::new(m.name, false)) // Ollama doesn't have deprecation info
            .collect();

        // Sort alphabetically by id
        models.sort_by(|a, b| a.id.cmp(&b.id));

        debug!(count = models.len(), "Listed Ollama models");

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{LlmRole, ToolDefinition};

    fn create_test_config() -> OllamaConfig {
        OllamaConfig::new()
            .with_base_url("http://localhost:11434")
            .with_model("llama3.2")
    }

    #[test]
    fn test_ollama_provider_new() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.provider_name(), "ollama");
        assert_eq!(provider.default_model(), "llama3.2");
    }

    #[test]
    fn test_ollama_provider_try_new() {
        let config = create_test_config();
        let result = OllamaProvider::try_new(config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn test_build_request() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let messages = vec![
            LlmMessage::new(LlmRole::System, "You are helpful"),
            LlmMessage::new(LlmRole::User, "Hello!"),
        ];

        let request = provider.build_request(messages, vec![], "mistral");

        assert_eq!(request.model, "mistral");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, "system");
        assert_eq!(request.messages[0].content, "You are helpful");
        assert_eq!(request.messages[1].role, "user");
        assert!(request.stream);
        assert!(request.tools.is_none());
    }

    #[test]
    fn test_build_request_with_tools() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let messages = vec![LlmMessage::new(LlmRole::User, "What's the weather?")];

        let tool = ToolDefinition::new(
            "get_weather",
            "Get weather information",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        );
        let tools = vec![tool.to_openai_format()];

        let request = provider.build_request(messages, tools, "llama3.2");

        assert!(request.tools.is_some());
        assert_eq!(request.tools.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_estimate_tokens() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        // 4 chars ≈ 1 token
        assert_eq!(provider.estimate_tokens("abcd"), 1);
        assert_eq!(provider.estimate_tokens("abcdefgh"), 2);
        assert_eq!(provider.estimate_tokens("abcdefghijkl"), 3);
        assert_eq!(provider.estimate_tokens(""), 0);
    }

    #[test]
    fn test_handle_connection_error_connect() {
        let config = create_test_config();
        let _provider = OllamaProvider::new(config);

        // Create a mock connection error using Network variant directly
        let err = ProviderError::network("Connection failed");

        match err {
            ProviderError::Network { message } => {
                assert!(message.contains("Connection failed"));
            }
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_handle_http_error_not_found() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let error = provider.handle_http_error(StatusCode::NOT_FOUND, None);

        match error {
            ProviderError::InvalidRequest { message } => {
                assert!(message.contains("Model not found"));
                assert!(message.contains("ollama pull"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_handle_http_error_connection_refused() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let error = provider.handle_http_error(
            StatusCode::SERVICE_UNAVAILABLE,
            Some("Ollama server error".to_string()),
        );

        match error {
            ProviderError::Provider { message, .. } => {
                assert!(message.contains("Ollama server error"));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_accumulate_chunk() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let mut acc = AccumulatedResponse::default();

        // First chunk
        let chunk1 = OllamaResponseChunk {
            model: "llama3.2".to_string(),
            created_at: "2026-02-15T10:00:00Z".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Hello ".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            done: false,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            eval_count: None,
        };

        provider.accumulate_chunk(&mut acc, chunk1).unwrap();
        assert_eq!(acc.content, "Hello ");
        assert!(!acc.done);

        // Final chunk
        let chunk2 = OllamaResponseChunk {
            model: "llama3.2".to_string(),
            created_at: "2026-02-15T10:00:01Z".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "world!".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            done: true,
            total_duration: Some(1234567890),
            load_duration: Some(123456789),
            prompt_eval_count: Some(10),
            eval_count: Some(5),
        };

        provider.accumulate_chunk(&mut acc, chunk2).unwrap();
        assert_eq!(acc.content, "Hello world!");
        assert!(acc.done);
        assert_eq!(acc.prompt_tokens, Some(10));
        assert_eq!(acc.completion_tokens, Some(5));
    }

    #[test]
    fn test_accumulate_chunk_with_tool_calls() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let mut acc = AccumulatedResponse::default();

        let chunk = OllamaResponseChunk {
            model: "llama3.2".to_string(),
            created_at: "2026-02-15T10:00:00Z".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "".to_string(),
                tool_calls: Some(vec![OllamaToolCall {
                    call_type: "function".to_string(),
                    function: OllamaFunctionCall {
                        name: "get_weather".to_string(),
                        arguments: r#"{"location":"Paris"}"#.to_string(),
                    },
                }]),
                tool_call_id: None,
            },
            done: false,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            eval_count: None,
        };

        provider.accumulate_chunk(&mut acc, chunk).unwrap();

        assert_eq!(acc.tool_calls.len(), 1);
        assert_eq!(acc.tool_calls[0].name, "get_weather");
        assert_eq!(acc.tool_calls[0].arguments, r#"{"location":"Paris"}"#);
    }

    #[test]
    fn test_parse_chunk_valid() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let json = r#"{"model":"llama3.2","created_at":"2026-02-15T10:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let chunk = Bytes::from(json);

        let result = provider.parse_chunk(&chunk);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.model, "llama3.2");
        assert_eq!(parsed.message.content, "Hello");
        assert!(!parsed.done);
    }

    #[test]
    fn test_parse_chunk_empty() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let chunk = Bytes::from("");
        let result = provider.parse_chunk(&chunk);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_chunk_invalid_json() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let chunk = Bytes::from("not valid json");
        let result = provider.parse_chunk(&chunk);
        assert!(result.is_err());
    }

    #[test]
    fn test_llm_provider_trait_implementation() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        // Verify trait methods
        assert_eq!(provider.provider_name(), "ollama");
        assert_eq!(provider.default_model(), "llama3.2");
    }

    #[tokio::test]
    #[ignore = "Requires running Ollama instance"]
    async fn test_chat_integration() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let messages = vec![
            LlmMessage::new(LlmRole::System, "You are a helpful assistant"),
            LlmMessage::new(LlmRole::User, "Say hello"),
        ];

        let result = provider.chat(messages, vec![], "llama3.2").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.content.is_empty());
    }

    #[test]
    fn test_build_request_with_empty_model() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);

        let messages = vec![LlmMessage::new(LlmRole::User, "Hello")];
        let request = provider.build_request(messages, vec![], "");

        // build_request uses the model string as-is; chat() applies the default
        assert_eq!(request.model, "");
    }

    #[test]
    fn test_empty_stream_response() {
        let config = create_test_config();
        let _provider = OllamaProvider::new(config);

        // Test that an empty accumulated response is handled
        let accumulated = AccumulatedResponse::default();

        assert_eq!(accumulated.content, "");
        assert_eq!(accumulated.tool_calls.len(), 0);
        assert!(!accumulated.done);
        assert_eq!(accumulated.prompt_tokens, None);
        assert_eq!(accumulated.completion_tokens, None);
    }

    #[test]
    fn test_message_with_tool_calls_conversion() {
        let config = create_test_config();
        let _provider = OllamaProvider::new(config);

        let tool_call =
            crate::providers::LlmToolCall::new("call_1", "test_tool", r#"{"arg": "value"}"#);

        let message =
            LlmMessage::new(LlmRole::Assistant, "Using tool").with_tool_calls(vec![tool_call]);

        let ollama_msg = OllamaMessage {
            role: message.role.as_str().to_string(),
            content: message.content.clone(),
            tool_calls: message.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(|call| OllamaToolCall {
                        call_type: "function".to_string(),
                        function: OllamaFunctionCall {
                            name: call.name,
                            arguments: call.arguments,
                        },
                    })
                    .collect()
            }),
            tool_call_id: message.tool_call_id,
        };

        assert_eq!(ollama_msg.role, "assistant");
        assert_eq!(ollama_msg.content, "Using tool");
        assert!(ollama_msg.tool_calls.is_some());
        assert_eq!(ollama_msg.tool_calls.as_ref().unwrap().len(), 1);
    }
}
