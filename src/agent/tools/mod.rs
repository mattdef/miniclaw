//! Tool system for miniclaw
//!
//! This module provides the tool trait, registry, and implementations
//! for tools that the agent can use to perform actions.

pub mod message;
pub mod types;

// Re-export types from types module for backward compatibility
pub use types::{Tool, ToolDefinition, ToolError, ToolExecutionContext, ToolResult};

use serde_json::Value;
use std::collections::HashMap;

/// Registry for managing available tools
///
/// The ToolRegistry stores and manages all tools available to the agent.
/// It provides methods for registering, retrieving, and listing tools,
/// as well as generating tool definitions for LLM function calling.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Creates a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
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
        let mut registry = Self::new();
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
    pub fn register(&mut self, tool: Box<dyn Tool>) -> types::ToolResult<()> {
        let name = tool.name().to_string();

        if self.tools.contains_key(&name) {
            return Err(ToolError::ExecutionFailed {
                tool: name.clone(),
                message: format!("Tool '{}' is already registered", name),
            });
        }

        self.tools.insert(name, tool);
        Ok(())
    }

    /// Unregisters a tool from the registry
    ///
    /// # Arguments
    /// * `name` - The name of the tool to unregister
    ///
    /// # Returns
    /// `true` if the tool was found and removed, `false` otherwise
    pub fn unregister(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    /// Retrieves a tool by name
    ///
    /// # Arguments
    /// * `name` - The name of the tool to retrieve
    ///
    /// # Returns
    /// `Some(&dyn Tool)` if found, `None` otherwise
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Checks if a tool is registered
    ///
    /// # Arguments
    /// * `name` - The name of the tool to check
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Lists all registered tools with their names and descriptions
    ///
    /// # Returns
    /// A vector of tuples containing (name, description) for each tool
    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools
            .values()
            .map(|t| (t.name(), t.description()))
            .collect()
    }

    /// Returns the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Checks if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Returns all tool definitions formatted for LLM function calling
    ///
    /// Converts all registered tools to OpenAI-compatible function definitions
    /// for use in LLM API calls.
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters(),
                    }
                })
            })
            .collect()
    }

    /// Returns all tools as ToolDefinition structs
    ///
    /// Similar to `get_tool_definitions()` but returns strongly-typed
    /// ToolDefinition structs instead of JSON values.
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
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
    pub async fn execute_tool(
        &self,
        name: &str,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
    ) -> types::ToolResult<String> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        tool.execute(args, ctx).await
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
        let mut registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_duplicate_registration_fails() {
        let mut registry = ToolRegistry::new();
        let tool1 = TestTool;
        let tool2 = TestTool;

        registry.register(Box::new(tool1)).unwrap();
        let result = registry.register(Box::new(tool2));

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { tool, .. } => {
                assert_eq!(tool, "test_tool");
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[test]
    fn test_get_tool() {
        let mut registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let retrieved = registry.get("test_tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_tool");

        let not_found = registry.get("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_contains() {
        let mut registry = ToolRegistry::new();
        let tool = TestTool;

        assert!(!registry.contains("test_tool"));

        registry.register(Box::new(tool)).unwrap();

        assert!(registry.contains("test_tool"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_unregister() {
        let mut registry = ToolRegistry::new();
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
        let mut registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "test_tool");
        assert_eq!(tools[0].1, "A test tool");
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
        let mut registry = ToolRegistry::new();
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
        let mut registry = ToolRegistry::new();
        let tool = TestTool;

        registry.register(Box::new(tool)).unwrap();

        let definitions = registry.get_tool_definitions();
        assert_eq!(definitions.len(), 1);

        let def = &definitions[0];
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "test_tool");
        assert_eq!(def["function"]["description"], "A test tool");
    }

    #[test]
    fn test_get_definitions() {
        let mut registry = ToolRegistry::new();
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
                json!({"type": "object"})
            }

            async fn execute(
                &self,
                _args: HashMap<String, Value>,
                _ctx: &ToolExecutionContext,
            ) -> types::ToolResult<String> {
                Ok("another result".to_string())
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(Box::new(TestTool)).unwrap();
        registry.register(Box::new(AnotherTool)).unwrap();

        assert_eq!(registry.len(), 2);

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"test_tool"));
        assert!(names.contains(&"another_tool"));
    }
}
