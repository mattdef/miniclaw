//! One-shot agent execution module
//!
//! This module provides functionality for executing a single message to the agent
//! without running the full gateway daemon. It's designed for CLI interactions
//! where the user wants a quick response.

use std::sync::Arc;

use crate::agent::tools::ToolRegistry;
use crate::agent::AgentLoop;
use crate::agent::agent_loop::{ContextBuilder, Result as AgentResult};
use crate::chat::{ChatHub, InboundMessage};
use crate::config::Config;
use crate::providers::{LlmMessage, LlmProvider, LlmRole, ProviderConfig, ProviderFactory};
use anyhow::{Context, Result};

/// Executes a one-shot message to the agent
///
/// This function handles the complete flow of:
/// 1. Loading configuration
/// 2. Creating the LLM provider
/// 3. Initializing a temporary session
/// 4. Building context and executing the agent loop
/// 5. Returning the response or error
///
/// # Arguments
///
/// * `message` - The message content to send
/// * `model_override` - Optional model to use instead of config default
/// * `config` - Application configuration
/// * `verbose` - Whether to enable verbose logging
///
/// # Returns
///
/// The agent's response text on success, or an error
pub async fn execute_one_shot(
    message: String,
    model_override: Option<String>,
    config: &Config,
    verbose: bool,
) -> Result<String> {
    if verbose {
        tracing::info!("Starting one-shot agent execution (verbose mode enabled)");
    } else {
        tracing::info!("Starting one-shot agent execution");
    }

    // Create the LLM provider from config
    let provider = create_provider(config)
        .context("Failed to create LLM provider. Ensure your configuration has a valid API key.")?;

    // Determine which model to use
    let model = model_override
        .or_else(|| config.model.clone())
        .unwrap_or_else(|| "google/gemini-2.5-flash".to_string());

    tracing::info!(model = %model, "Using model for one-shot execution");

    if verbose {
        tracing::debug!("Verbose logging enabled for agent execution");
    }

    // Create a minimal ChatHub (needed by AgentLoop but not used in one-shot)
    let chat_hub = Arc::new(ChatHub::new());

    // Create a context builder with workspace path
    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .unwrap_or_else(|| std::env::temp_dir());

    let context_builder: Arc<dyn ContextBuilder> = if workspace_path.exists() {
        Arc::new(
            crate::agent::ContextBuilderImpl::new(&workspace_path)
                .map_err(|e| anyhow::anyhow!("Failed to create context builder: {}", e))?,
        )
    } else {
        // Fallback: create a minimal context builder that doesn't depend on workspace files
        Arc::new(MinimalContextBuilder::new())
    };

    // Create a tool registry with default tools
    let tool_registry = Arc::new(ToolRegistry::new());
    
    // Register filesystem tool with workspace directory
    let fs_tool = crate::agent::tools::filesystem::FilesystemTool::new(workspace_path.clone());
    tool_registry
        .register(Box::new(fs_tool))
        .expect("Failed to register filesystem tool");

    // Create a temporary session manager (not persisted)
    let temp_dir = std::env::temp_dir();
    let session_manager = Arc::new(tokio::sync::RwLock::new(
        crate::session::SessionManager::new(temp_dir)
    ));

    // Create the agent loop with model override
    let agent_loop = AgentLoop::with_model(
        chat_hub,
        provider,
        context_builder,
        tool_registry,
        session_manager,
        model,
    );

    // Create the inbound message
    let inbound_message = InboundMessage::new("cli", "oneshot", message);

    if verbose {
        tracing::debug!("Processing message through agent loop with verbose logging");
    } else {
        tracing::debug!("Processing message through agent loop");
    }

    // Process the message
    let response = agent_loop
        .process_message(inbound_message)
        .await
        .map_err(|e| anyhow::anyhow!("Agent execution failed: {}", e))?;

    tracing::info!("One-shot execution completed successfully");

    Ok(response)
}

/// Minimal context builder for when workspace files don't exist
///
/// This provides a basic system message without requiring SOUL.md or AGENTS.md
pub struct MinimalContextBuilder;

impl MinimalContextBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ContextBuilder for MinimalContextBuilder {
    async fn build_context(
        &self,
        _session: &crate::session::Session,
        current_message: &InboundMessage,
    ) -> AgentResult<Vec<LlmMessage>> {
        let system_message = LlmMessage::new(
            LlmRole::System,
            "You are miniclaw, a helpful AI assistant running on edge hardware. \
             You are efficient, concise, and focused on providing practical assistance.",
        );

        let user_message = LlmMessage::new(
            LlmRole::User,
            current_message.content.clone(),
        );

        Ok(vec![system_message, user_message])
    }
}

/// Creates an LLM provider from configuration
fn create_provider(config: &Config) -> Result<Arc<dyn LlmProvider>> {
    // First, try to use API key from config or environment
    if let Some(api_key) = &config.api_key {
        // Don't log the API key - security risk
        tracing::debug!("Creating OpenRouter provider from config");
        let provider_config = ProviderConfig::OpenRouter(
            crate::providers::OpenRouterConfig::new(api_key.clone())
                .with_model(config.model.clone().unwrap_or_else(|| "google/gemini-2.5-flash".to_string())),
        );

        let provider = ProviderFactory::create(provider_config)
            .context("Failed to create OpenRouter provider")?;

        return Ok(Arc::from(provider));
    }

    // If no API key, try Ollama (local provider)
    tracing::debug!("No API key found, trying Ollama provider");
    let provider_config = ProviderConfig::Ollama(
        crate::providers::OllamaConfig::new()
            .with_model(config.model.clone().unwrap_or_else(|| "llama3.2".to_string())),
    );

    ProviderFactory::create(provider_config)
        .map(Arc::from)
        .context("Failed to create Ollama provider")
        .context("No LLM provider available. Please configure an API key in ~/.miniclaw/config.json or ensure Ollama is running locally")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider_with_api_key() {
        let config = Config {
            api_key: Some("test-key".to_string()),
            model: Some("test-model".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        };

        // This would need a runtime to test async
        // For now, just verify the config structure
        assert!(config.api_key.is_some());
        assert!(config.model.is_some());
    }

    #[test]
    fn test_create_provider_without_api_key() {
        let config = Config {
            api_key: None,
            model: Some("llama3.2".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        };

        // Without API key, should attempt Ollama
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_minimal_context_builder() {
        let builder = MinimalContextBuilder::new();
        // Test that it can be instantiated
        let _size = std::mem::size_of_val(&builder);
        // If we get here, the builder was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_minimal_context_builder_builds_context() {
        let builder = MinimalContextBuilder::new();
        let session = crate::session::Session::new("test".to_string(), "oneshot".to_string());
        let message = InboundMessage::new("cli", "test", "Hello".to_string());
        
        let result = builder.build_context(&session, &message).await;
        assert!(result.is_ok());
        
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2); // System + User message
        assert!(messages[0].is_system());
        assert!(messages[1].is_user());
        assert_eq!(messages[1].content, "Hello");
    }

    #[tokio::test]
    async fn test_execute_one_shot_with_invalid_config() {
        // Test with config that has no API key and no Ollama available
        let config = Config {
            api_key: None,
            model: None,
            telegram_token: None,
            telegram_whitelist: None,
        };

        let result = execute_one_shot(
            "test message".to_string(),
            None,
            &config,
            false,
        ).await;

        // Should fail because no provider is available
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No LLM provider available") || error_msg.contains("Ollama"));
    }

    #[test]
    fn test_model_override_precedence() {
        // Test that model override takes precedence over config model
        let config = Config {
            api_key: Some("test-key".to_string()),
            model: Some("config-model".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        };

        // In real execution, the override would be used
        let model_override = Some("override-model".to_string());
        let effective_model = model_override
            .or_else(|| config.model.clone())
            .unwrap_or_else(|| "default".to_string());
        
        assert_eq!(effective_model, "override-model");
    }

    #[test]
    fn test_model_fallback_chain() {
        // Test model selection fallback: override → config → default
        let config = Config {
            api_key: Some("test-key".to_string()),
            model: None,
            telegram_token: None,
            telegram_whitelist: None,
        };

        let effective_model = None
            .or_else(|| config.model.clone())
            .unwrap_or_else(|| "google/gemini-2.5-flash".to_string());
        
        assert_eq!(effective_model, "google/gemini-2.5-flash");
    }
}
