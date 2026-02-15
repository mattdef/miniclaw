//! LLM Provider module for miniclaw
//!
//! This module defines the trait and types for integrating with various LLM providers
//! such as OpenAI-compatible APIs (OpenRouter) and local models (Ollama).
//!
//! # Architecture
//!
//! The provider system uses a trait-based approach to allow multiple implementations:
//! - `LlmProvider` trait defines the interface all providers must implement
//! - Provider-specific implementations are in separate modules
//! - `ProviderFactory` creates the appropriate provider based on configuration
//!
//! # Example
//!
//! ```rust
//! use miniclaw::providers::{LlmProvider, LlmMessage, LlmRole};
//!
//! async fn example(provider: &dyn LlmProvider) {
//!     let messages = vec![
//!         LlmMessage::new(LlmRole::System, "You are a helpful assistant"),
//!         LlmMessage::new(LlmRole::User, "Hello!"),
//!     ];
//!     
//!     let response = provider.chat(messages, vec![], "model-name").await.unwrap();
//!     println!("Response: {}", response.content);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

pub mod error;
pub mod factory;
#[cfg(test)]
pub mod mock;
pub mod openai;

// Export error types
pub use error::ProviderError;

// Export factory types and configs
pub use factory::{OllamaConfig, OpenRouterConfig, ProviderConfig, ProviderFactory};

// Export OpenAI/OpenRouter provider
pub use openai::OpenRouterProvider;

/// Represents a message in the conversation for LLM context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmMessage {
    /// Role of the message sender
    pub role: LlmRole,
    /// Content of the message
    pub content: String,
    /// Optional tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
}

impl LlmMessage {
    /// Creates a new message with the specified role and content
    pub fn new(role: LlmRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Creates a new message with tool calls
    pub fn with_tool_calls(mut self, tool_calls: Vec<LlmToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Returns true if this message is from the system
    pub fn is_system(&self) -> bool {
        matches!(self.role, LlmRole::System)
    }

    /// Returns true if this message is from the user
    pub fn is_user(&self) -> bool {
        matches!(self.role, LlmRole::User)
    }

    /// Returns true if this message is from the assistant
    pub fn is_assistant(&self) -> bool {
        matches!(self.role, LlmRole::Assistant)
    }

    /// Returns true if this message is from a tool
    pub fn is_tool(&self) -> bool {
        matches!(self.role, LlmRole::Tool)
    }
}

/// Role of a message sender in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    /// System prompt/instructions
    System,
    /// User input
    User,
    /// Assistant response
    Assistant,
    /// Tool result
    Tool,
}

impl LlmRole {
    /// Returns the string representation of the role
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmRole::System => "system",
            LlmRole::User => "user",
            LlmRole::Assistant => "assistant",
            LlmRole::Tool => "tool",
        }
    }
}

impl fmt::Display for LlmRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to execute
    pub name: String,
    /// JSON-encoded arguments for the tool
    pub arguments: String,
}

impl LlmToolCall {
    /// Creates a new tool call
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    /// Parses the arguments as JSON
    pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

/// Response from the LLM provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmResponse {
    /// Content of the assistant's response
    pub content: String,
    /// Optional tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
    /// Number of tokens in the prompt (if provided by provider)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    /// Number of tokens in the completion (if provided by provider)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
}

impl LlmResponse {
    /// Creates a new response with just content
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: None,
            prompt_tokens: None,
            completion_tokens: None,
        }
    }

    /// Creates a new response with tool calls
    pub fn with_tool_calls(mut self, tool_calls: Vec<LlmToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Adds token usage information
    pub fn with_tokens(mut self, prompt: u32, completion: u32) -> Self {
        self.prompt_tokens = Some(prompt);
        self.completion_tokens = Some(completion);
        self
    }

    /// Returns true if this response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    /// Returns the total token count if available
    pub fn total_tokens(&self) -> Option<u32> {
        match (self.prompt_tokens, self.completion_tokens) {
            (Some(p), Some(c)) => Some(p + c),
            _ => None,
        }
    }
}

/// Definition of a tool for the LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for tool parameters
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Creates a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Converts to OpenAI function format
    pub fn to_openai_format(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// Trait for LLM providers (OpenAI-compatible, Ollama, etc.)
///
/// All implementations must be Send + Sync to allow concurrent usage across threads.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat request to the LLM with conversation history and available tools
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history
    /// * `tools` - Available tool definitions in OpenAI format (JSON values).
    ///   While `ToolDefinition` provides a typed way to create tools,
    ///   the trait accepts raw JSON to support provider-specific formats.
    ///   Use `ToolDefinition::to_openai_format()` to convert.
    /// * `model` - Model name to use for this request
    ///
    /// # Returns
    ///
    /// The LLM response containing content and optional tool calls
    ///
    /// # Design Note
    ///
    /// The trait accepts `Vec<serde_json::Value>` rather than `Vec<ToolDefinition>`
    /// to allow flexibility for provider-specific tool formats. Different providers
    /// may have slight variations in their tool schemas, and using JSON directly
    /// allows implementations to adapt without changing the trait signature.
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<serde_json::Value>,
        model: &str,
    ) -> Result<LlmResponse, ProviderError>;

    /// Returns the default model for this provider
    ///
    /// This is used when no specific model is requested
    fn default_model(&self) -> String;

    /// Returns the provider name
    ///
    /// Used for logging and identification
    fn provider_name(&self) -> &'static str;
}

/// Type alias for a boxed LlmProvider trait object
pub type BoxedProvider = Box<dyn LlmProvider>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_role_as_str() {
        assert_eq!(LlmRole::System.as_str(), "system");
        assert_eq!(LlmRole::User.as_str(), "user");
        assert_eq!(LlmRole::Assistant.as_str(), "assistant");
        assert_eq!(LlmRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_llm_role_display() {
        assert_eq!(format!("{}", LlmRole::System), "system");
        assert_eq!(format!("{}", LlmRole::User), "user");
    }

    #[test]
    fn test_llm_message_creation() {
        let msg = LlmMessage::new(LlmRole::User, "Hello");
        assert_eq!(msg.role, LlmRole::User);
        assert_eq!(msg.content, "Hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.is_user());
        assert!(!msg.is_system());
    }

    #[test]
    fn test_llm_message_with_tool_calls() {
        let tool_call = LlmToolCall::new("call_1", "test_tool", "{}");
        let msg =
            LlmMessage::new(LlmRole::Assistant, "Let me help").with_tool_calls(vec![tool_call]);

        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
        assert!(msg.is_assistant());
    }

    #[test]
    fn test_llm_tool_call_parse_arguments() {
        let tool_call = LlmToolCall::new("call_1", "test_tool", r#"{"key": "value", "num": 42}"#);

        #[derive(serde::Deserialize)]
        struct Args {
            key: String,
            num: i32,
        }

        let args: Args = tool_call.parse_arguments().unwrap();
        assert_eq!(args.key, "value");
        assert_eq!(args.num, 42);
    }

    #[test]
    fn test_llm_response_creation() {
        let response = LlmResponse::new("Hello!");
        assert_eq!(response.content, "Hello!");
        assert!(!response.has_tool_calls());
        assert!(response.total_tokens().is_none());
    }

    #[test]
    fn test_llm_response_with_tool_calls() {
        let tool_call = LlmToolCall::new("call_1", "test_tool", "{}");
        let response = LlmResponse::new("I'll help").with_tool_calls(vec![tool_call]);

        assert!(response.has_tool_calls());
        assert_eq!(response.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_llm_response_with_tokens() {
        let response = LlmResponse::new("Hello").with_tokens(10, 5);

        assert_eq!(response.prompt_tokens, Some(10));
        assert_eq!(response.completion_tokens, Some(5));
        assert_eq!(response.total_tokens(), Some(15));
    }

    #[test]
    fn test_tool_definition_openai_format() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let tool = ToolDefinition::new("test_tool", "A test tool", params.clone());
        let openai_format = tool.to_openai_format();

        assert_eq!(openai_format["type"], "function");
        assert_eq!(openai_format["function"]["name"], "test_tool");
        assert_eq!(openai_format["function"]["description"], "A test tool");
        assert_eq!(openai_format["function"]["parameters"], params);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let msg = LlmMessage::new(LlmRole::User, "Hello");
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: LlmMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_role_serialization() {
        let role = LlmRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");

        let decoded: LlmRole = serde_json::from_str(&json).unwrap();
        assert_eq!(role, decoded);
    }
}
