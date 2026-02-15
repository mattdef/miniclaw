//! Types for the tool system
//!
//! This module defines the core types for the tool system including:
//! - Tool trait for implementing tools
//! - ToolError for error handling
//! - ToolResult type alias
//! - ToolExecutionContext for execution context
//! - ToolDefinition for LLM function format

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Validates that a JSON value is a valid JSON Schema
///
/// Basic validation: checks for required fields and structure
fn validate_json_schema(schema: &Value) -> Result<(), String> {
    if !schema.is_object() {
        return Err("Schema must be an object".to_string());
    }
    
    let obj = schema.as_object().unwrap();
    
    // Check for required 'type' field
    if !obj.contains_key("type") {
        return Err("Schema must have a 'type' field".to_string());
    }
    
    let schema_type = obj.get("type").and_then(|v| v.as_str());
    if schema_type != Some("object") {
        return Err("Schema type must be 'object' for tool parameters".to_string());
    }
    
    // If properties exist, validate it's an object
    if let Some(props) = obj.get("properties") {
        if !props.is_object() {
            return Err("Schema 'properties' must be an object".to_string());
        }
    }
    
    // If required exists, validate it's an array
    if let Some(required) = obj.get("required") {
        if !required.is_array() {
            return Err("Schema 'required' must be an array".to_string());
        }
    }
    
    Ok(())
}

/// Validates arguments against a JSON schema
///
/// Basic validation: checks required fields are present
pub fn validate_args_against_schema(
    args: &HashMap<String, Value>,
    schema: &Value,
    tool_name: &str,
) -> ToolResult<()> {
    // Validate schema structure first
    validate_json_schema(schema).map_err(|e| ToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        message: format!("Invalid tool schema: {}", e),
    })?;
    
    let schema_obj = schema.as_object().unwrap();
    
    // Check required fields
    if let Some(required) = schema_obj.get("required") {
        if let Some(required_array) = required.as_array() {
            for req in required_array {
                if let Some(field_name) = req.as_str() {
                    if !args.contains_key(field_name) {
                        return Err(ToolError::InvalidArguments {
                            tool: tool_name.to_string(),
                            message: format!("Missing required parameter '{}'", field_name),
                        });
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Error types for tool execution
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ToolError {
    /// Tool not found in registry
    #[error("Tool not found: {0}")]
    NotFound(String),

    /// Invalid arguments provided to tool
    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArguments { tool: String, message: String },

    /// Tool execution failed (non-recoverable)
    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed { tool: String, message: String },

    /// Tool execution failed but is recoverable (can retry)
    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailedRecoverable { tool: String, message: String },

    /// Permission denied for tool execution
    #[error("Permission denied for tool '{tool}': {message}")]
    PermissionDenied { tool: String, message: String },

    /// Tool execution timed out
    #[error("Tool '{tool}' timed out after {duration}s")]
    Timeout { tool: String, duration: u64 },
}

impl ToolError {
    /// Check if the error is recoverable (can be retried)
    ///
    /// Recoverable errors include timeouts and certain execution failures
    /// that might succeed on retry.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ToolError::Timeout { .. } | ToolError::ExecutionFailedRecoverable { .. }
        )
    }

    /// Get the tool name from the error
    ///
    /// Returns the name of the tool that generated this error.
    /// If the name is empty, returns "<unnamed>" for better debugging.
    pub fn tool_name(&self) -> &str {
        let name = match self {
            ToolError::NotFound(name) => name.as_str(),
            ToolError::InvalidArguments { tool, .. } => tool.as_str(),
            ToolError::ExecutionFailed { tool, .. } => tool.as_str(),
            ToolError::ExecutionFailedRecoverable { tool, .. } => tool.as_str(),
            ToolError::PermissionDenied { tool, .. } => tool.as_str(),
            ToolError::Timeout { tool, .. } => tool.as_str(),
        };
        
        if name.trim().is_empty() {
            "<unnamed>"
        } else {
            name
        }
    }
}

/// Result type for tool operations
pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// Execution context passed to tools during execution
///
/// Provides tools with information about the current execution environment
/// and conversation context.
///
/// # Fields
///
/// * `channel` - The communication channel being used (e.g., "telegram", "cli").
///   Will be `None` if the tool is executed outside of a conversation context
///   (e.g., during testing or direct API calls).
///
/// * `chat_id` - The unique identifier for the current conversation or user.
///   Will be `None` if the tool is executed outside of a conversation context.
///   Tools should handle this gracefully when user identification is required.
#[derive(Debug, Clone, Default)]
pub struct ToolExecutionContext {
    /// Channel for the current conversation (e.g., "telegram", "cli")
    /// None when executing outside a conversation context
    pub channel: Option<String>,
    /// Chat/user identifier for the current conversation
    /// None when executing outside a conversation context
    pub chat_id: Option<String>,
}

/// Definition of a tool for LLM function calling
///
/// This structure represents the OpenAI-compatible function definition
/// format used to describe available tools to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// The type of tool (always "function" for now)
    pub r#type: String,
    /// Function metadata
    pub function: FunctionDefinition,
}

/// Function definition metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Tool description for LLM
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: Value,
}

impl ToolDefinition {
    /// Creates a new tool definition
    ///
    /// # Arguments
    /// * `name` - The unique name of the tool
    /// * `description` - Description of what the tool does
    /// * `parameters` - JSON Schema for the tool's parameters
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        &self.function.name
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        &self.function.description
    }

    /// Get the parameters schema
    pub fn parameters(&self) -> &Value {
        &self.function.parameters
    }
}

/// Trait for implementing tools that the agent can use
///
/// Any type implementing this trait can be registered with the ToolRegistry
/// and made available to the agent for execution.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of the tool
    ///
    /// This name is used to identify the tool in the registry and when
    /// the LLM requests tool execution.
    fn name(&self) -> &str;

    /// Returns a description of what the tool does
    ///
    /// This description is provided to the LLM to help it understand
    /// when and how to use the tool.
    fn description(&self) -> &str;

    /// Returns the JSON Schema for the tool's parameters
    ///
    /// The schema describes the expected parameters and their types,
    /// which the LLM uses to generate proper tool calls.
    /// 
    /// # Schema Validation
    /// 
    /// The returned schema MUST be a valid JSON Schema with:
    /// - `type: "object"` at the root level
    /// - `properties` object defining parameter types
    /// - `required` array listing mandatory parameters
    fn parameters(&self) -> Value;

    /// Executes the tool with the given arguments
    ///
    /// # Arguments
    /// * `args` - A HashMap of parameter names to their JSON values
    /// * `ctx` - Execution context providing information about the current conversation
    ///
    /// # Returns
    /// * `Ok(String)` - The result of the tool execution as a string
    /// * `Err(ToolError)` - If execution fails
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
    ) -> ToolResult<String>;

    /// Converts this tool to a ToolDefinition for LLM function calling
    ///
    /// This method generates the OpenAI-compatible function definition
    /// that describes this tool to the LLM.
    fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition::new(self.name(), self.description(), self.parameters())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_error_recoverable() {
        let timeout = ToolError::Timeout {
            tool: "test".to_string(),
            duration: 30,
        };
        assert!(timeout.is_recoverable());

        let recoverable = ToolError::ExecutionFailedRecoverable {
            tool: "test".to_string(),
            message: "buffer full".to_string(),
        };
        assert!(recoverable.is_recoverable());

        let not_recoverable = ToolError::NotFound("test".to_string());
        assert!(!not_recoverable.is_recoverable());

        let execution_failed = ToolError::ExecutionFailed {
            tool: "test".to_string(),
            message: "failed".to_string(),
        };
        assert!(!execution_failed.is_recoverable());
    }

    #[test]
    fn test_tool_error_tool_name() {
        let err = ToolError::NotFound("my_tool".to_string());
        assert_eq!(err.tool_name(), "my_tool");

        let err = ToolError::InvalidArguments {
            tool: "other_tool".to_string(),
            message: "bad args".to_string(),
        };
        assert_eq!(err.tool_name(), "other_tool");

        let err = ToolError::ExecutionFailed {
            tool: "exec_tool".to_string(),
            message: "error".to_string(),
        };
        assert_eq!(err.tool_name(), "exec_tool");

        let err = ToolError::PermissionDenied {
            tool: "secure_tool".to_string(),
            message: "no access".to_string(),
        };
        assert_eq!(err.tool_name(), "secure_tool");

        let err = ToolError::Timeout {
            tool: "slow_tool".to_string(),
            duration: 30,
        };
        assert_eq!(err.tool_name(), "slow_tool");
    }

    #[test]
    fn test_tool_definition_creation() {
        let params = json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            },
            "required": ["input"]
        });

        let def = ToolDefinition::new("test_tool", "A test tool", params.clone());

        assert_eq!(def.r#type, "function");
        assert_eq!(def.name(), "test_tool");
        assert_eq!(def.description(), "A test tool");
        assert_eq!(def.parameters(), &params);
    }

    #[test]
    fn test_tool_definition_accessors() {
        let def = ToolDefinition::new(
            "my_tool",
            "Does something useful",
            json!({"type": "object"}),
        );

        assert_eq!(def.name(), "my_tool");
        assert_eq!(def.description(), "Does something useful");
        assert_eq!(def.parameters(), &json!({"type": "object"}));
    }

    #[test]
    fn test_tool_execution_context_default() {
        let ctx = ToolExecutionContext::default();
        assert!(ctx.channel.is_none());
        assert!(ctx.chat_id.is_none());
    }

    #[test]
    fn test_tool_execution_context_with_values() {
        let ctx = ToolExecutionContext {
            channel: Some("telegram".to_string()),
            chat_id: Some("123456".to_string()),
        };
        assert_eq!(ctx.channel, Some("telegram".to_string()));
        assert_eq!(ctx.chat_id, Some("123456".to_string()));
    }

    #[test]
    fn test_tool_result_type() {
        let ok_result: ToolResult<String> = Ok("success".to_string());
        assert!(ok_result.is_ok());

        let err_result: ToolResult<String> = Err(ToolError::NotFound("tool".to_string()));
        assert!(err_result.is_err());
    }
}
