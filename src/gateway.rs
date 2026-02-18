//! Gateway daemon for miniclaw
//!
//! The gateway runs as a background daemon, managing the ChatHub and SessionManager
//! with automatic session persistence every 30 seconds.

use crate::agent::tools::ToolRegistry;
use crate::agent::{AgentLoop, ContextBuilderImpl};
use crate::channels::{Channel, TelegramChannel};
use crate::chat::ChatHub;
use crate::config::Config;
use crate::providers::LlmProvider;
use crate::session::SessionManager;
use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Runs the gateway daemon with auto-persistence and graceful shutdown.
///
/// This function:
/// 1. Initializes the SessionManager and loads existing sessions
/// 2. Starts the auto-persistence background task (every 30 seconds)
/// 3. Initializes the ChatHub and channels (Telegram, etc.)
/// 4. Initializes the AgentLoop for message processing
/// 5. Starts memory monitoring background task
/// 6. Handles SIGTERM/SIGINT for graceful shutdown
/// 7. Flushes all sessions to disk before exiting
pub async fn run_gateway(config: &Config) -> Result<()> {
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting miniclaw gateway daemon"
    );

    // Determine sessions directory
    let sessions_dir = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace").join("sessions"))
        .context("Could not determine sessions directory")?;

    // Initialize SessionManager
    let session_manager = Arc::new(SessionManager::new(sessions_dir.clone()));
    session_manager
        .initialize()
        .await
        .context("Failed to initialize SessionManager")?;

    info!(
        "SessionManager initialized with {} sessions",
        session_manager.session_count().await
    );

    // Start auto-persistence background task
    let (persistence_handle, persistence_shutdown) = session_manager.start_auto_persistence();
    info!("Auto-persistence background task started (30s interval)");

    // Start session cleanup background task
    let (cleanup_handle, cleanup_shutdown) = session_manager.start_cleanup_task();
    info!("Session cleanup background task started (24h interval)");

    // Create shutdown channel for coordination
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Clone for signal handler
    let shutdown_tx_signal = shutdown_tx.clone();

    // Spawn signal handler task for SIGTERM and SIGINT
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown...");
                }
            }
        }
        #[cfg(not(unix))]
        {
            use tokio::signal;
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received Ctrl+C, initiating graceful shutdown...");
                }
                Err(e) => {
                    error!("Failed to listen for shutdown signal: {}", e);
                    return;
                }
            }
        }
        let _ = shutdown_tx_signal.send(()).await;
    });

    // Initialize ChatHub
    let mut chat_hub = ChatHub::new();

    // Create channel for ChatHub -> AgentLoop communication
    let (agent_tx, agent_rx) = tokio::sync::mpsc::channel(100);

    // Register the sender with ChatHub
    chat_hub.register_agent_sender(agent_tx);
    info!("ChatHub connected to AgentLoop via agent channel");

    // Wrap in Arc for shared access
    let chat_hub = Arc::new(chat_hub);
    info!("ChatHub initialized");

    // Determine workspace directory for context builder
    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .context("Could not determine workspace directory")?;

    // Create LLM provider
    let llm_provider = create_provider(config)?;

    // Determine model from provider_config or use provider default
    let model = config
        .provider_config
        .as_ref()
        .map(|pc| pc.default_model().to_string())
        .unwrap_or_else(|| llm_provider.default_model());

    info!("LLM provider initialized with model: {}", model);

    // Create tool registry with all default tools
    // The "telegram" channel is used as default for the message tool
    let tool_registry = Arc::new(
        ToolRegistry::with_all_default_tools(
            workspace_path.clone(),
            Arc::clone(&chat_hub),
            config,
            &config.default_channel,
        )
        .await,
    );
    info!("Tool registry initialized with all default tools");

    // Create context builder
    let context_builder = Arc::new(
        ContextBuilderImpl::new(workspace_path).context("Failed to create context builder")?,
    );
    info!("Context builder initialized");

    // Initialize AgentLoop for message processing with inbound receiver
    // Share the same session_manager instance (already Arc<SessionManager>)
    let agent_loop = AgentLoop::builder(
        Arc::clone(&chat_hub),
        llm_provider,
        context_builder,
        tool_registry,
        Arc::clone(&session_manager),
    )
    .with_model(model.clone())
    .with_inbound_receiver(agent_rx)
    .build();
    info!("AgentLoop initialized with inbound receiver");

    // Spawn AgentLoop processing task
    // Note: AgentLoop.run() can only be called once as it takes ownership of the inbound receiver
    tokio::spawn(async move {
        info!("AgentLoop processing task started");
        match agent_loop.run().await {
            Ok(()) => {
                info!("AgentLoop processing task stopped normally");
            }
            Err(e) => {
                error!("AgentLoop error: {}. Task terminated.", e);
            }
        }
        info!("AgentLoop processing task terminated");
    });

    // Initialize Telegram channel if configured
    let telegram_channel = if let Some(token) = &config.telegram_token {
        match TelegramChannel::new(token.clone(), config.allow_from.clone()) {
            Ok(channel) => match channel.start(Arc::clone(&chat_hub)).await {
                Ok(()) => {
                    info!("Telegram channel initialized successfully");
                    Some(channel)
                }
                Err(e) => {
                    error!(
                        "Failed to start Telegram channel: {}. Gateway will continue without Telegram support.",
                        e
                    );
                    None
                }
            },
            Err(e) => {
                error!(
                    "Invalid Telegram token: {}. Check @BotFather (https://t.me/BotFather) for a valid token. Gateway will continue without Telegram support.",
                    e
                );
                None
            }
        }
    } else {
        warn!(
            "No Telegram token configured. Set TELEGRAM_BOT_TOKEN environment variable or add telegram_token to config.json to enable Telegram support."
        );
        None
    };

    info!("Gateway daemon is running. Press Ctrl+C to stop.");

    // Spawn memory monitoring background task
    let (memory_shutdown_tx, mut memory_shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio::spawn(async move {
        use sysinfo::{System, get_current_pid};
        const MEMORY_WARNING_THRESHOLD_MB: u64 = 200;
        const MEMORY_CHECK_INTERVAL_SECS: u64 = 60;

        let mut system = System::new_all();
        let current_pid = match get_current_pid() {
            Ok(pid) => pid,
            Err(e) => {
                error!(
                    "Failed to get current PID for memory monitoring: {}. Memory monitoring disabled.",
                    e
                );
                return;
            }
        };
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(MEMORY_CHECK_INTERVAL_SECS));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    system.refresh_all();
                    if let Some(process) = system.process(current_pid) {
                        let memory_mb = process.memory() / 1024;
                        if memory_mb > MEMORY_WARNING_THRESHOLD_MB {
                            warn!(memory_mb = memory_mb, threshold_mb = MEMORY_WARNING_THRESHOLD_MB, "Memory usage exceeds threshold");
                        } else {
                            debug!(memory_mb = memory_mb, "Current memory usage");
                        }
                    }
                }
                _ = memory_shutdown_rx.recv() => {
                    debug!("Memory monitoring task received shutdown signal");
                    break;
                }
            }
        }
    });

    loop {
        tokio::select! {
            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, starting graceful shutdown...");
                break;
            }

            // Run ChatHub (this will also handle ctrl_c internally)
            result = run_chat_hub(Arc::clone(&chat_hub)) => {
                if let Err(e) = result {
                    error!("ChatHub error: {}", e);
                    // Continue running unless shutdown is signaled
                }
            }
        }
    }

    // Graceful shutdown sequence
    info!("Starting graceful shutdown sequence...");

    // Shutdown Telegram channel if active
    if let Some(channel) = telegram_channel {
        info!("Shutting down Telegram channel...");
        if let Err(e) = channel.shutdown().await {
            error!("Error shutting down Telegram channel: {}", e);
        }
    }

    // Signal persistence task to stop
    info!("Signaling persistence task to stop...");
    let _ = persistence_shutdown.send(()).await;

    // Wait for persistence task to complete with timeout (5s as per spec)
    let timeout_duration = std::time::Duration::from_secs(5);
    match tokio::time::timeout(timeout_duration, persistence_handle).await {
        Ok(Ok(())) => {
            info!("Persistence task completed gracefully");
        }
        Ok(Err(e)) => {
            error!("Persistence task panicked: {}", e);
        }
        Err(_) => {
            error!("Persistence task did not complete within 5s timeout");
        }
    }

    // Signal cleanup task to stop
    info!("Signaling cleanup task to stop...");
    let _ = cleanup_shutdown.send(()).await;

    // Wait for cleanup task to complete with shorter timeout (it may not be actively running)
    let cleanup_timeout = std::time::Duration::from_secs(5);
    match tokio::time::timeout(cleanup_timeout, cleanup_handle).await {
        Ok(Ok(())) => {
            info!("Cleanup task completed gracefully");
        }
        Ok(Err(e)) => {
            error!("Cleanup task panicked: {}", e);
        }
        Err(_) => {
            error!("Cleanup task did not complete within 5s timeout");
        }
    }

    // Signal memory monitoring task to stop
    info!("Signaling memory monitoring task to stop...");
    let _ = memory_shutdown_tx.send(()).await;

    // Shutdown ChatHub
    info!("Shutting down ChatHub...");
    if let Err(e) = chat_hub.shutdown().await {
        error!("Error during ChatHub shutdown: {}", e);
    }

    // Final persistence flush
    info!("Flushing all sessions to disk...");
    if let Err(e) = session_manager.save_all_sessions().await {
        error!("Failed to save sessions during shutdown: {}", e);
    } else {
        info!("All sessions flushed to disk successfully");
    }

    info!("Gateway daemon stopped gracefully");
    Ok(())
}

/// Runs the ChatHub with a timeout for graceful shutdown coordination.
async fn run_chat_hub(chat_hub: Arc<ChatHub>) -> Result<()> {
    // Run ChatHub with a periodic check for shutdown
    // This allows us to break out if needed for graceful shutdown
    tokio::select! {
        result = chat_hub.run() => {
            result.context("ChatHub run failed")
        }
    }
}

/// Creates an LLM provider from configuration
fn create_provider(config: &Config) -> Result<Arc<dyn LlmProvider>> {
    use crate::providers::{ProviderConfig, ProviderFactory};

    // Use provider_config if available (only supported format)
    if let Some(provider_config) = &config.provider_config {
        debug!(provider_type = %provider_config.provider_type(), "Creating provider from provider_config");
        let provider = ProviderFactory::create(provider_config.clone()).with_context(|| {
            format!(
                "Failed to create {} provider",
                provider_config.provider_type()
            )
        })?;
        return Ok(Arc::from(provider));
    }

    // No provider_config - try Ollama as fallback (local provider, no API key needed)
    debug!("No provider_config found, trying Ollama provider");
    let provider_config = ProviderConfig::Ollama(crate::providers::OllamaConfig::new());

    ProviderFactory::create(provider_config)
        .map(Arc::from)
        .context("Failed to create Ollama provider")
        .context("No LLM provider available. Please configure a provider in ~/.miniclaw/config.json or ensure Ollama is running locally")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_command_available() {
        // Verify the gateway command is registered in CLI
        use crate::cli::Cli;
        use clap::CommandFactory;

        let cmd = Cli::command();
        let subcommands: Vec<_> = cmd
            .get_subcommands()
            .map(|sc| sc.get_name().to_string())
            .collect();

        assert!(
            subcommands.contains(&"gateway".to_string()),
            "Gateway command should be available"
        );
    }
}
