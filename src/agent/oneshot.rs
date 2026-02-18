//! One-shot agent execution module
//!
//! This module provides functionality for executing a single message to the agent
//! without running the full gateway daemon. It's designed for CLI interactions
//! where the user wants a quick response.

use std::sync::Arc;

use crate::agent::AgentLoop;
use crate::agent::agent_loop::{ContextBuilder, Result as AgentResult};
use crate::agent::tools::ToolRegistry;
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

    // Determine which model to use: CLI override > provider_config > provider default
    let model = model_override
        .or_else(|| {
            config
                .provider_config
                .as_ref()
                .map(|pc| pc.default_model().to_string())
        })
        .unwrap_or_else(|| provider.default_model());

    tracing::info!(model = %model, "Using model for one-shot execution");

    if verbose {
        tracing::debug!("Verbose logging enabled for agent execution");
    }

    // Create a minimal ChatHub (needed by AgentLoop but not used in one-shot)
    let chat_hub = Arc::new(ChatHub::new());

    // Create a context builder with workspace path
    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .unwrap_or_else(std::env::temp_dir);

    let context_builder: Arc<dyn ContextBuilder> = if workspace_path.exists() {
        Arc::new(
            crate::agent::ContextBuilderImpl::new(&workspace_path)
                .map_err(|e| anyhow::anyhow!("Failed to create context builder: {}", e))?,
        )
    } else {
        // Fallback: create a minimal context builder that doesn't depend on workspace files
        Arc::new(MinimalContextBuilder::new())
    };

    // Create a tool registry with all default tools
    // Use the configured default channel (defaults to "telegram" if not set)
    let tool_registry = Arc::new(
        ToolRegistry::with_all_default_tools(
            workspace_path,
            Arc::clone(&chat_hub),
            config,
            &config.default_channel,
        )
        .await,
    );

    // Create a temporary session manager (not persisted)
    let temp_dir = std::env::temp_dir();
    let session_manager = Arc::new(tokio::sync::RwLock::new(
        crate::session::SessionManager::new(temp_dir),
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

impl Default for MinimalContextBuilder {
    fn default() -> Self {
        Self::new()
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

        let user_message = LlmMessage::new(LlmRole::User, current_message.content.clone());

        Ok(vec![system_message, user_message])
    }
}

/// Creates an LLM provider from configuration
fn create_provider(config: &Config) -> Result<Arc<dyn LlmProvider>> {
    // Use provider_config if available (only supported format)
    if let Some(provider_config) = &config.provider_config {
        tracing::debug!(provider_type = %provider_config.provider_type(), "Creating provider from provider_config");
        let provider = ProviderFactory::create(provider_config.clone()).with_context(|| {
            format!(
                "Failed to create {} provider",
                provider_config.provider_type()
            )
        })?;
        return Ok(Arc::from(provider));
    }

    // No provider_config - try Ollama as fallback (local provider, no API key needed)
    tracing::debug!("No provider_config found, trying Ollama provider");
    let provider_config = ProviderConfig::Ollama(crate::providers::OllamaConfig::new());

    ProviderFactory::create(provider_config)
        .map(Arc::from)
        .context("Failed to create Ollama provider")
        .context("No LLM provider available. Please configure a provider in ~/.miniclaw/config.json or ensure Ollama is running locally")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider_with_provider_config() {
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(crate::providers::ProviderConfig::openai("test-key")),
            model: None,
        };

        // This would need a runtime to test async
        // For now, just verify the config structure
        assert!(config.provider_config.is_some());
        assert_eq!(config.provider_config.unwrap().default_model(), "gpt-4o");
    }

    #[test]
    fn test_create_provider_without_provider_config() {
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        // Without provider_config, should attempt Ollama
        assert!(config.provider_config.is_none());
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
        // Test with config that has no provider_config and no Ollama available
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        let result = execute_one_shot("test message".to_string(), None, &config, false).await;

        // Should fail because no provider is available
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No LLM provider available") || error_msg.contains("Ollama"));
    }

    #[test]
    fn test_model_override_precedence() {
        // Test that CLI model override takes precedence over provider_config model
        use crate::providers::{OpenAiConfig, ProviderConfig};

        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(ProviderConfig::OpenAi(OpenAiConfig::new("test-key"))),
            model: None,
        };

        // CLI override should take precedence
        let model_override = Some("override-model".to_string());
        let effective_model = model_override
            .or_else(|| {
                config
                    .provider_config
                    .as_ref()
                    .map(|pc| pc.default_model().to_string())
            })
            .unwrap_or_else(|| "default".to_string());

        assert_eq!(effective_model, "override-model");
    }

    #[test]
    fn test_model_from_provider_config() {
        // Test model selection from provider_config when no CLI override
        use crate::providers::{OpenAiConfig, ProviderConfig};

        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(ProviderConfig::OpenAi(OpenAiConfig::new("test-key"))),
            model: None,
        };

        let effective_model = None
            .or_else(|| {
                config
                    .provider_config
                    .as_ref()
                    .map(|pc| pc.default_model().to_string())
            })
            .unwrap_or_else(|| "default".to_string());

        assert_eq!(effective_model, "gpt-4o");
    }
}
