//! Memory tool for the agent
//!
//! This tool provides memory management capabilities allowing the agent to
//! persist information to long-term memory or create daily notes.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::agent::tools::types::{Tool, ToolError, ToolExecutionContext, ToolResult};
use crate::memory::MemoryStore;
use crate::memory::types::MemoryType;

/// Tool for memory management
///
/// Provides capabilities for:
/// - Writing to long-term memory (MEMORY.md)
/// - Creating daily notes (YYYY-MM-DD.md)
#[derive(Debug, Clone)]
pub struct MemoryTool {
    memory_store: MemoryStore,
    workspace_path: PathBuf,
}

impl MemoryTool {
    /// Creates a new MemoryTool with the provided workspace path
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory for memory files
    ///
    /// # Returns
    /// * `Ok(MemoryTool)` - Successfully created tool
    /// * `Err(String)` - If workspace path cannot be canonicalized
    pub fn new(workspace_path: PathBuf) -> Result<Self, String> {
        // Canonicalize workspace path for security
        let canonical_workspace = std::fs::canonicalize(&workspace_path).map_err(|e| {
            format!(
                "Failed to canonicalize workspace path {:?}: {}",
                workspace_path, e
            )
        })?;

        Ok(Self {
            memory_store: MemoryStore::new(canonical_workspace.clone()),
            workspace_path: canonical_workspace,
        })
    }
}

#[async_trait::async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "write_memory"
    }

    fn description(&self) -> &str {
        "Write information to memory (long-term or daily notes)"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The content to write to memory"
                },
                "type": {
                    "type": "string",
                    "enum": ["long_term", "daily"],
                    "description": "Type of memory to write (long_term for MEMORY.md, daily for YYYY-MM-DD.md)",
                    "default": "long_term"
                }
            },
            "required": ["content"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Get content parameter
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'content'".to_string(),
            })?;

        // Get memory type parameter (default to long_term)
        let memory_type_str = args
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("long_term");

        let memory_type = match memory_type_str {
            "long_term" => MemoryType::LongTerm,
            "daily" => MemoryType::Daily,
            _ => {
                return Err(ToolError::InvalidArguments {
                    tool: self.name().to_string(),
                    message: format!(
                        "Invalid memory type: '{}'. Must be 'long_term' or 'daily'",
                        memory_type_str
                    ),
                });
            }
        };

        // Use the validated workspace path from tool initialization
        let workspace_path = &self.workspace_path;

        // Validate workspace path using centralized validation
        use crate::utils::paths::validate_path;
        validate_path(workspace_path, "")
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Invalid workspace path: {}", e),
            })?;

        tracing::info!(
            "Memory tool executing: type={}, content_length={}",
            memory_type_str,
            content.len()
        );

        // Execute based on memory type
        let result = match memory_type {
            MemoryType::LongTerm => {
                self.memory_store
                    .append_to_memory(content.to_string())
                    .await
            }
            MemoryType::Daily => {
                self.memory_store
                    .create_daily_note(content.to_string())
                    .await
            }
        };

        // Handle result with proper error mapping
        match result {
            Ok(file_path) => {
                let message = match memory_type {
                    MemoryType::LongTerm => "Memory updated",
                    MemoryType::Daily => "Daily note created",
                };

                let response = json!({
                    "success": true,
                    "message": message,
                    "file_path": file_path,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                tracing::info!("Memory operation succeeded: {}", message);
                Ok(serde_json::to_string(&response).unwrap())
            }
            Err(memory_error) => {
                tracing::error!("Memory operation failed: {}", memory_error);

                // Map MemoryError to ToolError
                match memory_error {
                    crate::memory::types::MemoryError::FileTooLarge { path, size, limit } => {
                        // Note: This case should not occur anymore since we log warnings instead
                        let response = json!({
                            "success": true,
                            "message": "Memory updated (warning: file size approaching limit)",
                            "file_path": path,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "warning": format!("File size {} bytes exceeds limit {} bytes. Consider memory maintenance.", size, limit),
                        });
                        Ok(serde_json::to_string(&response).unwrap())
                    }
                    crate::memory::types::MemoryError::InvalidContent(msg) => {
                        // Return validation error
                        Err(ToolError::InvalidArguments {
                            tool: self.name().to_string(),
                            message: msg,
                        })
                    }
                    crate::memory::types::MemoryError::FileNotFound(path) => {
                        // Return execution error
                        Err(ToolError::ExecutionFailed {
                            tool: self.name().to_string(),
                            message: format!("Memory file not found: {}", path),
                        })
                    }
                    crate::memory::types::MemoryError::StorageFailed { operation, source } => {
                        // Return execution error with source
                        Err(ToolError::ExecutionFailed {
                            tool: self.name().to_string(),
                            message: format!(
                                "Storage operation '{}' failed: {}",
                                operation, source
                            ),
                        })
                    }
                    crate::memory::types::MemoryError::PathValidationFailed(msg) => {
                        // Return permission error
                        Err(ToolError::PermissionDenied {
                            tool: self.name().to_string(),
                            message: msg,
                        })
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_tool_name() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let tool = MemoryTool::new(workspace_path).unwrap();
        assert_eq!(tool.name(), "write_memory");
    }

    #[test]
    fn test_memory_tool_description() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let tool = MemoryTool::new(workspace_path).unwrap();
        let desc = tool.description();
        assert!(desc.contains("memory"));
    }

    #[test]
    fn test_memory_tool_parameters() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let tool = MemoryTool::new(workspace_path).unwrap();
        let params = tool.parameters();

        // Check structure
        assert_eq!(params.get("type").unwrap(), "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());

        // Check content parameter
        let properties = params.get("properties").unwrap();
        let content = properties.get("content").unwrap();
        assert_eq!(content.get("type").unwrap(), "string");

        // Check type parameter
        let memory_type = properties.get("type").unwrap();
        let enum_values = memory_type.get("enum").unwrap().as_array().unwrap();
        assert!(enum_values.contains(&json!("long_term")));
        assert!(enum_values.contains(&json!("daily")));
    }
}
