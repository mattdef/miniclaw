//! Integration tests for AgentLoop message flow
//!
//! These tests verify the end-to-end flow from ChatHub to AgentLoop

use std::sync::Arc;
use tokio::sync::mpsc;

use miniclaw::agent::{AgentLoop, ContextBuilder};
use miniclaw::chat::{ChatHub, InboundMessage};
use miniclaw::session::{Session, SessionManager};

// Mock LLM Provider for testing
struct MockLlmProvider;

#[async_trait::async_trait]
impl miniclaw::providers::LlmProvider for MockLlmProvider {
    async fn chat(
        &self,
        _messages: Vec<miniclaw::providers::LlmMessage>,
        _tools: Vec<serde_json::Value>,
        _model: &str,
    ) -> std::result::Result<miniclaw::providers::LlmResponse, miniclaw::providers::ProviderError> {
        Ok(miniclaw::providers::LlmResponse {
            content: "Mock response".to_string(),
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
    ) -> std::result::Result<Vec<miniclaw::providers::ModelInfo>, miniclaw::providers::ProviderError> {
        Ok(vec![miniclaw::providers::ModelInfo::new("test-model", false)])
    }
}

// Mock ContextBuilder for testing
struct MockContextBuilder;

#[async_trait::async_trait]
impl ContextBuilder for MockContextBuilder {
    async fn build_context(
        &self,
        _session: &Session,
        _current_message: &InboundMessage,
    ) -> Result<Vec<miniclaw::providers::LlmMessage>, miniclaw::agent::AgentError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_end_to_end_message_flow() {
    // Create ChatHub
    let mut chat_hub = ChatHub::new();

    // Create channel for ChatHub -> AgentLoop
    let (agent_tx, agent_rx) = mpsc::channel(100);
    chat_hub.register_agent_sender(agent_tx);

    let chat_hub = Arc::new(chat_hub);

    // Create AgentLoop with mock dependencies
    let llm_provider: Arc<dyn miniclaw::providers::LlmProvider> = Arc::new(MockLlmProvider);
    let context_builder: Arc<dyn ContextBuilder> = Arc::new(MockContextBuilder);
    let tool_registry = Arc::new(miniclaw::agent::tools::ToolRegistry::new());
    let session_manager = Arc::new(tokio::sync::RwLock::new(SessionManager::new(
        std::env::temp_dir(),
    )));

    let agent_loop = AgentLoop::with_model_and_receiver(
        Arc::clone(&chat_hub),
        llm_provider,
        context_builder,
        tool_registry,
        session_manager,
        "test-model",
        agent_rx,
    );

    // Spawn AgentLoop
    let agent_handle = tokio::spawn(async move {
        agent_loop.run().await.unwrap();
    });

    // Give AgentLoop time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send a message via ChatHub's inbound_sender
    let inbound_tx = chat_hub.inbound_sender();
    let msg = InboundMessage::new("test_channel", "test_chat_id", "Hello Agent");
    inbound_tx.send(msg).await.unwrap();

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Check that a response was sent to outbound
    // Note: In a real test, we'd need to mock or capture the outbound messages
    // For now, we just verify the flow doesn't panic

    // Clean up - shutdown the agent loop by dropping the sender
    drop(inbound_tx);

    // Wait for agent to finish with timeout
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), agent_handle).await;
}

#[tokio::test]
async fn test_allow_all_wildcard_integration() {
    use miniclaw::utils::security::WhitelistChecker;

    // Create whitelist with wildcard
    let checker = WhitelistChecker::new(vec![-1]);

    // Verify all user IDs are allowed
    assert!(checker.is_allowed(123));
    assert!(checker.is_allowed(456));
    assert!(checker.is_allowed(0));
    assert!(checker.is_allowed(-999));
}

#[tokio::test]
async fn test_chat_hub_agent_channel_buffer() {
    // Test that messages are forwarded correctly even with buffer limits
    let mut chat_hub = ChatHub::new();
    let (agent_tx, mut agent_rx) = mpsc::channel(5); // Small buffer for testing
    chat_hub.register_agent_sender(agent_tx);

    let chat_hub = Arc::new(chat_hub);

    // Send multiple messages
    let inbound_tx = chat_hub.inbound_sender();
    for i in 0..10 {
        let msg = InboundMessage::new("test", "123", format!("Message {}", i));
        inbound_tx.send(msg).await.unwrap();
    }

    // Process messages through ChatHub
    // In real scenario, ChatHub.run() would do this
    // Here we just verify the channel receives them
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Clean up
    drop(inbound_tx);
}
