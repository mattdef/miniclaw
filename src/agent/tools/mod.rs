//! Tool system for miniclaw
//!
//! This module provides the tool trait, registry, and implementations
//! for tools that the agent can use to perform actions.

pub mod cron;
pub mod exec;
pub mod filesystem;
pub mod memory;
pub mod message;
pub mod security;
pub mod skill;
pub mod spawn;
pub mod types;
pub mod web;

// Re-export types from types module for backward compatibility
pub use types::{
    Tool, ToolDefinition, ToolError, ToolExecutionContext, ToolResult, validate_args_against_schema,
};

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::Config;

/// Registry for managing available tools
///
/// The ToolRegistry stores and manages all tools available to the agent.
/// It provides methods for registering, retrieving, and listing tools,
/// as well as generating tool definitions for LLM function calling.
///
/// # Thread Safety
///
/// ToolRegistry uses RwLock internally for thread-safe concurrent access.
/// Multiple threads can read simultaneously, but writes are exclusive.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
    // Cache for tool definitions to avoid re-serialization
    definitions_cache: Arc<RwLock<Option<Vec<Value>>>>,
}

impl ToolRegistry {
    /// Creates a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            definitions_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates a new registry with default tools pre-registered
    ///
    /// # Arguments
    /// * `hub` - The ChatHub for sending messages
    /// * `default_channel` - The default channel to use for message tool
    pub async fn with_default_tools(
        hub: std::sync::Arc<crate::chat::ChatHub>,
        default_channel: impl Into<String>,
    ) -> Self {
        let registry = Self::new();
        registry
            .register(Box::new(crate::agent::tools::message::MessageTool::new(
                hub,
                default_channel,
            )))
            .await
            .expect("default tool registration must succeed");
        registry
    }

    /// Creates a new registry with all default tools pre-registered
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory for tools that need filesystem access
    /// * `chat_hub` - The ChatHub for the message tool
    /// * `config` - Application config for tool-specific settings
    /// * `default_channel` - The default channel to use for the message tool
    ///
    /// # Returns
    /// A ToolRegistry with all default tools registered. If a tool fails to register,
    /// a warning is logged and the registry continues without that tool (graceful degradation).
    pub async fn with_all_default_tools(
        workspace_path: PathBuf,
        chat_hub: Arc<crate::chat::ChatHub>,
        config: &Config,
        default_channel: impl Into<String>,
    ) -> Self {
        let registry = Self::new();
        let default_channel = default_channel.into();

        // Register filesystem tool
        if let Err(e) = registry
            .register(Box::new(
                crate::agent::tools::filesystem::FilesystemTool::new(workspace_path.clone()),
            ))
            .await
        {
            tracing::warn!(error = %e, "Failed to register filesystem tool, continuing without it");
        }

        // Register exec tool
        match crate::agent::tools::exec::ExecTool::new(workspace_path.clone()) {
            Ok(exec_tool) => {
                if let Err(e) = registry.register(Box::new(exec_tool)).await {
                    tracing::warn!(error = %e, "Failed to register exec tool, continuing without it");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create exec tool, continuing without it");
            }
        }

        // Register web tool
        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::web::WebTool::new()))
            .await
        {
            tracing::warn!(error = %e, "Failed to register web tool, continuing without it");
        }

        // Register spawn tool
        match crate::agent::tools::spawn::SpawnTool::new(
            workspace_path.clone(),
            config.spawn_log_output,
        ) {
            Ok(spawn_tool) => {
                if let Err(e) = registry.register(Box::new(spawn_tool)).await {
                    tracing::warn!(error = %e, "Failed to register spawn tool, continuing without it");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create spawn tool, continuing without it");
            }
        }

        // Register cron tool and start scheduler
        let cron_scheduler = crate::cron::CronScheduler::new();
        let cron_tool = crate::agent::tools::cron::CronTool::new(cron_scheduler.clone());

        if let Err(e) = registry.register(Box::new(cron_tool)).await {
            tracing::warn!(error = %e, "Failed to register cron tool, continuing without it");
        } else {
            // Start cron scheduler background task
            let _scheduler_handle = cron_scheduler.start_scheduler();
            tracing::info!("Cron scheduler started");
        }

        // Register memory tool
        match crate::agent::tools::memory::MemoryTool::new(workspace_path.clone()) {
            Ok(memory_tool) => {
                if let Err(e) = registry.register(Box::new(memory_tool)).await {
                    tracing::warn!(error = %e, "Failed to register memory tool, continuing without it");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create memory tool, continuing without it");
            }
        }

        // Register skill management tools
        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::skill::CreateSkillTool::new(
                workspace_path.clone(),
            )))
            .await
        {
            tracing::warn!(error = %e, "Failed to register create_skill tool, continuing without it");
        }

        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::skill::ListSkillsTool::new(
                workspace_path.clone(),
            )))
            .await
        {
            tracing::warn!(error = %e, "Failed to register list_skills tool, continuing without it");
        }

        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::skill::ReadSkillTool::new(
                workspace_path.clone(),
            )))
            .await
        {
            tracing::warn!(error = %e, "Failed to register read_skill tool, continuing without it");
        }

        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::skill::DeleteSkillTool::new(
                workspace_path.clone(),
            )))
            .await
        {
            tracing::warn!(error = %e, "Failed to register delete_skill tool, continuing without it");
        }

        // Register message tool
        if let Err(e) = registry
            .register(Box::new(crate::agent::tools::message::MessageTool::new(
                chat_hub,
                default_channel,
            )))
            .await
        {
            tracing::warn!(error = %e, "Failed to register message tool, continuing without it");
        }

        let tool_count = registry.len().await;
        tracing::info!(
            tool_count = tool_count,
            "Tool registry initialized with default tools"
        );

        registry
    }

    /// Registers a tool in the registry
    ///
    /// # Arguments
    /// * `tool` - The tool to register
    ///
    /// # Errors
    /// Returns `ToolError::ExecutionFailed` if a tool with the same name is already registered
    pub async fn register(&self, tool: Box<dyn Tool>) -> types::ToolResult<()> {
        let name = tool.name().to_string();

        let mut tools = self.tools.write().await;

        if tools.contains_key(&name) {
            return Err(ToolError::ExecutionFailed {
                tool: name.clone(),
                message: format!(
                    "Tool '{}' is already registered. Suggestion: Use a different name like '{}_v2' or '{}_{}'",
                    name, name, name, "alt"
                ),
            });
        }

        tools.insert(name, tool);

        // Invalidate cache
        let mut cache = self.definitions_cache.write().await;
        *cache = None;

        Ok(())
    }

    /// Unregisters a tool from the registry
    ///
    /// # Arguments
    /// * `name` - The name of the tool to unregister
    ///
    /// # Returns
    /// `true` if the tool was found and removed, `false` otherwise
    pub async fn unregister(&self, name: &str) -> bool {
        let mut tools = self.tools.write().await;
        let removed = tools.remove(name).is_some();

        if removed {
            // Invalidate cache
            let mut cache = self.definitions_cache.write().await;
            *cache = None;
        }

        removed
    }

    /// Checks if a tool is registered
    ///
    /// # Arguments
    /// * `name` - The name of the tool to check
    pub async fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// Lists all registered tools with their names, descriptions, and parameter schemas
    ///
    /// # Returns
    /// A vector of tuples containing (name, description, parameters) for each tool
    pub async fn list_tools(&self) -> Vec<(String, String, Value)> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|t| {
                (
                    t.name().to_string(),
                    t.description().to_string(),
                    t.parameters(),
                )
            })
            .collect()
    }

    /// Returns the number of registered tools
    pub async fn len(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Checks if the registry is empty
    pub async fn is_empty(&self) -> bool {
        let tools = self.tools.read().await;
        tools.is_empty()
    }

    /// Returns all tool definitions formatted for LLM function calling
    ///
    /// Converts all registered tools to OpenAI-compatible function definitions
    /// for use in LLM API calls.
    ///
    /// # Caching
    /// Results are cached after first call. Cache is invalidated on register/unregister.
    pub async fn get_tool_definitions(&self) -> Vec<Value> {
        // Check cache first
        {
            let cache = self.definitions_cache.read().await;
            if let Some(ref cached) = *cache {
                return cached.clone();
            }
        }

        // Generate definitions
        let tools = self.tools.read().await;
        let definitions: Vec<Value> = tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters(),
                        "strict": false
                    }
                })
            })
            .collect();

        // Update cache
        {
            let mut cache = self.definitions_cache.write().await;
            *cache = Some(definitions.clone());
        }

        definitions
    }

    /// Returns all tools as ToolDefinition structs
    ///
    /// Similar to `get_tool_definitions()` but returns strongly-typed
    /// ToolDefinition structs instead of JSON values.
    pub async fn get_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    /// Executes a tool by name with the given arguments
    ///
    /// # Arguments
    /// * `name` - The name of the tool to execute
    /// * `args` - Arguments to pass to the tool
    /// * `ctx` - Execution context
    ///
    /// # Returns
    /// The tool's result as a string, or an error if execution fails
    ///
    /// # Validation
    /// This method validates arguments against the tool's schema before execution
    ///
    /// # Timeout
    /// Tool execution has a 30-second timeout by default
    #[allow(clippy::await_holding_lock)]
    pub async fn execute_tool(
        &self,
        name: &str,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
    ) -> types::ToolResult<String> {
        self.execute_tool_with_timeout(name, args, ctx, Duration::from_secs(30))
            .await
    }

    /// Executes a tool with a custom timeout
    ///
    /// # Arguments
    /// * `name` - The name of the tool to execute
    /// * `args` - Arguments to pass to the tool
    /// * `ctx` - Execution context
    /// * `timeout` - Maximum duration for tool execution
    ///
    /// # Returns
    /// The tool's result as a string, or an error if execution fails or times out
    #[allow(clippy::await_holding_lock)]
    pub async fn execute_tool_with_timeout(
        &self,
        name: &str,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
        timeout: Duration,
    ) -> types::ToolResult<String> {
        // Get tool schema and validate args (under read lock)
        let schema = {
            let tools = self.tools.read().await;
            let tool = tools
                .get(name)
                .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
            tool.parameters()
        };

        validate_args_against_schema(&args, &schema, name)?;

        // Get the tool and clone it for execution
        // With tokio::sync::RwLock, the guard is Send so we can hold it across await
        let tools = self.tools.read().await;
        let tool = tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        // Execute with timeout
        match tokio::time::timeout(timeout, tool.execute(args, ctx)).await {
            Ok(result) => result,
            Err(_) => Err(ToolError::Timeout {
                tool: name.to_string(),
                duration: timeout.as_secs(),
            }),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::{Tool, ToolExecutionContext};

    struct TestTool;

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            })
        }

        async fn execute(
            &self,
            args: HashMap<String, Value>,
            _ctx: &ToolExecutionContext,
        ) -> types::ToolResult<String> {
            let input = args.get("input").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolError::InvalidArguments {
                    tool: "test_tool".to_string(),
                    message: "Missing required parameter 'input'".to_string(),
                }
            })?;

            Ok(format!("Processed: {}", input))
        }
    }

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();
        assert_eq!(registry.len().await, 1);
        assert!(!registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_duplicate_registration_fails() {
        let registry = ToolRegistry::new();
        let tool1 = TestTool;
        let tool2 = TestTool;

        registry.register(Box::new(tool1)).await.unwrap();
        let result = registry.register(Box::new(tool2)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { tool, message } => {
                assert_eq!(tool, "test_tool");
                assert!(message.contains("already registered"));
                assert!(message.contains("Suggestion"));
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_contains() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        assert!(!registry.contains("test_tool").await);

        registry.register(Box::new(tool)).await.unwrap();

        assert!(registry.contains("test_tool").await);
        assert!(!registry.contains("nonexistent").await);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();
        assert_eq!(registry.len().await, 1);

        let removed = registry.unregister("test_tool").await;
        assert!(removed);
        assert_eq!(registry.len().await, 0);

        let not_found = registry.unregister("nonexistent").await;
        assert!(!not_found);
    }

    #[tokio::test]
    async fn test_list_tools() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();

        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "test_tool");
        assert_eq!(tools[0].1, "A test tool");
        // Now also includes parameters schema
        assert!(tools[0].2.is_object());
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = TestTool;
        let mut args = HashMap::new();
        args.insert("input".to_string(), json!("hello"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: hello");
    }

    #[tokio::test]
    async fn test_tool_execution_missing_param() {
        let tool = TestTool;
        let args = HashMap::new();

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ToolError::InvalidArguments { tool, .. } => {
                assert_eq!(tool, "test_tool");
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_execute_tool_via_registry() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();

        let mut args = HashMap::new();
        args.insert("input".to_string(), json!("world"));

        let ctx = ToolExecutionContext::default();
        let result = registry.execute_tool("test_tool", args, &ctx).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: world");
    }

    #[tokio::test]
    async fn test_execute_tool_not_found() {
        let registry = ToolRegistry::new();

        let args = HashMap::new();
        let ctx = ToolExecutionContext::default();
        let result = registry.execute_tool("nonexistent", args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(tool) => {
                assert_eq!(tool, "nonexistent");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_tool_definitions() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();

        let definitions = registry.get_tool_definitions().await;
        assert_eq!(definitions.len(), 1);

        let def = &definitions[0];
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "test_tool");
        assert_eq!(def["function"]["description"], "A test tool");
        assert_eq!(def["function"]["strict"], false);
    }

    #[tokio::test]
    async fn test_get_definitions() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).await.unwrap();

        let definitions = registry.get_definitions().await;
        assert_eq!(definitions.len(), 1);

        let def = &definitions[0];
        assert_eq!(def.name(), "test_tool");
        assert_eq!(def.description(), "A test tool");
        assert_eq!(def.r#type, "function");
    }

    #[tokio::test]
    async fn test_default() {
        let registry: ToolRegistry = Default::default();
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_multiple_tools() {
        struct AnotherTool;

        #[async_trait::async_trait]
        impl Tool for AnotherTool {
            fn name(&self) -> &str {
                "another_tool"
            }

            fn description(&self) -> &str {
                "Another test tool"
            }

            fn parameters(&self) -> Value {
                json!({"type": "object", "properties": {}})
            }

            async fn execute(
                &self,
                _args: HashMap<String, Value>,
                _ctx: &ToolExecutionContext,
            ) -> types::ToolResult<String> {
                Ok("another result".to_string())
            }
        }

        let registry = ToolRegistry::new();
        registry.register(Box::new(TestTool)).await.unwrap();
        registry.register(Box::new(AnotherTool)).await.unwrap();

        assert_eq!(registry.len().await, 2);

        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 2);

        let names: Vec<String> = tools.iter().map(|(n, _, _)| n.clone()).collect();
        assert!(names.contains(&"test_tool".to_string()));
        assert!(names.contains(&"another_tool".to_string()));
    }

    #[tokio::test]
    async fn test_with_all_default_tools() {
        use crate::chat::ChatHub;
        use crate::config::Config;

        let workspace_path = std::env::temp_dir();
        let chat_hub = Arc::new(ChatHub::new());
        let config = Config::default();

        let registry =
            ToolRegistry::with_all_default_tools(workspace_path, chat_hub, &config, "test_channel")
                .await;

        // Should have registered all tools
        assert!(!registry.is_empty().await);
        assert!(registry.len().await >= 9); // At least 9 default tools

        // Verify specific tools are registered
        assert!(registry.contains("filesystem").await);
        assert!(registry.contains("exec").await);
        assert!(registry.contains("web").await);
        assert!(registry.contains("spawn").await);
        assert!(registry.contains("cron").await);
        assert!(registry.contains("write_memory").await);
        assert!(registry.contains("create_skill").await);
        assert!(registry.contains("list_skills").await);
        assert!(registry.contains("read_skill").await);
        assert!(registry.contains("delete_skill").await);
        assert!(registry.contains("message").await);
    }
}
