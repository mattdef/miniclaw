//! Tool system for miniclaw
//!
//! This module provides the tool trait, registry, and implementations
//! for tools that the agent can use to perform actions.

pub mod cron;
pub mod exec;
pub mod filesystem;
pub mod message;
pub mod spawn;
pub mod types;
pub mod web;

// Re-export types from types module for backward compatibility
pub use types::{
    validate_args_against_schema, Tool, ToolDefinition, ToolError, ToolExecutionContext,
    ToolResult,
};

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

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
    pub fn with_default_tools(
        hub: std::sync::Arc<crate::chat::ChatHub>,
        default_channel: impl Into<String>,
    ) -> Self {
        let registry = Self::new();
        registry
            .register(Box::new(crate::agent::tools::message::MessageTool::new(
                hub,
                default_channel,
            )))
            .expect("default tool registration must succeed");
        registry
    }

    /// Registers a tool in the registry
    ///
    /// # Arguments
    /// * `tool` - The tool to register
    ///
    /// # Errors
    /// Returns `ToolError::ExecutionFailed` if a tool with the same name is already registered
    pub fn register(&self, tool: Box<dyn Tool>) -> types::ToolResult<()> {
        let name = tool.name().to_string();
        
        let mut tools = self.tools.write().unwrap();
        
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
        let mut cache = self.definitions_cache.write().unwrap();
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
    pub fn unregister(&self, name: &str) -> bool {
        let mut tools = self.tools.write().unwrap();
        let removed = tools.remove(name).is_some();
        
        if removed {
            // Invalidate cache
            let mut cache = self.definitions_cache.write().unwrap();
            *cache = None;
        }
        
        removed
    }

    /// Checks if a tool is registered
    ///
    /// # Arguments
    /// * `name` - The name of the tool to check
    pub fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }

    /// Lists all registered tools with their names, descriptions, and parameter schemas
    ///
    /// # Returns
    /// A vector of tuples containing (name, description, parameters) for each tool
    pub fn list_tools(&self) -> Vec<(String, String, Value)> {
        let tools = self.tools.read().unwrap();
        tools
            .values()
            .map(|t| (t.name().to_string(), t.description().to_string(), t.parameters()))
            .collect()
    }

    /// Returns the number of registered tools
    pub fn len(&self) -> usize {
        let tools = self.tools.read().unwrap();
        tools.len()
    }

    /// Checks if the registry is empty
    pub fn is_empty(&self) -> bool {
        let tools = self.tools.read().unwrap();
        tools.is_empty()
    }

    /// Returns all tool definitions formatted for LLM function calling
    ///
    /// Converts all registered tools to OpenAI-compatible function definitions
    /// for use in LLM API calls.
    ///
    /// # Caching
    /// Results are cached after first call. Cache is invalidated on register/unregister.
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        // Check cache first
        {
            let cache = self.definitions_cache.read().unwrap();
            if let Some(ref cached) = *cache {
                return cached.clone();
            }
        }
        
        // Generate definitions
        let tools = self.tools.read().unwrap();
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
            let mut cache = self.definitions_cache.write().unwrap();
            *cache = Some(definitions.clone());
        }
        
        definitions
    }

    /// Returns all tools as ToolDefinition structs
    ///
    /// Similar to `get_tool_definitions()` but returns strongly-typed
    /// ToolDefinition structs instead of JSON values.
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().unwrap();
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
    pub async fn execute_tool_with_timeout(
        &self,
        name: &str,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
        timeout: Duration,
    ) -> types::ToolResult<String> {
        // Get tool schema and validate args (under read lock)
        let schema = {
            let tools = self.tools.read().unwrap();
            let tool = tools
                .get(name)
                .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
            tool.parameters()
        };
        
        validate_args_against_schema(&args, &schema, name)?;

        // Clone the tool for execution (Box<dyn Tool> can't be cloned, so we keep the read lock)
        // Execute with timeout
        let tools = self.tools.read().unwrap();
        let tool = tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
            
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

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_tool_registration() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_duplicate_registration_fails() {
        let registry = ToolRegistry::new();
        let tool1 = TestTool;
        let tool2 = TestTool;

        registry.register(Box::new(tool1)).unwrap();
        let result = registry.register(Box::new(tool2));

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

    #[test]
    fn test_contains() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        assert!(!registry.contains("test_tool"));

        registry.register(Box::new(tool)).unwrap();

        assert!(registry.contains("test_tool"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_unregister() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();
        assert_eq!(registry.len(), 1);

        let removed = registry.unregister("test_tool");
        assert!(removed);
        assert_eq!(registry.len(), 0);

        let not_found = registry.unregister("nonexistent");
        assert!(!not_found);
    }

    #[test]
    fn test_list_tools() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let tools = registry.list_tools();
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

        registry.register(Box::new(tool)).unwrap();

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

    #[test]
    fn test_get_tool_definitions() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let definitions = registry.get_tool_definitions();
        assert_eq!(definitions.len(), 1);

        let def = &definitions[0];
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "test_tool");
        assert_eq!(def["function"]["description"], "A test tool");
        assert_eq!(def["function"]["strict"], false);
    }

    #[test]
    fn test_get_definitions() {
        let registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let definitions = registry.get_definitions();
        assert_eq!(definitions.len(), 1);

        let def = &definitions[0];
        assert_eq!(def.name(), "test_tool");
        assert_eq!(def.description(), "A test tool");
        assert_eq!(def.r#type, "function");
    }

    #[test]
    fn test_default() {
        let registry: ToolRegistry = Default::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_multiple_tools() {
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
        registry.register(Box::new(TestTool)).unwrap();
        registry.register(Box::new(AnotherTool)).unwrap();

        assert_eq!(registry.len(), 2);

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 2);

        let names: Vec<String> = tools.iter().map(|(n, _, _)| n.clone()).collect();
        assert!(names.contains(&"test_tool".to_string()));
        assert!(names.contains(&"another_tool".to_string()));
    }
}
