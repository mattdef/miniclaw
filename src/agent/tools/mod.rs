pub mod message;

use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Error types for tool execution
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    
    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArguments { tool: String, message: String },
    
    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed { tool: String, message: String },
    
    #[error("Permission denied for tool '{tool}': {message}")]
    PermissionDenied { tool: String, message: String },
    
    #[error("Tool '{tool}' timed out after {duration}s")]
    Timeout { tool: String, duration: u64 },
}

impl ToolError {
    /// Check if the error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ToolError::Timeout { .. })
    }
    
    /// Get the tool name from the error
    pub fn tool_name(&self) -> &str {
        match self {
            ToolError::NotFound(name) => name,
            ToolError::InvalidArguments { tool, .. } => tool,
            ToolError::ExecutionFailed { tool, .. } => tool,
            ToolError::PermissionDenied { tool, .. } => tool,
            ToolError::Timeout { tool, .. } => tool,
        }
    }
}

/// Result type for tool operations
pub type Result<T> = std::result::Result<T, ToolError>;

/// Trait for implementing tools that the agent can use
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of the tool
    fn name(&self) -> &str;
    
    /// Returns a description of what the tool does
    fn description(&self) -> &str;
    
    /// Returns the JSON Schema for the tool's parameters
    fn parameters(&self) -> Value;
    
    /// Executes the tool with the given arguments
    /// 
    /// # Arguments
    /// * `args` - A HashMap of parameter names to their JSON values
    /// 
    /// # Returns
    /// * `Ok(String)` - The result of the tool execution as a string
    /// * `Err(ToolError)` - If execution fails
    async fn execute(&self, args: HashMap<String, Value>) -> Result<String>;
}

/// Registry for managing available tools
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
    
    /// Registers a tool in the registry
    /// 
    /// # Arguments
    /// * `tool` - The tool to register
    /// 
    /// # Errors
    /// Returns an error if a tool with the same name is already registered
    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<()> {
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
    
    /// Retrieves a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    /// Lists all registered tools with their descriptions
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

        async fn execute(&self, args: HashMap<String, Value>) -> Result<String> {
            let input = args
                .get("input")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "test_tool".to_string(),
                    message: "Missing required parameter 'input'".to_string(),
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
        
        let result = tool.execute(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: hello");
    }

    #[tokio::test]
    async fn test_tool_execution_missing_param() {
        let tool = TestTool;
        let args = HashMap::new();
        
        let result = tool.execute(args).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ToolError::InvalidArguments { tool, .. } => {
                assert_eq!(tool, "test_tool");
            }
            _ => panic!("Expected InvalidArguments error"),
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
    fn test_tool_error_recoverable() {
        let timeout_error = ToolError::Timeout {
            tool: "test".to_string(),
            duration: 30,
        };
        assert!(timeout_error.is_recoverable());
        
        let not_found_error = ToolError::NotFound("test".to_string());
        assert!(!not_found_error.is_recoverable());
        
        let exec_error = ToolError::ExecutionFailed {
            tool: "test".to_string(),
            message: "failed".to_string(),
        };
        assert!(!exec_error.is_recoverable());
    }

    #[test]
    fn test_tool_error_tool_name() {
        let error = ToolError::NotFound("my_tool".to_string());
        assert_eq!(error.tool_name(), "my_tool");
        
        let error = ToolError::InvalidArguments {
            tool: "other_tool".to_string(),
            message: "bad args".to_string(),
        };
        assert_eq!(error.tool_name(), "other_tool");
    }
}
