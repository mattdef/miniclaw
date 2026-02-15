//! Context Builder implementation for assembling conversation context
//!
//! This module implements the ContextBuilder trait defined in agent_loop.rs
//! to assemble complete conversation context from various sources.

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::agent::agent_loop::{ContextBuilder, LlmMessage, LlmRole, AgentError, Result};
use crate::chat::InboundMessage;
use crate::session::Session;

/// Configuration for context building
#[derive(Debug, Clone)]
pub struct ContextBuilderConfig {
    /// Maximum tokens allowed in context (approximate)
    pub max_context_tokens: usize,
    /// Maximum history messages to include
    pub max_history_messages: usize,
    /// Maximum memory entries to include
    pub max_memory_entries: usize,
    /// Whether to use cached TOOLS.md content
    pub cache_tools_content: bool,
}

impl Default for ContextBuilderConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4000,
            max_history_messages: 50,
            max_memory_entries: 20,
            cache_tools_content: true,
        }
    }
}

/// Implementation of ContextBuilder that assembles context from workspace files
pub struct ContextBuilderImpl {
    workspace_path: PathBuf,
    config: ContextBuilderConfig,
    cached_tools_content: Option<String>,
}

impl ContextBuilderImpl {
    /// Creates a new ContextBuilderImpl with the given workspace path
    pub fn new(workspace_path: impl AsRef<Path>) -> Result<Self> {
        let workspace_path = workspace_path.as_ref().to_path_buf();
        
        if !workspace_path.exists() {
            return Err(AgentError::ContextBuildError(
                format!("Workspace path does not exist: {:?}", workspace_path)
            ));
        }
        
        Ok(Self {
            workspace_path,
            config: ContextBuilderConfig::default(),
            cached_tools_content: None,
        })
    }
    
    /// Creates a new ContextBuilderImpl with custom configuration
    pub fn with_config(
        workspace_path: impl AsRef<Path>,
        config: ContextBuilderConfig,
    ) -> Result<Self> {
        let workspace_path = workspace_path.as_ref().to_path_buf();
        
        if !workspace_path.exists() {
            return Err(AgentError::ContextBuildError(
                format!("Workspace path does not exist: {:?}", workspace_path)
            ));
        }
        
        Ok(Self {
            workspace_path,
            config,
            cached_tools_content: None,
        })
    }
    
    /// Returns the workspace path
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }
    
    /// Returns the configuration
    pub fn config(&self) -> &ContextBuilderConfig {
        &self.config
    }
    
    /// Estimates token count for a string (simple heuristic: chars / 4)
    fn estimate_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }
    
    /// Loads SOUL.md from workspace
    async fn load_soul_md(&self) -> Option<String> {
        let path = self.workspace_path.join("SOUL.md");
        match fs::read_to_string(&path).await {
            Ok(content) => {
                tracing::debug!("Loaded SOUL.md");
                Some(content)
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "SOUL.md not found, using default");
                None
            }
        }
    }
    
    /// Loads AGENTS.md from workspace
    async fn load_agents_md(&self) -> Option<String> {
        let path = self.workspace_path.join("AGENTS.md");
        match fs::read_to_string(&path).await {
            Ok(content) => {
                tracing::debug!("Loaded AGENTS.md");
                Some(content)
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "AGENTS.md not found, using default");
                None
            }
        }
    }
    
    /// Creates the system message from SOUL.md and AGENTS.md
    async fn build_system_message(&self) -> LlmMessage {
        let soul_content = self.load_soul_md().await;
        let agents_content = self.load_agents_md().await;
        
        let content = match (soul_content, agents_content) {
            (Some(soul), Some(agents)) => {
                format!("{}\n\n{}", soul, agents)
            }
            (Some(soul), None) => soul,
            (None, Some(agents)) => agents,
            (None, None) => {
                "You are miniclaw, a helpful AI assistant.".to_string()
            }
        };
        
        LlmMessage {
            role: LlmRole::System,
            content,
            tool_calls: None,
        }
    }
    
    /// Builds bootstrap context with agent capabilities and environment info
    fn build_bootstrap_message(&self) -> LlmMessage {
        let now = chrono::Local::now();
        let content = format!(
            "Current date/time: {}\n\nYou have access to tools and skills that can help accomplish tasks.",
            now.format("%Y-%m-%d %H:%M:%S")
        );
        
        LlmMessage {
            role: LlmRole::System,
            content,
            tool_calls: None,
        }
    }
    
    /// Loads MEMORY.md from workspace/memory/
    async fn load_memory_md(&self) -> Option<String> {
        let path = self.workspace_path.join("memory").join("MEMORY.md");
        match fs::read_to_string(&path).await {
            Ok(content) => {
                tracing::debug!("Loaded MEMORY.md");
                Some(content)
            }
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "MEMORY.md not found");
                None
            }
        }
    }
    
    /// Builds memory context message
    async fn build_memory_message(&self) -> Option<LlmMessage> {
        let memory_content = self.load_memory_md().await?;
        
        // For now, include all memories (ranking will be added in Story 8.4)
        let content = format!("Relevant memories:\n{}", memory_content);
        
        Some(LlmMessage {
            role: LlmRole::System,
            content,
            tool_calls: None,
        })
    }
    
    /// Scans skills directory and loads all SKILL.md files
    async fn load_skills(&self) -> Vec<(String, String)> {
        let skills_dir = self.workspace_path.join("skills");
        let mut skills = Vec::new();
        
        match fs::read_dir(&skills_dir).await {
            Ok(mut entries) => {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_md = path.join("SKILL.md");
                        if skill_md.exists() {
                            match fs::read_to_string(&skill_md).await {
                                Ok(content) => {
                                    let skill_name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    tracing::debug!(skill = %skill_name, "Loaded skill");
                                    skills.push((skill_name, content));
                                }
                                Err(e) => {
                                    tracing::warn!(path = %skill_md.display(), error = %e, "Failed to read skill");
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(path = %skills_dir.display(), error = %e, "Skills directory not found");
            }
        }
        
        skills
    }
    
    /// Builds skills context message
    async fn build_skills_message(&self) -> Option<LlmMessage> {
        let skills = self.load_skills().await;
        
        if skills.is_empty() {
            return None;
        }
        
        let skills_text = skills
            .into_iter()
            .map(|(name, content)| format!("- {}: {}", name, content))
            .collect::<Vec<_>>()
            .join("\n");
        
        let content = format!("Available skills:\n{}", skills_text);
        
        Some(LlmMessage {
            role: LlmRole::System,
            content,
            tool_calls: None,
        })
    }
    
    /// Loads TOOLS.md from workspace
    async fn load_tools_md(&self) -> Option<String> {
        // Check cache first
        if let Some(ref cached) = self.cached_tools_content {
            tracing::debug!("Using cached TOOLS.md content");
            return Some(cached.clone());
        }
        
        let path = self.workspace_path.join("TOOLS.md");
        match fs::read_to_string(&path).await {
            Ok(content) => {
                tracing::debug!("Loaded TOOLS.md");
                Some(content)
            }
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "TOOLS.md not found");
                None
            }
        }
    }
    
    /// Builds tools context message
    async fn build_tools_message(&self) -> Option<LlmMessage> {
        let tools_content = self.load_tools_md().await?;
        
        let content = format!("Available tools:\n{}", tools_content);
        
        Some(LlmMessage {
            role: LlmRole::System,
            content,
            tool_calls: None,
        })
    }
    
    /// Builds conversation history from session
    fn build_history_messages(&self, session: &Session) -> Vec<LlmMessage> {
        session
            .messages
            .iter()
            .take(self.config.max_history_messages)
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => LlmRole::User,
                    "assistant" => LlmRole::Assistant,
                    "tool" => LlmRole::Tool,
                    _ => LlmRole::System,
                };
                
                LlmMessage {
                    role,
                    content: msg.content.clone(),
                    tool_calls: msg.tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .map(|call| crate::agent::agent_loop::LlmToolCall {
                                id: call.id.clone(),
                                name: call.name.clone(),
                                arguments: call.arguments.clone(),
                            })
                            .collect()
                    }),
                }
            })
            .collect()
    }
    
    /// Truncates context to fit within token limit
    /// Strategy: Remove oldest history messages first, never remove system or current message
    fn truncate_context(&self, mut messages: Vec<LlmMessage>, _current_message: &LlmMessage) -> Vec<LlmMessage> {
        let max_tokens = self.config.max_context_tokens;
        
        loop {
            let total_tokens: usize = messages
                .iter()
                .map(|m| self.estimate_tokens(&m.content))
                .sum();
            
            if total_tokens <= max_tokens {
                break;
            }
            
            // Find oldest non-system message to remove
            let removable_index = messages
                .iter()
                .enumerate()
                .position(|(i, m)| {
                    m.role != LlmRole::System && i != messages.len() - 1
                });
            
            if let Some(idx) = removable_index {
                let removed = messages.remove(idx);
                tracing::debug!(
                    role = ?removed.role,
                    "Truncated message from context"
                );
            } else {
                // Can't remove any more messages
                tracing::warn!(
                    total_tokens = total_tokens,
                    max_tokens = max_tokens,
                    "Context still exceeds token limit but cannot truncate further"
                );
                break;
            }
        }
        
        messages
    }
}

#[async_trait::async_trait]
impl ContextBuilder for ContextBuilderImpl {
    async fn build_context(
        &self,
        session: &Session,
        current_message: &InboundMessage,
    ) -> Result<Vec<LlmMessage>> {
        tracing::info!("Building conversation context");
        
        let mut context = Vec::new();
        
        // 1. System message (SOUL.md + AGENTS.md)
        let system_msg = self.build_system_message().await;
        context.push(system_msg);
        
        // 2. Bootstrap context
        let bootstrap_msg = self.build_bootstrap_message();
        context.push(bootstrap_msg);
        
        // 3. Memory layer (if available)
        if let Some(memory_msg) = self.build_memory_message().await {
            context.push(memory_msg);
        }
        
        // 4. Skills layer (if available)
        if let Some(skills_msg) = self.build_skills_message().await {
            context.push(skills_msg);
        }
        
        // 5. Tools layer (if available)
        if let Some(tools_msg) = self.build_tools_message().await {
            context.push(tools_msg);
        }
        
        // 6. Conversation history
        let history = self.build_history_messages(session);
        context.extend(history);
        
        // 7. Current user message
        let current_msg = LlmMessage {
            role: LlmRole::User,
            content: current_message.content.clone(),
            tool_calls: None,
        };
        context.push(current_msg.clone());
        
        // Truncate if necessary
        let context = self.truncate_context(context, &current_msg);
        
        tracing::info!(
            message_count = context.len(),
            "Context built successfully"
        );
        
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_session() -> Session {
        Session::new("telegram".to_string(), "123456789".to_string())
    }

    #[tokio::test]
    async fn test_context_builder_creation() {
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path());
        assert!(builder.is_ok());
    }

    #[tokio::test]
    async fn test_context_builder_invalid_path() {
        let result = ContextBuilderImpl::new("/nonexistent/path");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_build_system_message_with_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create SOUL.md
        fs::write(temp_dir.path().join("SOUL.md"), "You are a helpful assistant.").await.unwrap();
        fs::write(temp_dir.path().join("AGENTS.md"), "Be concise and clear.").await.unwrap();
        
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_system_message().await;
        
        assert_eq!(msg.role, LlmRole::System);
        assert!(msg.content.contains("helpful assistant"));
        assert!(msg.content.contains("concise and clear"));
    }

    #[tokio::test]
    async fn test_build_system_message_default() {
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_system_message().await;
        
        assert_eq!(msg.role, LlmRole::System);
        assert!(msg.content.contains("miniclaw"));
    }

    #[tokio::test]
    async fn test_build_bootstrap_message() {
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_bootstrap_message();
        
        assert_eq!(msg.role, LlmRole::System);
        assert!(msg.content.contains("Current date/time"));
        assert!(msg.content.contains("tools and skills"));
    }

    #[tokio::test]
    async fn test_build_memory_message() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("memory")).await.unwrap();
        fs::write(temp_dir.path().join("memory/MEMORY.md"), "User likes Rust.").await.unwrap();
        
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_memory_message().await;
        
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.role, LlmRole::System);
        assert!(msg.content.contains("User likes Rust"));
    }

    #[tokio::test]
    async fn test_build_memory_message_missing() {
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_memory_message().await;
        
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_load_skills() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).await.unwrap();
        
        let weather_dir = skills_dir.join("weather");
        fs::create_dir(&weather_dir).await.unwrap();
        fs::write(weather_dir.join("SKILL.md"), "Get weather information.").await.unwrap();
        
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let skills = builder.load_skills().await;
        
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].0, "weather");
        assert!(skills[0].1.contains("weather information"));
    }

    #[tokio::test]
    async fn test_build_history_messages() {
        let mut session = create_test_session();
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "Hello".to_string(),
        ));
        session.add_message(crate::session::Message::new(
            "assistant".to_string(),
            "Hi there!".to_string(),
        ));
        
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let history = builder.build_history_messages(&session);
        
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, LlmRole::User);
        assert_eq!(history[0].content, "Hello");
        assert_eq!(history[1].role, LlmRole::Assistant);
        assert_eq!(history[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_estimate_tokens() {
        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        
        // Simple heuristic: chars / 4
        assert_eq!(builder.estimate_tokens("aaaa"), 1);
        assert_eq!(builder.estimate_tokens("aaaaaaaa"), 2);
    }

    #[tokio::test]
    async fn test_truncate_context() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = ContextBuilderConfig::default();
        config.max_context_tokens = 10; // Very small for testing
        
        let builder = ContextBuilderImpl::with_config(temp_dir.path(), config).unwrap();
        
        let current = LlmMessage {
            role: LlmRole::User,
            content: "Current".to_string(),
            tool_calls: None,
        };
        
        let messages = vec![
            LlmMessage { role: LlmRole::System, content: "System".to_string(), tool_calls: None },
            LlmMessage { role: LlmRole::User, content: "Old message".to_string(), tool_calls: None },
            LlmMessage { role: LlmRole::Assistant, content: "Response".to_string(), tool_calls: None },
            current.clone(),
        ];
        
        let truncated = builder.truncate_context(messages, &current);
        
        // Should keep system and current, remove oldest non-system
        assert!(truncated.iter().any(|m| m.role == LlmRole::System));
        assert!(truncated.iter().any(|m| m.content == "Current"));
    }

    #[tokio::test]
    async fn test_build_context_full() {
        let temp_dir = TempDir::new().unwrap();
        
        // Setup workspace files
        fs::write(temp_dir.path().join("SOUL.md"), "Personality").await.unwrap();
        fs::write(temp_dir.path().join("AGENTS.md"), "Behavior").await.unwrap();
        fs::write(temp_dir.path().join("TOOLS.md"), "Tools info").await.unwrap();
        
        let mut session = create_test_session();
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "Previous message".to_string(),
        ));
        
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let inbound = InboundMessage::new("telegram", "123", "Hello");
        
        let context = builder.build_context(&session, &inbound).await.unwrap();
        
        // Should have: system, bootstrap, tools, history, current
        assert!(context.len() >= 4);
        assert_eq!(context[0].role, LlmRole::System);
        assert_eq!(context.last().unwrap().role, LlmRole::User);
        assert_eq!(context.last().unwrap().content, "Hello");
    }
}
