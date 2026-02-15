//! Mock LLM provider for testing
//!
//! This module provides a mock implementation of LlmProvider for unit testing.
//! It allows configuring responses and simulating errors without making actual API calls.

use std::sync::{Arc, Mutex};

use crate::providers::{LlmMessage, LlmProvider, LlmResponse, LlmToolCall, ProviderError};

/// Mock provider for testing
///
/// This provider can be configured with predefined responses and supports
/// simulating various error conditions for testing error handling.
///
/// # Example
///
/// ```rust
/// use miniclaw::providers::mock::MockLlmProvider;
/// use miniclaw::providers::{LlmMessage, LlmRole};
///
/// async fn test_example() {
///     let mut mock = MockLlmProvider::new();
///     mock.set_response("Hello!");
///
///     let messages = vec![LlmMessage::new(LlmRole::User, "Hi")];
///     let response = mock.chat(messages, vec![], "model").await.unwrap();
///
///     assert_eq!(response.content, "Hello!");
/// }
/// ```
pub struct MockLlmProvider {
    /// The response to return from chat()
    response: Arc<Mutex<LlmResponse>>,
    /// Optional error to return instead of the response
    error: Arc<Mutex<Option<ProviderError>>>,
    /// Default model name
    default_model: String,
    /// Provider name (stored as &'static str for trait compliance)
    name: &'static str,
    /// Call counter for tracking
    call_count: Arc<Mutex<usize>>,
    /// Last messages received (for verification)
    last_messages: Arc<Mutex<Option<Vec<LlmMessage>>>>,
}

impl MockLlmProvider {
    /// Creates a new mock provider with default settings
    pub fn new() -> Self {
        Self {
            response: Arc::new(Mutex::new(LlmResponse::new("Mock response"))),
            error: Arc::new(Mutex::new(None)),
            default_model: "mock-model".to_string(),
            name: "MockProvider",
            call_count: Arc::new(Mutex::new(0)),
            last_messages: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a new mock provider with a specific name
    pub fn with_name(name: &'static str) -> Self {
        Self {
            name,
            response: Arc::new(Mutex::new(LlmResponse::new("Mock response"))),
            error: Arc::new(Mutex::new(None)),
            default_model: "mock-model".to_string(),
            call_count: Arc::new(Mutex::new(0)),
            last_messages: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the response content to return
    pub fn set_response(&self, content: impl Into<String>) {
        let mut response = self.response.lock().unwrap();
        *response = LlmResponse::new(content);
    }

    /// Sets a response with tool calls
    pub fn set_response_with_tool_calls(&self,
        content: impl Into<String>,
        tool_calls: Vec<LlmToolCall>,
    ) {
        let mut response = self.response.lock().unwrap();
        *response = LlmResponse::new(content).with_tool_calls(tool_calls);
    }

    /// Sets the error to return (instead of the response)
    pub fn set_error(&self, error: ProviderError) {
        let mut err = self.error.lock().unwrap();
        *err = Some(error);
    }

    /// Clears any set error
    pub fn clear_error(&self) {
        let mut err = self.error.lock().unwrap();
        *err = None;
    }

    /// Sets the default model name
    pub fn set_default_model(&mut self, model: impl Into<String>) {
        self.default_model = model.into();
    }

    /// Returns the number of times chat() was called
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// Returns the last messages passed to chat()
    pub fn last_messages(&self) -> Option<Vec<LlmMessage>> {
        self.last_messages.lock().unwrap().clone()
    }

    /// Resets the mock to initial state
    pub fn reset(&self) {
        *self.response.lock().unwrap() = LlmResponse::new("Mock response");
        *self.error.lock().unwrap() = None;
        *self.call_count.lock().unwrap() = 0;
        *self.last_messages.lock().unwrap() = None;
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        _tools: Vec<serde_json::Value>,
        _model: &str,
    ) -> Result<LlmResponse, ProviderError> {
        // Increment call counter
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        // Store last messages
        let mut last = self.last_messages.lock().unwrap();
        *last = Some(messages);

        // Check if we should return an error
        let error = self.error.lock().unwrap().clone();
        if let Some(err) = error {
            return Err(err);
        }

        // Return the configured response
        let response = self.response.lock().unwrap().clone();
        Ok(response)
    }

    fn default_model(&self) -> String {
        self.default_model.clone()
    }

    fn provider_name(&self) -> &'static str {
        self.name
    }
}

/// Builder for creating mock providers with specific configurations
pub struct MockProviderBuilder {
    provider: MockLlmProvider,
    custom_name: Option<&'static str>,
}

impl MockProviderBuilder {
    /// Creates a new builder
    pub fn new() -> Self {
        Self {
            provider: MockLlmProvider::new(),
            custom_name: None,
        }
    }

    /// Sets the response content
    pub fn response(self, content: impl Into<String>) -> Self {
        self.provider.set_response(content);
        self
    }

    /// Sets the response with tool calls
    pub fn response_with_tool_calls(
        self,
        content: impl Into<String>,
        tool_calls: Vec<LlmToolCall>,
    ) -> Self {
        self.provider.set_response_with_tool_calls(content, tool_calls);
        self
    }

    /// Sets an error to return
    pub fn error(self, error: ProviderError) -> Self {
        self.provider.set_error(error);
        self
    }

    /// Sets the default model
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.provider.set_default_model(model);
        self
    }

    /// Sets the provider name
    pub fn name(mut self, name: &'static str) -> Self {
        self.custom_name = Some(name);
        self
    }

    /// Builds the mock provider
    pub fn build(mut self) -> MockLlmProvider {
        if let Some(name) = self.custom_name {
            self.provider.name = name;
        }
        self.provider
    }
}

impl Default for MockProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{LlmRole, ProviderError};

    #[tokio::test]
    async fn test_mock_provider_basic() {
        let mock = MockLlmProvider::new();
        mock.set_response("Hello!");

        let messages = vec![LlmMessage::new(LlmRole::User, "Hi")];
        let response = mock.chat(messages, vec![], "model").await.unwrap();

        assert_eq!(response.content, "Hello!");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_provider_with_tool_calls() {
        let mock = MockLlmProvider::new();
        let tool_call = LlmToolCall::new("call_1", "test_tool", "{}");
        mock.set_response_with_tool_calls("I'll help", vec![tool_call]);

        let messages = vec![LlmMessage::new(LlmRole::User, "Help")];
        let response = mock.chat(messages, vec![], "model").await.unwrap();

        assert_eq!(response.content, "I'll help");
        assert!(response.has_tool_calls());
    }

    #[tokio::test]
    async fn test_mock_provider_error() {
        let mock = MockLlmProvider::new();
        mock.set_error(ProviderError::network("Connection failed"));

        let messages = vec![LlmMessage::new(LlmRole::User, "Hi")];
        let result = mock.chat(messages, vec![], "model").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Network error"));
    }

    #[tokio::test]
    async fn test_mock_provider_clears_error() {
        let mock = MockLlmProvider::new();
        mock.set_error(ProviderError::network("Connection failed"));
        mock.clear_error();

        let messages = vec![LlmMessage::new(LlmRole::User, "Hi")];
        let result = mock.chat(messages, vec![], "model").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_tracks_messages() {
        let mock = MockLlmProvider::new();
        mock.set_response("Response");

        let messages = vec![
            LlmMessage::new(LlmRole::System, "System prompt"),
            LlmMessage::new(LlmRole::User, "User message"),
        ];
        mock.chat(messages.clone(), vec![], "model").await.unwrap();

        let last = mock.last_messages().unwrap();
        assert_eq!(last.len(), 2);
        assert_eq!(last[0].content, "System prompt");
        assert_eq!(last[1].content, "User message");
    }

    #[tokio::test]
    async fn test_mock_provider_reset() {
        let mock = MockLlmProvider::new();
        mock.set_response("Custom");
        mock.set_error(ProviderError::network("fail"));

        let _ = mock.chat(vec![], vec![], "model").await;
        assert_eq!(mock.call_count(), 1);

        mock.reset();
        assert_eq!(mock.call_count(), 0);
        // After reset, should return default response, not error
        let result = mock.chat(vec![], vec![], "model").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_default_model() {
        let mut mock = MockLlmProvider::new();
        assert_eq!(mock.default_model(), "mock-model");

        mock.set_default_model("custom-model");
        assert_eq!(mock.default_model(), "custom-model");
    }

    #[tokio::test]
    async fn test_mock_provider_name() {
        let mock = MockLlmProvider::with_name("TestProvider");
        assert_eq!(mock.provider_name(), "TestProvider");
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let mock = MockProviderBuilder::new()
            .name("BuilderTest")
            .response("Built response")
            .default_model("built-model")
            .build();

        assert_eq!(mock.provider_name(), "BuilderTest");
        assert_eq!(mock.default_model(), "built-model");

        let response = mock
            .chat(vec![LlmMessage::new(LlmRole::User, "Hi")], vec![], "model")
            .await
            .unwrap();
        assert_eq!(response.content, "Built response");
    }

    #[tokio::test]
    async fn test_builder_with_error() {
        let mock = MockProviderBuilder::new()
            .response("Success")
            .error(ProviderError::timeout(30))
            .build();

        let result = mock.chat(vec![], vec![], "model").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockLlmProvider>();
    }
}
