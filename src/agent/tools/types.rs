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
    pub fn tool_name(&self) -> &str {
        match self {
            ToolError::NotFound(name) => name,
            ToolError::InvalidArguments { tool, .. } => tool,
            ToolError::ExecutionFailed { tool, .. } => tool,
            ToolError::ExecutionFailedRecoverable { tool, .. } => tool,
            ToolError::PermissionDenied { tool, .. } => tool,
            ToolError::Timeout { tool, .. } => tool,
        }
    }
}

/// Result type for tool operations
pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// Execution context passed to tools during execution
///
/// Provides tools with information about the current execution environment
/// and conversation context.
#[derive(Debug, Clone, Default)]
pub struct ToolExecutionContext {
    /// Channel for the current conversation (e.g., "telegram", "cli")
    pub channel: Option<String>,
    /// Chat/user identifier for the current conversation
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
