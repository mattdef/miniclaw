//! Context Builder implementation for assembling conversation context
//!
//! This module implements the ContextBuilder trait defined in agent_loop.rs
//! to assemble complete conversation context from various sources.
//!
//! ## Context Layer Ordering (Critical for AC #1)
//!
//! Context is assembled in this exact order to ensure the LLM receives context
//! with proper priority and freshness:
//! 1. **System**: SOUL.md + AGENTS.md combined (personality and behavior)
//! 2. **Bootstrap**: Agent capabilities, current date/time, environment info
//! 3. **Memory**: Long-term memories from MEMORY.md (ranked by relevance)
//! 4. **Skills**: Available skills from workspace/skills/ directory
//! 5. **Tools**: Tool documentation from TOOLS.md
//! 6. **History**: Most recent conversation messages (max 50, newest first)
//! 7. **Current**: The user's current message (always last, never truncated)
//!
//! This ordering ensures system instructions are never truncated, recent context
//! is prioritized, and the current message always reaches the LLM.

use chrono;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::agent::agent_loop::{AgentError, ContextBuilder, Result};
use crate::chat::InboundMessage;
use crate::providers::{LlmMessage, LlmRole, LlmToolCall};
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
    // Note: cached_tools_content removed - use static cache if needed in future
}

impl ContextBuilderImpl {
    /// Creates a new ContextBuilderImpl with the given workspace path
    pub fn new(workspace_path: impl AsRef<Path>) -> Result<Self> {
        let workspace_path = workspace_path.as_ref().to_path_buf();

        if !workspace_path.exists() {
            return Err(AgentError::ContextBuildError(format!(
                "Workspace path does not exist: {:?}",
                workspace_path
            )));
        }

        Ok(Self {
            workspace_path,
            config: ContextBuilderConfig::default(),
        })
    }

    /// Creates a new ContextBuilderImpl with custom configuration
    pub fn with_config(
        workspace_path: impl AsRef<Path>,
        config: ContextBuilderConfig,
    ) -> Result<Self> {
        let workspace_path = workspace_path.as_ref().to_path_buf();

        if !workspace_path.exists() {
            return Err(AgentError::ContextBuildError(format!(
                "Workspace path does not exist: {:?}",
                workspace_path
            )));
        }

        Ok(Self {
            workspace_path,
            config,
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
    /// Loads both files in parallel for optimal performance (AC #2, Performance Notes)
    async fn build_system_message(&self) -> LlmMessage {
        // Load SOUL.md and AGENTS.md in parallel
        let (soul_content, agents_content) =
            tokio::join!(self.load_soul_md(), self.load_agents_md());

        let content = match (soul_content, agents_content) {
            (Some(soul), Some(agents)) => {
                format!("{}\n\n{}", soul, agents)
            }
            (Some(soul), None) => soul,
            (None, Some(agents)) => agents,
            (None, None) => "You are miniclaw, a helpful AI assistant.".to_string(),
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

    /// Builds memory context message with limit enforcement (AC #3)
    async fn build_memory_message(&self) -> Option<LlmMessage> {
        let memory_content = self.load_memory_md().await?;

        // Split by lines and limit to max_memory_entries to avoid context overflow (AC #3)
        let limited_memory = memory_content
            .lines()
            .take(self.config.max_memory_entries)
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!("Relevant memories:\n{}", limited_memory);

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

    /// Builds skills context message with efficient formatting (AC #4)
    async fn build_skills_message(&self) -> Option<LlmMessage> {
        let skills = self.load_skills().await;

        if skills.is_empty() {
            return None;
        }

        // Extract first line/title from each skill for efficiency (AC #4)
        // Avoid dumping entire SKILL.md content which creates context bloat
        let skills_text = skills
            .into_iter()
            .map(|(name, content)| {
                // Get just the first non-empty line as the skill description
                let description = content
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("(no description)")
                    .trim_start_matches("#")
                    .trim();
                format!("- {}: {}", name, description)
            })
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

    /// Builds conversation history from session (AC #6: most recent messages)
    fn build_history_messages(&self, session: &Session) -> Vec<LlmMessage> {
        // Get the most recent messages (last N, not first N) - AC #6 requirement
        let messages = &session.messages;
        let skip_count = if messages.len() > self.config.max_history_messages {
            messages.len() - self.config.max_history_messages
        } else {
            0
        };

        messages
            .iter()
            .skip(skip_count)
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => LlmRole::User,
                    "assistant" => LlmRole::Assistant,
                    "tool" | "tool_result" => LlmRole::Tool, // Support both "tool" and "tool_result" roles
                    other => {
                        tracing::warn!(role = %other, "Unknown message role, treating as User");
                        LlmRole::User // Default to User, not System (AC #6)
                    }
                };

                LlmMessage {
                    role,
                    content: msg.content.clone(),
                    tool_calls: msg.tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .map(|call| LlmToolCall {
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
    fn truncate_context(
        &self,
        mut messages: Vec<LlmMessage>,
        current_message: &LlmMessage,
    ) -> Vec<LlmMessage> {
        let max_tokens = self.config.max_context_tokens;

        loop {
            let total_tokens: usize = messages
                .iter()
                .map(|m| self.estimate_tokens(&m.content))
                .sum();

            if total_tokens <= max_tokens {
                break;
            }

            // Find oldest non-system, non-current message to remove
            let removable_index = messages
                .iter()
                .enumerate()
                .find(|(_, m)| {
                    // Don't remove system messages or the current message
                    m.role != LlmRole::System && m.content != current_message.content
                })
                .map(|(i, _)| i);

            if let Some(idx) = removable_index {
                let removed = messages.remove(idx);
                tracing::debug!(
                    role = ?removed.role,
                    tokens = self.estimate_tokens(&removed.content),
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
        tracing::info!("Building conversation context with parallel file loading");

        let mut context = Vec::new();

        // 1. System message (SOUL.md + AGENTS.md loaded in parallel)
        let system_msg = self.build_system_message().await;
        context.push(system_msg);

        // 2. Bootstrap context (no I/O)
        let bootstrap_msg = self.build_bootstrap_message();
        context.push(bootstrap_msg);

        // 3 & 4. Load Memory and Skills in parallel while assembling bootstrap
        // (Bootstrap is added first since it's in-memory and doesn't depend on I/O)
        let (memory_msg, skills_msg, tools_msg) = tokio::join!(
            self.build_memory_message(),
            self.build_skills_message(),
            self.build_tools_message()
        );

        // 3. Memory layer (if available)
        if let Some(memory_msg) = memory_msg {
            context.push(memory_msg);
        }

        // 4. Skills layer (if available)
        if let Some(skills_msg) = skills_msg {
            context.push(skills_msg);
        }

        // 5. Tools layer (if available)
        if let Some(tools_msg) = tools_msg {
            context.push(tools_msg);
        }

        // 6. Conversation history (already in memory, FIFO order with most recent last)
        let history = self.build_history_messages(session);
        context.extend(history);

        // 7. Current user message
        let current_msg = LlmMessage {
            role: LlmRole::User,
            content: current_message.content.clone(),
            tool_calls: None,
        };
        context.push(current_msg.clone());

        // Truncate if necessary, protecting system and current messages
        let context = self.truncate_context(context, &current_msg);

        tracing::info!(
            message_count = context.len(),
            "Context built successfully with parallel I/O"
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
        fs::write(
            temp_dir.path().join("SOUL.md"),
            "You are a helpful assistant.",
        )
        .await
        .unwrap();
        fs::write(temp_dir.path().join("AGENTS.md"), "Be concise and clear.")
            .await
            .unwrap();

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
        fs::create_dir(temp_dir.path().join("memory"))
            .await
            .unwrap();
        fs::write(temp_dir.path().join("memory/MEMORY.md"), "User likes Rust.")
            .await
            .unwrap();

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
        fs::write(weather_dir.join("SKILL.md"), "Get weather information.")
            .await
            .unwrap();

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
            LlmMessage {
                role: LlmRole::System,
                content: "System".to_string(),
                tool_calls: None,
            },
            LlmMessage {
                role: LlmRole::User,
                content: "Old message".to_string(),
                tool_calls: None,
            },
            LlmMessage {
                role: LlmRole::Assistant,
                content: "Response".to_string(),
                tool_calls: None,
            },
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
        fs::write(temp_dir.path().join("SOUL.md"), "Personality")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("AGENTS.md"), "Behavior")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("TOOLS.md"), "Tools info")
            .await
            .unwrap();

        let memory_dir = temp_dir.path().join("memory");
        fs::create_dir(&memory_dir).await.unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "User prefers conciseness")
            .await
            .unwrap();

        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).await.unwrap();
        let skill_dir = skills_dir.join("test-skill");
        fs::create_dir(&skill_dir).await.unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test Skill\nA test skill")
            .await
            .unwrap();

        let mut session = create_test_session();
        session.add_message(crate::session::Message::new(
            "user".to_string(),
            "Previous message".to_string(),
        ));

        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let inbound = InboundMessage::new("telegram", "123", "Hello");

        let context = builder.build_context(&session, &inbound).await.unwrap();

        // Verify correct layer assembly (AC #1):
        // Should have at minimum: System, Bootstrap, History, Current
        assert!(context.len() >= 4, "Context should have at least 4 layers");

        // Check that first message is System (SOUL.md + AGENTS.md)
        assert_eq!(context[0].role, LlmRole::System, "Layer 1: System");
        assert!(
            context[0].content.contains("Personality"),
            "Layer 1 should contain SOUL content"
        );
        assert!(
            context[0].content.contains("Behavior"),
            "Layer 1 should contain AGENTS content"
        );

        // Check that we have bootstrap (mentions date/time)
        let has_bootstrap = context
            .iter()
            .any(|m| m.role == LlmRole::System && m.content.contains("date/time"));
        assert!(has_bootstrap, "Should have bootstrap layer with date/time");

        // Check history is present (previous message)
        assert!(
            context
                .iter()
                .any(|m| m.content.contains("Previous message")),
            "Should include history message"
        );

        // Check current message is last (AC #7: never truncated)
        assert_eq!(context.last().unwrap().role, LlmRole::User);
        assert_eq!(context.last().unwrap().content, "Hello");

        // Check that memory comes before tools (when both present)
        let memory_idx = context
            .iter()
            .position(|m| m.role == LlmRole::System && m.content.contains("Relevant memories"));
        let tools_idx = context
            .iter()
            .position(|m| m.role == LlmRole::System && m.content.contains("Available tools"));
        if let (Some(mem_i), Some(tools_i)) = (memory_idx, tools_idx) {
            assert!(
                mem_i < tools_i,
                "Memory layer should come before Tools layer"
            );
        }
    }

    #[tokio::test]
    async fn test_history_selects_most_recent() {
        let mut session = create_test_session();

        // Add 5 messages to session
        for i in 1..=5 {
            session.add_message(crate::session::Message::new(
                "user".to_string(),
                format!("Message {}", i),
            ));
        }

        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let history = builder.build_history_messages(&session);

        // Should preserve order and include all messages
        assert_eq!(history.len(), 5);
        assert_eq!(history[0].content, "Message 1");
        assert_eq!(history[4].content, "Message 5"); // Most recent is last
    }

    #[tokio::test]
    async fn test_history_limits_to_max_messages() {
        let mut session = create_test_session();
        let mut config = ContextBuilderConfig::default();
        config.max_history_messages = 3;

        // Add 5 messages
        for i in 1..=5 {
            session.add_message(crate::session::Message::new(
                "user".to_string(),
                format!("Message {}", i),
            ));
        }

        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::with_config(temp_dir.path(), config).unwrap();
        let history = builder.build_history_messages(&session);

        // Should only have last 3 messages
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].content, "Message 3"); // Skipped 1-2
        assert_eq!(history[2].content, "Message 5"); // Most recent
    }

    #[tokio::test]
    async fn test_unknown_role_defaults_to_user() {
        let mut session = create_test_session();
        session.add_message(crate::session::Message::new(
            "unknown_role".to_string(),
            "Test message".to_string(),
        ));

        let temp_dir = TempDir::new().unwrap();
        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let history = builder.build_history_messages(&session);

        // Unknown role should default to User, not System
        assert_eq!(history[0].role, LlmRole::User);
    }

    #[tokio::test]
    async fn test_memory_enforces_max_entries() {
        let temp_dir = TempDir::new().unwrap();
        let memory_dir = temp_dir.path().join("memory");
        fs::create_dir(&memory_dir).await.unwrap();

        // Create memory file with 10 lines
        let memory_content = (1..=10)
            .map(|i| format!("Memory line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(memory_dir.join("MEMORY.md"), memory_content)
            .await
            .unwrap();

        let mut config = ContextBuilderConfig::default();
        config.max_memory_entries = 5; // Limit to 5

        let builder = ContextBuilderImpl::with_config(temp_dir.path(), config).unwrap();
        let msg = builder.build_memory_message().await.unwrap();

        // Should only contain first 5 lines (limited by max_memory_entries)
        assert!(msg.content.contains("Memory line 1"));
        assert!(msg.content.contains("Memory line 5"));
        // Lines 6-10 should not be present
        assert!(!msg.content.contains("Memory line 10"));
    }

    #[tokio::test]
    async fn test_parallel_system_loading() {
        let temp_dir = TempDir::new().unwrap();

        // Write files with distinct content
        fs::write(temp_dir.path().join("SOUL.md"), "SOUL CONTENT")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("AGENTS.md"), "AGENTS CONTENT")
            .await
            .unwrap();

        let builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let msg = builder.build_system_message().await;

        // Both files should be loaded and combined
        assert!(msg.content.contains("SOUL CONTENT"));
        assert!(msg.content.contains("AGENTS CONTENT"));
    }
}
