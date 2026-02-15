//! OpenRouter/OpenAI-compatible provider implementation
//!
//! This module provides an implementation of the `LlmProvider` trait for OpenRouter,
//! which offers an OpenAI-compatible API for accessing various LLM models.
//!
//! # Features
//!
//! - OpenAI-compatible API format
//! - Support for tool calling
//! - Token usage tracking
//! - Exponential backoff retry for rate limiting
//! - Comprehensive error handling
//!
//! # Example
//!
//! ```rust
//! use miniclaw::providers::{OpenRouterProvider, OpenRouterConfig};
//!
//! async fn example() {
//!     let config = OpenRouterConfig::new("your-api-key");
//!     let provider = OpenRouterProvider::new(config);
//!     // Use provider...
//! }
//! ```

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

use crate::providers::factory::OpenRouterConfig;
use crate::providers::{LlmMessage, LlmProvider, LlmResponse, LlmToolCall, ProviderError};

/// OpenRouter API request body format
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    /// Model to use for completion
    model: String,
    /// Messages in the conversation
    messages: Vec<OpenRouterMessage>,
    /// Available tools for the model to use
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// OpenRouter message format
#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterMessage {
    /// Role of the message sender
    role: String,
    /// Content of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    /// Tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenRouterToolCall>>,
}

/// OpenRouter tool call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenRouterToolCall {
    /// Unique identifier for the tool call
    id: String,
    /// Type of the tool call (always "function")
    #[serde(rename = "type")]
    call_type: String,
    /// Function call details
    function: OpenRouterFunctionCall,
}

/// OpenRouter function call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenRouterFunctionCall {
    /// Name of the function to call
    name: String,
    /// Arguments as JSON string
    arguments: String,
}

/// OpenRouter API response format
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    /// Response choices (usually only 1)
    choices: Vec<OpenRouterChoice>,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<OpenRouterUsage>,
    /// Error information if request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<OpenRouterError>,
}

/// OpenRouter choice format
#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    /// Message from the assistant
    message: OpenRouterMessage,
    /// Reason for finishing
    #[serde(rename = "finish_reason")]
    finish_reason: Option<String>,
}

/// OpenRouter token usage format
#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    /// Tokens in the prompt
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: u32,
    /// Tokens in the completion
    #[serde(rename = "completion_tokens")]
    completion_tokens: u32,
}

/// OpenRouter error format
#[derive(Debug, Deserialize)]
struct OpenRouterError {
    /// Error message
    message: String,
    /// Error type
    #[serde(rename = "type")]
    error_type: Option<String>,
    /// Error code
    code: Option<String>,
}

/// OpenRouter provider implementation
///
/// This struct implements the `LlmProvider` trait for OpenRouter's API,
/// which provides access to various LLM models through an OpenAI-compatible interface.
#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    /// Configuration for the provider
    config: OpenRouterConfig,
    /// HTTP client for making requests
    client: Client,
}

impl OpenRouterProvider {
    /// Creates a new OpenRouter provider with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - OpenRouter configuration including API key and settings
    ///
    /// # Returns
    ///
    /// A new `OpenRouterProvider` instance
    pub fn new(config: OpenRouterConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, client }
    }

    /// Builds the OpenRouter API request body from messages and tools
    fn build_request(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> OpenRouterRequest {
        let openrouter_messages: Vec<OpenRouterMessage> = messages
            .into_iter()
            .map(|msg| OpenRouterMessage {
                role: msg.role.as_str().to_string(),
                content: if msg.content.is_empty() {
                    None
                } else {
                    Some(msg.content)
                },
                tool_calls: msg.tool_calls.map(|calls| {
                    calls
                        .into_iter()
                        .map(|call| OpenRouterToolCall {
                            id: call.id,
                            call_type: "function".to_string(),
                            function: OpenRouterFunctionCall {
                                name: call.name,
                                arguments: call.arguments,
                            },
                        })
                        .collect()
                }),
            })
            .collect();

        let tool_choice = if tools.is_empty() {
            None
        } else {
            Some("auto".to_string())
        };

        OpenRouterRequest {
            model: model.to_string(),
            messages: openrouter_messages,
            tools,
            tool_choice,
        }
    }

    /// Parses the OpenRouter response into LlmResponse
    fn parse_response(&self, response: OpenRouterResponse) -> Result<LlmResponse, ProviderError> {
        // Check for API-level errors first
        if let Some(error) = response.error {
            return Err(ProviderError::provider(
                error.message,
                error.code.or(error.error_type),
            ));
        }

        // Get the first choice (there should always be at least one)
        let choice =
            response.choices.into_iter().next().ok_or_else(|| {
                ProviderError::provider("No response choices returned", None::<&str>)
            })?;

        let message = choice.message;

        // Convert tool calls if present
        let tool_calls = message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|call| LlmToolCall {
                    id: call.id,
                    name: call.function.name,
                    arguments: call.function.arguments,
                })
                .collect()
        });

        // Build response
        let mut llm_response = LlmResponse {
            content: message.content.unwrap_or_default(),
            tool_calls,
            prompt_tokens: None,
            completion_tokens: None,
        };

        // Add token usage if available
        if let Some(usage) = response.usage {
            llm_response.prompt_tokens = Some(usage.prompt_tokens);
            llm_response.completion_tokens = Some(usage.completion_tokens);
        }

        Ok(llm_response)
    }

    /// Makes the API request with retry logic for rate limiting
    async fn make_request_with_retry(
        &self,
        request: &OpenRouterRequest,
    ) -> Result<OpenRouterResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let max_retries = 3;
        let mut attempt = 0;
        let mut last_error: Option<ProviderError> = None;

        loop {
            attempt += 1;
            debug!(attempt = attempt, url = %url, "Making OpenRouter API request");

            let response: Result<reqwest::Response, reqwest::Error> = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://miniclaw.local")
                .header("X-Title", "miniclaw")
                .json(request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    debug!(status = %status, "Received response");

                    // Handle different status codes
                    match status {
                        StatusCode::OK => {
                            // Success - parse the response
                            let body = resp.json::<OpenRouterResponse>().await.map_err(|e| {
                                ProviderError::serialization(format!(
                                    "Failed to parse response: {}",
                                    e
                                ))
                            })?;
                            return Ok(body);
                        }
                        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(ProviderError::auth(format!(
                                "Authentication failed ({}): {}",
                                status, error_text
                            )));
                        }
                        StatusCode::TOO_MANY_REQUESTS => {
                            // Rate limit - retry with exponential backoff
                            if attempt >= max_retries {
                                let error_text = resp.text().await.unwrap_or_default();
                                return Err(ProviderError::rate_limit(
                                    format!(
                                        "Rate limit exceeded after {} retries: {}",
                                        max_retries, error_text
                                    ),
                                    None,
                                ));
                            }

                            let delay = 2_u64.pow(attempt - 1); // 1s, 2s, 4s
                            warn!(
                                attempt = attempt,
                                max_retries = max_retries,
                                delay_secs = delay,
                                "Rate limited, retrying with exponential backoff"
                            );
                            tokio::time::sleep(Duration::from_secs(delay)).await;
                            continue;
                        }
                        status if status.is_client_error() => {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(ProviderError::invalid_request(format!(
                                "Client error ({}): {}",
                                status, error_text
                            )));
                        }
                        status if status.is_server_error() => {
                            let error_text = resp.text().await.unwrap_or_default();
                            // Server errors might be transient, retry if we haven't exceeded max
                            if attempt < max_retries {
                                let delay = 2_u64.pow(attempt - 1);
                                warn!(
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    delay_secs = delay,
                                    "Server error, retrying",
                                );
                                tokio::time::sleep(Duration::from_secs(delay)).await;
                                continue;
                            }
                            return Err(ProviderError::provider(
                                format!("Server error ({}): {}", status, error_text),
                                Some(status.as_u16().to_string()),
                            ));
                        }
                        _ => {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(ProviderError::provider(
                                format!("Unexpected status ({}): {}", status, error_text),
                                Some(status.as_u16().to_string()),
                            ));
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Request failed");

                    // Convert reqwest error to ProviderError
                    let provider_error = if e.is_timeout() {
                        ProviderError::timeout(self.config.timeout_seconds)
                    } else if e.is_connect() {
                        ProviderError::network(format!("Connection failed: {}", e))
                    } else {
                        ProviderError::network(format!("Request failed: {}", e))
                    };

                    // Check if we should retry
                    if attempt < max_retries && provider_error.is_retryable() {
                        let delay = provider_error.retry_after().unwrap_or(1);
                        warn!(
                            attempt = attempt,
                            max_retries = max_retries,
                            delay_secs = delay,
                            error = %e,
                            "Request failed, retrying"
                        );
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                        last_error = Some(provider_error);
                        continue;
                    }

                    return Err(provider_error);
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> Result<LlmResponse, ProviderError> {
        info!(
            model = model,
            message_count = messages.len(),
            tool_count = tools.len(),
            "Sending chat request to OpenRouter"
        );

        // Build request
        let request = self.build_request(messages, tools, model);
        debug!(request = ?request, "Built request");

        // Make request with retry logic
        let response = self.make_request_with_retry(&request).await?;

        // Parse response
        let llm_response = self.parse_response(response)?;

        info!(
            content_length = llm_response.content.len(),
            has_tool_calls = llm_response.has_tool_calls(),
            prompt_tokens = ?llm_response.prompt_tokens,
            completion_tokens = ?llm_response.completion_tokens,
            "Received response from OpenRouter"
        );

        Ok(llm_response)
    }

    fn default_model(&self) -> String {
        self.config.default_model.clone()
    }

    fn provider_name(&self) -> &'static str {
        "openrouter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{LlmMessage, LlmRole, LlmToolCall, ToolDefinition};

    fn create_test_config() -> OpenRouterConfig {
        OpenRouterConfig::new("test-api-key")
            .with_model("anthropic/claude-3.5-sonnet")
            .with_timeout(30)
    }

    fn create_test_provider() -> OpenRouterProvider {
        OpenRouterProvider::new(create_test_config())
    }

    #[test]
    fn test_openrouter_provider_creation() {
        let config = create_test_config();
        let provider = OpenRouterProvider::new(config);

        assert_eq!(provider.provider_name(), "openrouter");
        assert_eq!(provider.default_model(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_build_request_basic() {
        let provider = create_test_provider();
        let messages = vec![
            LlmMessage::new(LlmRole::System, "You are helpful"),
            LlmMessage::new(LlmRole::User, "Hello"),
        ];

        let request = provider.build_request(messages, vec![], "test-model");

        assert_eq!(request.model, "test-model");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, "system");
        assert_eq!(
            request.messages[0].content,
            Some("You are helpful".to_string())
        );
        assert_eq!(request.messages[1].role, "user");
        assert_eq!(request.messages[1].content, Some("Hello".to_string()));
        assert!(request.tools.is_empty());
        assert!(request.tool_choice.is_none());
    }

    #[test]
    fn test_build_request_with_tools() {
        let provider = create_test_provider();
        let messages = vec![LlmMessage::new(LlmRole::User, "Use a tool")];
        let tool = ToolDefinition::new(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            }),
        );
        let tools = vec![tool.to_openai_format()];

        let request = provider.build_request(messages, tools, "test-model");

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tool_choice, Some("auto".to_string()));
    }

    #[test]
    fn test_build_request_with_tool_calls() {
        let provider = create_test_provider();
        let tool_call = LlmToolCall::new("call_1", "test_tool", r#"{"key": "value"}"#);
        let messages =
            vec![LlmMessage::new(LlmRole::Assistant, "I'll help").with_tool_calls(vec![tool_call])];

        let request = provider.build_request(messages, vec![], "test-model");

        assert_eq!(request.messages.len(), 1);
        assert!(request.messages[0].tool_calls.is_some());
        let calls = request.messages[0].tool_calls.as_ref().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].function.name, "test_tool");
    }

    #[test]
    fn test_parse_response_success() {
        let provider = create_test_provider();
        let response = OpenRouterResponse {
            choices: vec![OpenRouterChoice {
                message: OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenRouterUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
            error: None,
        };

        let result = provider.parse_response(response).unwrap();

        assert_eq!(result.content, "Hello!");
        assert!(!result.has_tool_calls());
        assert_eq!(result.prompt_tokens, Some(10));
        assert_eq!(result.completion_tokens, Some(5));
        assert_eq!(result.total_tokens(), Some(15));
    }

    #[test]
    fn test_parse_response_with_tool_calls() {
        let provider = create_test_provider();
        let response = OpenRouterResponse {
            choices: vec![OpenRouterChoice {
                message: OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: Some("I'll help".to_string()),
                    tool_calls: Some(vec![OpenRouterToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: OpenRouterFunctionCall {
                            name: "test_tool".to_string(),
                            arguments: r#"{"key": "value"}"#.to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: Some(OpenRouterUsage {
                prompt_tokens: 20,
                completion_tokens: 10,
            }),
            error: None,
        };

        let result = provider.parse_response(response).unwrap();

        assert_eq!(result.content, "I'll help");
        assert!(result.has_tool_calls());
        let calls = result.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "test_tool");
        assert_eq!(calls[0].arguments, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_parse_response_api_error() {
        let provider = create_test_provider();
        let response = OpenRouterResponse {
            choices: vec![],
            usage: None,
            error: Some(OpenRouterError {
                message: "Invalid API key".to_string(),
                error_type: Some("authentication_error".to_string()),
                code: Some("401".to_string()),
            }),
        };

        let result = provider.parse_response(response);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_parse_response_no_choices() {
        let provider = create_test_provider();
        let response = OpenRouterResponse {
            choices: vec![],
            usage: None,
            error: None,
        };

        let result = provider.parse_response(response);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No response choices returned"));
    }

    #[test]
    fn test_default_model() {
        let config = OpenRouterConfig::new("key").with_model("custom-model");
        let provider = OpenRouterProvider::new(config);

        assert_eq!(provider.default_model(), "custom-model");
    }

    #[test]
    fn test_provider_name() {
        let provider = create_test_provider();
        assert_eq!(provider.provider_name(), "openrouter");
    }

    #[test]
    fn test_empty_content_handling() {
        let provider = create_test_provider();
        let messages = vec![LlmMessage::new(LlmRole::User, "")];

        let request = provider.build_request(messages, vec![], "test-model");

        // Empty content should be serialized as None (skipped)
        assert!(request.messages[0].content.is_none());
    }

    #[test]
    fn test_message_role_conversion() {
        let provider = create_test_provider();
        let messages = vec![
            LlmMessage::new(LlmRole::System, "System message"),
            LlmMessage::new(LlmRole::User, "User message"),
            LlmMessage::new(LlmRole::Assistant, "Assistant message"),
            LlmMessage::new(LlmRole::Tool, "Tool message"),
        ];

        let request = provider.build_request(messages, vec![], "test-model");

        assert_eq!(request.messages[0].role, "system");
        assert_eq!(request.messages[1].role, "user");
        assert_eq!(request.messages[2].role, "assistant");
        assert_eq!(request.messages[3].role, "tool");
    }

    #[tokio::test]
    async fn test_chat_trait_implementation() {
        // This test verifies the trait is implemented correctly
        // We use the mock provider pattern to avoid actual API calls
        let config = OpenRouterConfig::new("test-key").with_timeout(5);
        let provider = OpenRouterProvider::new(config);

        // Verify trait methods exist and have correct signatures
        let _: &dyn LlmProvider = &provider;

        // Test default model
        assert_eq!(provider.default_model(), "anthropic/claude-3.5-sonnet");

        // Test provider name
        assert_eq!(provider.provider_name(), "openrouter");
    }
}
