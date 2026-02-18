use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, mpsc};

use crate::agent::metrics::ResponseMetrics;
use crate::agent::tools::ToolRegistry;
use crate::chat::{ChatHub, InboundMessage};
use crate::providers::{LlmMessage, LlmProvider, LlmResponse, LlmRole, LlmToolCall};
use crate::session::{Session, SessionManager};

/// Maximum number of iterations before terminating to prevent infinite loops
pub const MAX_ITERATIONS: u32 = 200;

/// Timeout for LLM API calls in seconds
pub const LLM_TIMEOUT_SECS: u64 = 30;

/// Maximum number of retries for transient LLM errors
pub const MAX_LLM_RETRIES: u32 = 3;

/// Target startup time in milliseconds (NFR-P3)
pub const TARGET_STARTUP_TIME_MS: u128 = 100;

/// Target response time (95th percentile) in milliseconds (NFR-P4)
pub const TARGET_RESPONSE_TIME_P95_MS: u128 = 2000;

/// Errors that can occur during agent loop execution
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Context building failed: {0}")]
    ContextBuildError(String),

    #[error("LLM communication failed: {0}")]
    LlmError(String),

    #[error("Max iterations ({0}) reached")]
    MaxIterationsReached(u32),

    #[error("Tool execution failed: {0}")]
    ToolExecutionError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Chat hub error: {0}")]
    ChatHubError(String),
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Trait for building context from various sources
#[async_trait::async_trait]
pub trait ContextBuilder: Send + Sync {
    /// Assemble the complete context for a conversation
    async fn build_context(
        &self,
        session: &Session,
        current_message: &InboundMessage,
    ) -> Result<Vec<LlmMessage>>;
}

/// The main agent loop that orchestrates message processing
pub struct AgentLoop {
    chat_hub: Arc<ChatHub>,
    llm_provider: Arc<dyn LlmProvider>,
    context_builder: Arc<dyn ContextBuilder>,
    tool_registry: Arc<ToolRegistry>,
    session_manager: Arc<RwLock<SessionManager>>,
    max_iterations: u32,
    model: String,
    response_metrics: Arc<ResponseMetrics>,
    inbound_rx: Mutex<Option<mpsc::Receiver<InboundMessage>>>,
}

impl AgentLoop {
    /// Creates a new AgentLoop with the required dependencies
    pub fn new(
        chat_hub: Arc<ChatHub>,
        llm_provider: Arc<dyn LlmProvider>,
        context_builder: Arc<dyn ContextBuilder>,
        tool_registry: Arc<ToolRegistry>,
        session_manager: Arc<RwLock<SessionManager>>,
    ) -> Self {
        let model = llm_provider.default_model();
        Self {
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
            max_iterations: MAX_ITERATIONS,
            model,
            response_metrics: Arc::new(ResponseMetrics::new()),
            inbound_rx: Mutex::new(None),
        }
    }

    /// Creates a new AgentLoop with an inbound message receiver
    pub fn with_inbound_receiver(
        chat_hub: Arc<ChatHub>,
        llm_provider: Arc<dyn LlmProvider>,
        context_builder: Arc<dyn ContextBuilder>,
        tool_registry: Arc<ToolRegistry>,
        session_manager: Arc<RwLock<SessionManager>>,
        inbound_rx: mpsc::Receiver<InboundMessage>,
    ) -> Self {
        let model = llm_provider.default_model();
        Self {
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
            max_iterations: MAX_ITERATIONS,
            model,
            response_metrics: Arc::new(ResponseMetrics::new()),
            inbound_rx: Mutex::new(Some(inbound_rx)),
        }
    }

    /// Creates a new AgentLoop with a specific model override
    pub fn with_model(
        chat_hub: Arc<ChatHub>,
        llm_provider: Arc<dyn LlmProvider>,
        context_builder: Arc<dyn ContextBuilder>,
        tool_registry: Arc<ToolRegistry>,
        session_manager: Arc<RwLock<SessionManager>>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self {
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
            max_iterations: MAX_ITERATIONS,
            model,
            response_metrics: Arc::new(ResponseMetrics::new()),
            inbound_rx: Mutex::new(None),
        }
    }

    /// Creates a new AgentLoop with a specific model override and inbound receiver
    pub fn with_model_and_receiver(
        chat_hub: Arc<ChatHub>,
        llm_provider: Arc<dyn LlmProvider>,
        context_builder: Arc<dyn ContextBuilder>,
        tool_registry: Arc<ToolRegistry>,
        session_manager: Arc<RwLock<SessionManager>>,
        model: impl Into<String>,
        inbound_rx: mpsc::Receiver<InboundMessage>,
    ) -> Self {
        let model = model.into();
        Self {
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
            max_iterations: MAX_ITERATIONS,
            model,
            response_metrics: Arc::new(ResponseMetrics::new()),
            inbound_rx: Mutex::new(Some(inbound_rx)),
        }
    }

    /// Returns the maximum iterations limit
    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    /// Returns the current model being used
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Processes a single inbound message through the agent loop
    ///
    /// This is the main entry point for handling messages. It:
    /// 1. Gets or creates a session for the chat
    /// 2. Adds the user message to the session
    /// 3. Builds the conversation context
    /// 4. Runs the LLM→Tools→Reply cycle
    /// 5. Returns the final response
    pub async fn process_message(&self, message: InboundMessage) -> Result<String> {
        let session_id = format!("{}_{}", message.channel, message.chat_id);

        // Start timing for response measurement
        let msg_start = std::time::Instant::now();

        tracing::debug!(
            session_id = %session_id,
            channel = %message.channel,
            chat_id = %message.chat_id,
            "Starting message processing"
        );

        // Get or create session
        let mut session = self
            .get_or_create_session(&message.channel, &message.chat_id)
            .await?;

        // Add user message to session
        let user_message =
            crate::session::Message::new("user".to_string(), message.content.clone());
        session.add_message(user_message);

        // Build context with timing
        let context_start = std::time::Instant::now();
        let context = self
            .context_builder
            .build_context(&session, &message)
            .await
            .map_err(|e| AgentError::ContextBuildError(e.to_string()))?;
        let context_time = context_start.elapsed();

        tracing::debug!(
            session_id = %session_id,
            context_messages = context.len(),
            context_ms = context_time.as_millis(),
            "Context built successfully"
        );

        tracing::trace!(
            session_id = %session_id,
            context_ms = context_time.as_millis(),
            "Component timing: context assembly"
        );

        // Run the main agent loop
        let response = self
            .run_agent_loop(&session_id, &mut session, context)
            .await?;

        // Calculate and log response time
        let response_time = msg_start.elapsed();

        // Record for percentile tracking
        self.response_metrics.record(response_time);

        // Log individual response time and current metrics
        if let (Some(p95), Some(avg)) = (
            self.response_metrics.percentile_95(),
            self.response_metrics.average(),
        ) {
            tracing::debug!(
                response_ms = response_time.as_millis(),
                p95_ms = p95,
                avg_ms = avg,
                sample_count = self.response_metrics.sample_count(),
                session_id = %session_id,
                "Message processed"
            );

            // Warn if 95th percentile exceeds target
            if p95 > TARGET_RESPONSE_TIME_P95_MS {
                tracing::warn!(
                    p95_ms = p95,
                    target_ms = TARGET_RESPONSE_TIME_P95_MS,
                    "Response time 95th percentile exceeds target"
                );
            }
        } else {
            // First few samples before we have enough data for percentile
            tracing::debug!(
                response_ms = response_time.as_millis(),
                session_id = %session_id,
                "Message processed"
            );
        }

        Ok(response)
    }

    /// Gets an existing session or creates a new one
    async fn get_or_create_session(&self, channel: &str, chat_id: &str) -> Result<Session> {
        let session_manager = self.session_manager.read().await;

        session_manager
            .get_or_create_session(channel, chat_id)
            .await
            .map_err(|e| AgentError::SessionError(e.to_string()))
    }

    /// Runs the main agent loop: LLM → Tools → Reply cycle
    ///
    /// This method implements the core loop that:
    /// - Calls the LLM with current context
    /// - Handles tool calls if present
    /// - Continues iterating until text-only response or max iterations
    async fn run_agent_loop(
        &self,
        session_id: &str,
        session: &mut Session,
        mut context: Vec<LlmMessage>,
    ) -> Result<String> {
        let mut iteration: u32 = 0;
        let loop_start = std::time::Instant::now();
        let mut llm_time_ms: u128 = 0;
        let mut tool_time_ms: u128 = 0;

        loop {
            // Check max iterations
            if iteration >= self.max_iterations {
                tracing::warn!(
                    session_id = %session_id,
                    iterations = iteration,
                    "Max iterations reached, terminating loop"
                );
                return Err(AgentError::MaxIterationsReached(iteration));
            }

            iteration += 1;

            tracing::debug!(
                session_id = %session_id,
                iteration = iteration,
                "Agent loop iteration"
            );

            // Get available tools
            let tools = self.tool_registry.get_tool_definitions().await;

            // Time the LLM call
            let llm_start = std::time::Instant::now();

            // Call LLM
            let llm_response = self.call_llm_with_retry(&context, &tools).await?;
            let llm_elapsed = llm_start.elapsed().as_millis();
            llm_time_ms += llm_elapsed;

            tracing::trace!(
                session_id = %session_id,
                iteration = iteration,
                llm_ms = llm_elapsed,
                "LLM call completed"
            );

            // Check if we have tool calls
            if let Some(tool_calls) = llm_response.tool_calls.clone() {
                tracing::info!(
                    session_id = %session_id,
                    tool_count = tool_calls.len(),
                    "LLM requested tool executions"
                );

                // Add assistant message WITH tool_calls to session
                let session_tool_calls: Vec<crate::session::ToolCall> = tool_calls
                    .iter()
                    .map(|tc| crate::session::ToolCall {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    })
                    .collect();

                let assistant_message = crate::session::Message::new(
                    "assistant".to_string(),
                    llm_response.content.clone(),
                )
                .with_tool_calls(session_tool_calls);
                session.add_message(assistant_message);

                // Execute tools with timing
                let tool_start = std::time::Instant::now();
                let tool_results = self.execute_tools(tool_calls).await;
                let tool_elapsed = tool_start.elapsed().as_millis();
                tool_time_ms += tool_elapsed;

                tracing::trace!(
                    session_id = %session_id,
                    iteration = iteration,
                    tool_ms = tool_elapsed,
                    "Tool execution completed"
                );

                // Add tool results to context AND session
                for (tool_id, result) in tool_results {
                    // Add to LLM context
                    context.push(LlmMessage {
                        role: LlmRole::Tool,
                        content: format!("Tool {} result: {}", tool_id, result),
                        tool_calls: None,
                        tool_call_id: None,
                    });

                    // Add to session as tool_result message
                    let tool_result_message = crate::session::Message::tool_result(format!(
                        "Tool {} result: {}",
                        tool_id, result
                    ));
                    session.add_message(tool_result_message);
                }

                // Continue to next iteration
                continue;
            } else {
                // Text-only response - we're done
                let total_time_ms = loop_start.elapsed().as_millis();

                tracing::info!(
                    session_id = %session_id,
                    iterations = iteration,
                    total_ms = total_time_ms,
                    llm_ms = llm_time_ms,
                    tool_ms = tool_time_ms,
                    "Agent loop complete with text response"
                );

                // Add assistant message to session (no tool calls)
                let assistant_message = crate::session::Message::new(
                    "assistant".to_string(),
                    llm_response.content.clone(),
                );
                session.add_message(assistant_message);

                // Save session changes
                self.save_session(session).await?;

                return Ok(llm_response.content);
            }
        }
    }

    /// Calls the LLM with exponential backoff retry logic
    async fn call_llm_with_retry(
        &self,
        context: &[LlmMessage],
        tools: &[serde_json::Value],
    ) -> Result<LlmResponse> {
        let mut retry_count = 0;
        let mut delay_ms = 1000u64;

        loop {
            match self
                .llm_provider
                .chat(context.to_vec(), tools.to_vec(), &self.model)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if retry_count >= MAX_LLM_RETRIES {
                        tracing::error!(
                            retries = retry_count,
                            error = %e,
                            "LLM call failed after all retries"
                        );
                        return Err(AgentError::LlmError(e.to_string()));
                    }

                    retry_count += 1;
                    tracing::warn!(
                        retry = retry_count,
                        delay_ms = delay_ms,
                        error = %e,
                        "LLM call failed, retrying with backoff"
                    );

                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Executes a batch of tool calls in parallel
    /// Returns tuples of (tool_call_id, result_message) for proper correlation
    async fn execute_tools(&self, tool_calls: Vec<LlmToolCall>) -> Vec<(String, String)> {
        use futures::stream::{FuturesUnordered, StreamExt};

        let mut futures = FuturesUnordered::new();

        for tool_call in tool_calls {
            let tool_registry = Arc::clone(&self.tool_registry);
            let tool_call_id = tool_call.id.clone();

            futures.push(async move {
                let tool_name = tool_call.name.clone();

                match Self::execute_single_tool(tool_call, &tool_registry).await {
                    Ok(result) => {
                        tracing::info!(tool = %tool_name, tool_id = %tool_call_id, "Tool executed successfully");
                        (tool_call_id, result)
                    }
                    Err(e) => {
                        tracing::error!(tool = %tool_name, tool_id = %tool_call_id, error = %e, "Tool execution failed");
                        (tool_call_id, format!("Error executing tool '{}': {}", tool_name, e))
                    }
                }
            });
        }

        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            results.push(result);
        }

        results
    }

    /// Executes a single tool call
    async fn execute_single_tool(
        tool_call: LlmToolCall,
        tool_registry: &ToolRegistry,
    ) -> Result<String> {
        // Parse arguments from JSON string
        let args: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&tool_call.arguments).map_err(|e| {
                AgentError::ToolExecutionError(format!("Failed to parse tool arguments: {}", e))
            })?;

        // Execute the tool (includes validation and timeout)
        let ctx = crate::agent::tools::ToolExecutionContext {
            channel: None,
            chat_id: None,
        };

        tool_registry
            .execute_tool(&tool_call.name, args, &ctx)
            .await
            .map_err(|e| AgentError::ToolExecutionError(e.to_string()))
    }

    /// Saves the session to persistent storage
    async fn save_session(&self, session: &Session) -> Result<()> {
        let session_manager = self.session_manager.read().await;

        // Update the session in the manager with our modified version
        // Note: This clones the entire session (potentially 50 messages). For high-throughput
        // scenarios, consider using Arc<Session> or implementing a dirty-flag mechanism
        // to avoid repeated cloning. Current implementation prioritizes simplicity.
        session_manager
            .update_session(session.clone())
            .await
            .map_err(|e| AgentError::SessionError(e.to_string()))?;

        // Also trigger immediate persistence for this session
        // Note: Persistence failures are logged but don't fail the request to avoid data loss
        // in the in-memory session state. The auto-persistence task will retry on next cycle.
        if let Err(e) = session_manager.persist_session(session).await {
            tracing::error!(
                session_id = %session.session_id,
                error = %e,
                "Failed to persist session to disk - will retry on next auto-persistence cycle"
            );
            // TODO: Track failed persistence attempts for monitoring/alerting
        }

        Ok(())
    }

    /// Runs the agent loop continuously, processing messages from the chat hub
    ///
    /// This method is designed to run as a background task and will:
    /// - Listen for inbound messages from ChatHub
    /// - Process each message through the agent loop
    /// - Send responses back through ChatHub
    /// - Handle graceful shutdown on SIGTERM
    pub async fn run(&self) -> Result<()> {
        let mut shutdown_signal = std::pin::pin!(tokio::signal::ctrl_c());

        // Extract the inbound receiver if configured
        let mut inbound_rx = match self.inbound_rx.lock().unwrap().take() {
            Some(rx) => {
                tracing::info!("Agent loop started, processing messages");
                rx
            }
            None => {
                tracing::error!("AgentLoop started without inbound receiver - cannot process messages");
                return Err(AgentError::ChatHubError(
                    "No inbound receiver configured".to_string()
                ));
            }
        };

        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = &mut shutdown_signal => {
                    tracing::info!("Received shutdown signal, stopping agent loop");
                    break;
                }

                // Receive and process messages
                Some(msg) = inbound_rx.recv() => {
                    tracing::debug!(
                        channel = %msg.channel,
                        chat_id = %msg.chat_id,
                        "Processing inbound message"
                    );

                    // Process the message
                    match self.process_message(msg.clone()).await {
                        Ok(response) => {
                            // Send response back via chat_hub
                            if let Err(e) = self.chat_hub.reply(
                                &msg.channel,
                                &msg.chat_id,
                                response
                            ).await {
                                tracing::error!(
                                    channel = %msg.channel,
                                    chat_id = %msg.chat_id,
                                    error = %e,
                                    "Failed to send response"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                channel = %msg.channel,
                                chat_id = %msg.chat_id,
                                error = %e,
                                "Message processing failed"
                            );
                            // Continue processing other messages (graceful degradation)
                        }
                    }
                }

                // Channel closed - exit loop
                else => {
                    tracing::warn!("Inbound channel closed, stopping agent loop");
                    break;
                }
            }
        }

        tracing::info!("Agent loop stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ProviderError;

    // Mock implementations for testing
    struct MockLlmProvider;

    #[async_trait::async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _tools: Vec<serde_json::Value>,
            _model: &str,
        ) -> std::result::Result<LlmResponse, ProviderError> {
            Ok(LlmResponse {
                content: "Test response".to_string(),
                tool_calls: None,
                prompt_tokens: None,
                completion_tokens: None,
            })
        }

        fn default_model(&self) -> String {
            "test-model".to_string()
        }

        fn provider_name(&self) -> &'static str {
            "MockLlmProvider"
        }

        async fn list_models(
            &self,
        ) -> std::result::Result<Vec<crate::providers::ModelInfo>, ProviderError> {
            Ok(vec![crate::providers::ModelInfo::new("test-model", false)])
        }
    }

    struct MockContextBuilder;

    #[async_trait::async_trait]
    impl ContextBuilder for MockContextBuilder {
        async fn build_context(
            &self,
            _session: &Session,
            _current_message: &InboundMessage,
        ) -> Result<Vec<LlmMessage>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_agent_loop_creation() {
        let chat_hub = Arc::new(ChatHub::new());
        let llm_provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider);
        let context_builder: Arc<dyn ContextBuilder> = Arc::new(MockContextBuilder);
        let tool_registry = Arc::new(ToolRegistry::new());
        let session_manager = Arc::new(RwLock::new(SessionManager::new(std::path::PathBuf::from(
            "/tmp/sessions",
        ))));

        let agent = AgentLoop::new(
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
        );

        assert_eq!(agent.max_iterations(), MAX_ITERATIONS);
        assert_eq!(agent.model(), "test-model");
    }

    #[test]
    fn test_agent_loop_with_model_override() {
        let chat_hub = Arc::new(ChatHub::new());
        let llm_provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider);
        let context_builder: Arc<dyn ContextBuilder> = Arc::new(MockContextBuilder);
        let tool_registry = Arc::new(ToolRegistry::new());
        let session_manager = Arc::new(RwLock::new(SessionManager::new(std::path::PathBuf::from(
            "/tmp/sessions",
        ))));

        let agent = AgentLoop::with_model(
            chat_hub,
            llm_provider,
            context_builder,
            tool_registry,
            session_manager,
            "custom-model",
        );

        assert_eq!(agent.model(), "custom-model");
    }

    #[test]
    fn test_max_iterations_constant() {
        assert_eq!(MAX_ITERATIONS, 200);
    }

    #[test]
    fn test_llm_role_as_str() {
        assert_eq!(LlmRole::System.as_str(), "system");
        assert_eq!(LlmRole::User.as_str(), "user");
        assert_eq!(LlmRole::Assistant.as_str(), "assistant");
        assert_eq!(LlmRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_llm_tool_call_creation() {
        let tool_call = LlmToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"key": "value"}"#.to_string(),
        };

        assert_eq!(tool_call.id, "call_123");
        assert_eq!(tool_call.name, "test_tool");
        assert_eq!(tool_call.arguments, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::MaxIterationsReached(200);
        assert_eq!(err.to_string(), "Max iterations (200) reached");

        let err = AgentError::ToolExecutionError("test error".to_string());
        assert_eq!(err.to_string(), "Tool execution failed: test error");
    }

    // Session management tests
    #[test]
    fn test_session_lifecycle_message_ordering() {
        // Test that messages are added in correct order
        let mut session = Session::new("telegram".to_string(), "123".to_string());

        // Add user message
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "Hello".to_string(),
        ));

        // Add assistant message
        session.add_message(crate::session::Message::new(
            "assistant".to_string(),
            "Hi there".to_string(),
        ));

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
    }

    #[test]
    fn test_session_with_tool_calls() {
        let mut session = Session::new("telegram".to_string(), "123".to_string());

        // Add user message
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "List files".to_string(),
        ));

        // Add assistant message with tool calls
        let tool_calls = vec![crate::session::ToolCall {
            id: "call_1".to_string(),
            name: "filesystem".to_string(),
            arguments: "{\"operation\": \"list\", \"path\": \"/tmp\"}".to_string(),
        }];

        let assistant_msg = crate::session::Message::new(
            "assistant".to_string(),
            "I'll list the files for you.".to_string(),
        )
        .with_tool_calls(tool_calls);

        session.add_message(assistant_msg);

        // Verify assistant message has tool_calls
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[1].role, "assistant");
        assert!(session.messages[1].tool_calls.is_some());
        assert_eq!(session.messages[1].tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(
            session.messages[1].tool_calls.as_ref().unwrap()[0].name,
            "filesystem"
        );
    }

    #[test]
    fn test_session_tool_result_messages() {
        let mut session = Session::new("telegram".to_string(), "123".to_string());

        // Add tool result message
        session.add_message(crate::session::Message::tool_result(
            "Tool call_1 result: file1.txt, file2.txt".to_string(),
        ));

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "tool_result");
        assert!(session.messages[0].content.contains("file1.txt"));
    }

    #[test]
    fn test_session_complete_tool_interaction_flow() {
        let mut session = Session::new("telegram".to_string(), "123".to_string());

        // Complete flow: user -> assistant (with tool_calls) -> tool_result
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "What's the weather?".to_string(),
        ));

        let tool_calls = vec![crate::session::ToolCall {
            id: "call_weather_1".to_string(),
            name: "weather".to_string(),
            arguments: "{\"city\": \"Paris\"}".to_string(),
        }];

        session.add_message(
            crate::session::Message::new(
                "assistant".to_string(),
                "Let me check the weather.".to_string(),
            )
            .with_tool_calls(tool_calls),
        );

        session.add_message(crate::session::Message::tool_result(
            "Tool call_weather_1 result: Sunny, 25°C".to_string(),
        ));

        session.add_message(crate::session::Message::new(
            "assistant".to_string(),
            "It's sunny and 25°C in Paris.".to_string(),
        ));

        assert_eq!(session.messages.len(), 4);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.messages[2].role, "tool_result");
        assert_eq!(session.messages[3].role, "assistant");

        // Verify tool_calls preserved
        assert!(session.messages[1].tool_calls.is_some());
        assert_eq!(
            session.messages[1].tool_calls.as_ref().unwrap()[0].id,
            "call_weather_1"
        );
    }

    #[test]
    fn test_session_fifo_with_tool_interactions() {
        // Test that FIFO works correctly with tool interaction messages
        let mut session = Session::new("telegram".to_string(), "123".to_string());

        // Add 50 messages with tool interactions
        for i in 0..25 {
            // User message
            session.add_message(crate::session::Message::new(
                "user".to_string(),
                format!("Message {}", i),
            ));

            // Assistant message
            session.add_message(crate::session::Message::new(
                "assistant".to_string(),
                format!("Response {}", i),
            ));
        }

        assert_eq!(session.messages.len(), 50);

        // Add one more message (should trigger FIFO)
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "Overflow message".to_string(),
        ));

        // Should still be 50, with oldest removed (Message 0 was removed)
        assert_eq!(session.messages.len(), 50);
        assert_eq!(session.messages[0].content, "Response 0"); // First message (Message 0) was removed, now starts with Response 0
        assert_eq!(session.messages[49].content, "Overflow message");
    }

    #[test]
    fn test_message_is_user_assistant_tool_result_helpers() {
        let user_msg = crate::session::Message::new("user".to_string(), "Hello".to_string());
        assert!(user_msg.is_user());
        assert!(!user_msg.is_assistant());
        assert!(!user_msg.is_tool_result());

        let assistant_msg = crate::session::Message::new("assistant".to_string(), "Hi".to_string());
        assert!(!assistant_msg.is_user());
        assert!(assistant_msg.is_assistant());
        assert!(!assistant_msg.is_tool_result());

        let tool_result_msg = crate::session::Message::tool_result("Result".to_string());
        assert!(!tool_result_msg.is_user());
        assert!(!tool_result_msg.is_assistant());
        assert!(tool_result_msg.is_tool_result());
    }
}
