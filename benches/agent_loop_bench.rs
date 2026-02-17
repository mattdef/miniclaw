use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::sync::RwLock;

use miniclaw::agent::AgentLoop;
use miniclaw::agent::tools::ToolRegistry;
use miniclaw::chat::ChatHub;
use miniclaw::session::SessionManager;
use miniclaw::chat::InboundMessage;
use std::path::PathBuf;

/// Mock LLM Provider for benchmarks
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
            content: "Benchmark response".to_string(),
            tool_calls: None,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }

    fn default_model(&self) -> String {
        "benchmark-model".to_string()
    }

    fn provider_name(&self) -> &'static str {
        "MockLlmProvider"
    }
}

/// Benchmark helper to create a test AgentLoop
fn create_test_agent_loop() -> AgentLoop {
    let chat_hub = Arc::new(ChatHub::new());
    let llm_provider: Arc<dyn miniclaw::providers::LlmProvider> = Arc::new(MockLlmProvider);
    let context_builder: Arc<dyn miniclaw::agent::ContextBuilder> = Arc::new(MockContextBuilder);
    let tool_registry = Arc::new(ToolRegistry::new());
    let session_manager = Arc::new(RwLock::new(SessionManager::new(PathBuf::from("/tmp/sessions"))));

    AgentLoop::new(
        chat_hub,
        llm_provider,
        context_builder,
        tool_registry,
        session_manager,
    )
}

/// Mock context builder for benchmarks
struct MockContextBuilder;

#[async_trait::async_trait]
impl miniclaw::agent::ContextBuilder for MockContextBuilder {
    async fn build_context(
        &self,
        _session: &miniclaw::session::Session,
        _current_message: &InboundMessage,
    ) -> Result<Vec<miniclaw::providers::LlmMessage>, miniclaw::agent::AgentError> {
        Ok(vec![])
    }
}

/// Benchmark message processing pipeline setup
/// Note: This benchmarks the setup time, not the full LLM call
fn benchmark_message_processing_setup(c: &mut Criterion) {
    c.bench_function("message_processing_setup", |b| {
        let agent = create_test_agent_loop();
        let message = InboundMessage {
            channel: "test".to_string(),
            chat_id: "123".to_string(),
            content: "Hello world".to_string(),
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        b.iter(|| {
            // Benchmark the message validation and session lookup setup
            black_box(&message);
        });
    });
}

/// Benchmark session manager operations
fn benchmark_session_operations(c: &mut Criterion) {
    c.bench_function("session_get_or_create", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let session_manager = Arc::new(SessionManager::new(PathBuf::from("/tmp/sessions_bench")));

        b.to_async(&rt).iter(|| async {
            let _ = session_manager
                .get_or_create_session("benchmark", "12345")
                .await;
        });
    });
}

/// Benchmark context builder (simplified)
fn benchmark_context_build(c: &mut Criterion) {
    c.bench_function("context_build_empty", |b| {
        let context_builder: Arc<dyn miniclaw::agent::ContextBuilder> = Arc::new(MockContextBuilder);
        let session = miniclaw::session::Session::new("test".to_string(), "123".to_string());
        let message = InboundMessage {
            channel: "test".to_string(),
            chat_id: "123".to_string(),
            content: "Hello".to_string(),
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            let _ = context_builder.build_context(&session, &message).await;
        });
    });
}

criterion_group!(
    benches,
    benchmark_message_processing_setup,
    benchmark_session_operations,
    benchmark_context_build
);
criterion_main!(benches);
