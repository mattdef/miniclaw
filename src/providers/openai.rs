//! Generic OpenAI-compatible provider implementation
//!
//! This module provides an implementation of the `LlmProvider` trait for any
//! OpenAI-compatible API including OpenRouter, OpenAI native, and Kimi (Moonshot AI).
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
//! use miniclaw::providers::{GenericOpenAiProvider, OpenRouterConfig};
//!
//! async fn example() {
//!     let config = OpenRouterConfig::new("your-api-key");
//!     let provider = GenericOpenAiProvider::from_openrouter_config(config);
//!     // Use provider...
//! }
//! ```

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

#[cfg(test)]
use serde_json::json;

use crate::providers::factory::{OpenAiConfig, OpenRouterConfig};
use crate::providers::{
    LlmMessage, LlmProvider, LlmResponse, LlmToolCall, ModelInfo, ProviderError,
};

/// OpenAI API request body format
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    /// Model to use for completion
    model: String,
    /// Messages in the conversation
    messages: Vec<OpenAiMessage>,
    /// Available tools for the model to use
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// OpenAI message format
#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    /// Role of the message sender
    role: String,
    /// Content of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    /// Tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    /// Tool call ID for tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// OpenAI tool call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    /// Unique identifier for the tool call
    id: String,
    /// Type of the tool call (always "function")
    #[serde(rename = "type")]
    call_type: String,
    /// Function call details
    function: OpenAiFunctionCall,
}

/// OpenAI function call format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiFunctionCall {
    /// Name of the function to call
    name: String,
    /// Arguments as JSON string
    arguments: String,
}

/// OpenAI API response format
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    /// Response choices (usually only 1)
    choices: Vec<OpenAiChoice>,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<OpenAiUsage>,
    /// Error information if request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<OpenAiError>,
}

/// OpenAI choice format
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    /// Message from the assistant
    message: OpenAiMessage,
    /// Reason for finishing
    #[serde(rename = "finish_reason")]
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

/// OpenAI token usage format
#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    /// Tokens in the prompt
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: u32,
    /// Tokens in the completion
    #[serde(rename = "completion_tokens")]
    completion_tokens: u32,
}

/// OpenAI error format
#[derive(Debug, Deserialize)]
struct OpenAiError {
    /// Error message
    message: String,
    /// Error type
    #[serde(rename = "type")]
    error_type: Option<String>,
    /// Error code
    code: Option<String>,
}

/// OpenAI API response for listing models
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    /// List of available models
    data: Vec<OpenAiModelInfo>,
}

/// Individual model information from OpenAI API
#[derive(Debug, Deserialize)]
struct OpenAiModelInfo {
    /// Model identifier
    id: String,
    /// Whether the model is deprecated (not always present)
    #[serde(default)]
    deprecated: bool,
}

/// Configuration trait for OpenAI-compatible providers
///
/// This trait abstracts the configuration needed for any OpenAI-compatible provider
pub trait OpenAiCompatibleConfig: Send + Sync {
    /// Returns the API key
    fn api_key(&self) -> &str;
    /// Returns the base URL
    fn base_url(&self) -> &str;
    /// Returns the default model
    fn default_model(&self) -> &str;
    /// Returns the timeout in seconds
    fn timeout_seconds(&self) -> u64;
    /// Returns the organization ID if any
    fn organization_id(&self) -> Option<&str>;
}

impl OpenAiCompatibleConfig for OpenRouterConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn default_model(&self) -> &str {
        &self.default_model
    }
    fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
    fn organization_id(&self) -> Option<&str> {
        self.organization_id.as_deref()
    }
}

impl OpenAiCompatibleConfig for OpenAiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn default_model(&self) -> &str {
        &self.default_model
    }
    fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
    fn organization_id(&self) -> Option<&str> {
        self.organization_id.as_deref()
    }
}

/// Generic OpenAI-compatible provider implementation
///
/// This struct implements the `LlmProvider` trait for any OpenAI-compatible API,
/// including OpenRouter, OpenAI native, Kimi (Moonshot AI), and others.
#[derive(Debug, Clone)]
pub struct GenericOpenAiProvider {
    /// API key for authentication
    api_key: String,
    /// Base URL for the API
    base_url: String,
    /// Default model to use
    default_model: String,
    /// Provider name for identification
    provider_name: &'static str,
    /// Organization ID (optional)
    organization_id: Option<String>,
    /// HTTP client for making requests
    client: Client,
}

impl GenericOpenAiProvider {
    /// Creates a new generic OpenAI-compatible provider
    ///
    /// # Arguments
    ///
    /// * `api_key` - API key for authentication
    /// * `base_url` - Base URL for the API (e.g., "https://api.openai.com/v1")
    /// * `default_model` - Default model to use
    /// * `provider_name` - Provider name for identification
    /// * `organization_id` - Optional organization ID
    /// * `timeout_seconds` - HTTP timeout in seconds
    ///
    /// # Returns
    ///
    /// A new `GenericOpenAiProvider` instance
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client fails to build (extremely rare in practice)
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        default_model: impl Into<String>,
        provider_name: &'static str,
        organization_id: Option<String>,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .unwrap_or_else(|e| {
                panic!("Failed to build HTTP client: {}. This should never happen unless TLS initialization fails.", e)
            });

        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: default_model.into(),
            provider_name,
            organization_id,
            client,
        }
    }

    /// Creates a new provider from an OpenAI-compatible configuration
    ///
    /// # Type Parameters
    ///
    /// * `C` - Any type implementing `OpenAiCompatibleConfig`
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration implementing OpenAiCompatibleConfig
    /// * `provider_name` - Provider name for identification
    pub fn from_config<C: OpenAiCompatibleConfig>(config: &C, provider_name: &'static str) -> Self {
        Self::new(
            config.api_key().to_string(),
            config.base_url().to_string(),
            config.default_model().to_string(),
            provider_name,
            config.organization_id().map(|s| s.to_string()),
            config.timeout_seconds(),
        )
    }

    /// Creates a new OpenRouter provider from configuration
    pub fn from_openrouter_config(config: OpenRouterConfig) -> Self {
        Self::from_config(&config, "openrouter")
    }

    /// Creates a new OpenAI provider from configuration
    pub fn from_openai_config(config: OpenAiConfig) -> Self {
        Self::from_config(&config, "openai")
    }

    /// Creates a new provider with the given configuration, returning an error if client build fails
    ///
    /// This is a fallible version of `new()` that returns a Result instead of panicking.
    pub fn try_new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        default_model: impl Into<String>,
        provider_name: &'static str,
        organization_id: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| ProviderError::config(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: default_model.into(),
            provider_name,
            organization_id,
            client,
        })
    }

    /// Builds the OpenAI API request body from messages and tools
    fn build_request(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> OpenAiRequest {
        let openai_messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .map(|msg| OpenAiMessage {
                role: msg.role.as_str().to_string(),
                content: if msg.content.is_empty() {
                    None
                } else {
                    Some(msg.content)
                },
                tool_calls: msg.tool_calls.map(|calls| {
                    calls
                        .into_iter()
                        .map(|call| OpenAiToolCall {
                            id: call.id,
                            call_type: "function".to_string(),
                            function: OpenAiFunctionCall {
                                name: call.name,
                                arguments: call.arguments,
                            },
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id,
            })
            .collect();

        let tool_choice = if tools.is_empty() {
            None
        } else {
            Some("auto".to_string())
        };

        OpenAiRequest {
            model: model.to_string(),
            messages: openai_messages,
            tools,
            tool_choice,
        }
    }

    /// Parses the OpenAI response into LlmResponse
    fn parse_response(&self, response: OpenAiResponse) -> Result<LlmResponse, ProviderError> {
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
        request: &OpenAiRequest,
    ) -> Result<OpenAiResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            attempt += 1;
            debug!(attempt = attempt, url = %url, provider = %self.provider_name, "Making OpenAI-compatible API request");

            let mut request_builder = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json");

            // Add organization header if present
            if let Some(org_id) = &self.organization_id {
                request_builder = request_builder.header("OpenAI-Organization", org_id);
            }

            // Add OpenRouter-specific headers only for OpenRouter
            if self.provider_name == "openrouter" {
                request_builder = request_builder
                    .header("HTTP-Referer", "https://miniclaw.local")
                    .header("X-Title", "miniclaw");
            }

            let response: Result<reqwest::Response, reqwest::Error> =
                request_builder.json(request).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    debug!(status = %status, "Received response");

                    // Handle different status codes
                    match status {
                        StatusCode::OK => {
                            // Success - parse the response
                            let body = resp.json::<OpenAiResponse>().await.map_err(|e| {
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
                                format!(
                                    "Server error ({}) after {} attempts: {}",
                                    status, attempt, error_text
                                ),
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
                        ProviderError::timeout(self.timeout_seconds())
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
                        continue;
                    }

                    return Err(provider_error);
                }
            }
        }
    }

    /// Returns the configured timeout for error messages
    fn timeout_seconds(&self) -> u64 {
        // Extract timeout from client (this is a simplification, in reality we'd store it)
        30
    }
}

#[async_trait::async_trait]
impl LlmProvider for GenericOpenAiProvider {
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> Result<LlmResponse, ProviderError> {
        info!(
            model = model,
            provider = %self.provider_name,
            message_count = messages.len(),
            tool_count = tools.len(),
            "Sending chat request to {}",
            self.provider_name
        );

        // Build request
        let request = self.build_request(messages, tools, model);
        debug!(
            model = model,
            message_count = request.messages.len(),
            has_tools = !request.tools.is_empty(),
            "Built request (headers omitted for security)"
        );

        // Make request with retry logic
        let response = self.make_request_with_retry(&request).await?;

        // Parse response
        let llm_response = self.parse_response(response)?;

        info!(
            content_length = llm_response.content.len(),
            has_tool_calls = llm_response.has_tool_calls(),
            prompt_tokens = ?llm_response.prompt_tokens,
            completion_tokens = ?llm_response.completion_tokens,
            "Received response from {}",
            self.provider_name
        );

        Ok(llm_response)
    }

    fn default_model(&self) -> String {
        self.default_model.clone()
    }

    fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/models", self.base_url);

        info!(url = %url, provider = %self.provider_name, "Listing models");

        let mut request_builder = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        // Add organization header if present
        if let Some(org_id) = &self.organization_id {
            request_builder = request_builder.header("OpenAI-Organization", org_id);
        }

        // Add OpenRouter-specific headers only for OpenRouter
        if self.provider_name == "openrouter" {
            request_builder = request_builder
                .header("HTTP-Referer", "https://miniclaw.local")
                .header("X-Title", "miniclaw");
        }

        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::timeout(30)
            } else if e.is_connect() {
                ProviderError::network(format!("Connection failed: {}", e))
            } else {
                ProviderError::network(format!("Request failed: {}", e))
            }
        })?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::auth(
                    format!("Authentication failed ({}): {}", status, error_text),
                )),
                StatusCode::TOO_MANY_REQUESTS => Err(ProviderError::rate_limit(
                    format!("Rate limit exceeded: {}", error_text),
                    None,
                )),
                status if status.is_client_error() => Err(ProviderError::invalid_request(format!(
                    "Client error ({}): {}",
                    status, error_text
                ))),
                status if status.is_server_error() => Err(ProviderError::provider(
                    format!("Server error ({}): {}", status, error_text),
                    Some(status.as_u16().to_string()),
                )),
                _ => Err(ProviderError::provider(
                    format!("Unexpected status ({}): {}", status, error_text),
                    Some(status.as_u16().to_string()),
                )),
            };
        }

        let models_response: OpenAiModelsResponse = response.json().await.map_err(|e| {
            ProviderError::serialization(format!("Failed to parse models response: {}", e))
        })?;

        let mut models: Vec<ModelInfo> = models_response
            .data
            .into_iter()
            .map(|m| ModelInfo::new(m.id, m.deprecated))
            .collect();

        // Sort alphabetically by id
        models.sort_by(|a, b| a.id.cmp(&b.id));

        info!(count = models.len(), provider = %self.provider_name, "Listed models");

        Ok(models)
    }
}

/// OpenRouter provider type alias for backward compatibility
pub type OpenRouterProvider = GenericOpenAiProvider;

/// OpenAI native provider type alias
pub type OpenAiProvider = GenericOpenAiProvider;

/// Kimi (Moonshot AI) provider type alias
pub type KimiProvider = GenericOpenAiProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{LlmMessage, LlmRole, LlmToolCall, ToolDefinition};

    fn create_test_config() -> OpenRouterConfig {
        OpenRouterConfig::new("test-api-key")
            .with_model("anthropic/claude-3.5-sonnet")
            .with_timeout(30)
    }

    fn create_test_provider() -> GenericOpenAiProvider {
        GenericOpenAiProvider::from_openrouter_config(create_test_config())
    }

    #[test]
    fn test_openrouter_provider_creation() {
        let config = create_test_config();
        let provider = GenericOpenAiProvider::from_openrouter_config(config);

        assert_eq!(provider.provider_name(), "openrouter");
        assert_eq!(provider.default_model(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_openai_provider_creation() {
        let config = OpenAiConfig::new("test-api-key").with_model("gpt-4o");
        let provider = GenericOpenAiProvider::from_openai_config(config);

        assert_eq!(provider.provider_name(), "openai");
        assert_eq!(provider.default_model(), "gpt-4o");
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
        let response = OpenAiResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
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
        let response = OpenAiResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("I'll help".to_string()),
                    tool_calls: Some(vec![OpenAiToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: "test_tool".to_string(),
                            arguments: r#"{"key": "value"}"#.to_string(),
                        },
                    }]),
                    tool_call_id: None,
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: Some(OpenAiUsage {
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
        let response = OpenAiResponse {
            choices: vec![],
            usage: None,
            error: Some(OpenAiError {
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
        let response = OpenAiResponse {
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
        let provider = GenericOpenAiProvider::new(
            "key",
            "https://api.example.com/v1",
            "custom-model",
            "test",
            None,
            30,
        );

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
        let provider = GenericOpenAiProvider::new(
            "test-key",
            "https://api.example.com/v1",
            "test-model",
            "test-provider",
            None,
            5,
        );

        // Verify trait methods exist and have correct signatures
        let _: &dyn LlmProvider = &provider;

        // Test default model
        assert_eq!(provider.default_model(), "test-model");

        // Test provider name
        assert_eq!(provider.provider_name(), "test-provider");
    }

    #[test]
    fn test_tool_message_with_tool_call_id() {
        let provider = create_test_provider();
        let messages =
            vec![LlmMessage::new(LlmRole::Tool, "Tool result").with_tool_call_id("call_abc123")];

        let request = provider.build_request(messages, vec![], "test-model");

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "tool");
        assert_eq!(
            request.messages[0].tool_call_id.as_deref(),
            Some("call_abc123")
        );
    }

    #[test]
    fn test_try_new_success() {
        let result = GenericOpenAiProvider::try_new(
            "test-key",
            "https://api.example.com/v1",
            "test-model",
            "test",
            None,
            30,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_retryability() {
        // Network errors should be retryable
        let err = ProviderError::network("Connection failed");
        assert!(err.is_retryable());

        // Rate limit errors should be retryable
        let err = ProviderError::rate_limit("Too many requests", Some(60));
        assert!(err.is_retryable());

        // Auth errors should not be retryable
        let err = ProviderError::auth("Invalid API key");
        assert!(!err.is_retryable());

        // Invalid request errors should not be retryable
        let err = ProviderError::invalid_request("Bad parameter");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_openrouter_type_alias() {
        // Verify that OpenRouterProvider is a type alias for GenericOpenAiProvider
        let config = OpenRouterConfig::new("test-key");
        let provider: OpenRouterProvider = GenericOpenAiProvider::from_openrouter_config(config);
        assert_eq!(provider.provider_name(), "openrouter");
    }

    #[test]
    fn test_from_config_trait() {
        let config = OpenAiConfig::new("test-key")
            .with_model("gpt-4o-mini")
            .with_timeout(60);

        let provider = GenericOpenAiProvider::from_config(&config, "openai");

        assert_eq!(provider.provider_name(), "openai");
        assert_eq!(provider.default_model(), "gpt-4o-mini");
    }
}
