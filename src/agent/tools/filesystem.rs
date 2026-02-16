//! Filesystem tool for the agent
//!
//! This tool provides filesystem operations (read, write, list) with security constraints.
//! All paths are validated and canonicalized to prevent path traversal attacks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::fs;

use crate::agent::tools::types::{Tool, ToolError, ToolExecutionContext, ToolResult};
use crate::utils::paths::{validate_path, PathValidationError};

/// Tool for filesystem operations
///
/// Provides read, write, and list operations on files within a restricted
/// base directory. All paths are validated to prevent path traversal attacks.
///
/// # Security
/// This tool implements NFR-S3: All paths are canonicalized and validated
/// using the centralized path validation utilities in `crate::utils::paths`.
pub struct FilesystemTool {
    /// The canonicalized base directory that all operations are restricted to
    /// This is stored in canonical form for performance (no repeated canonicalization)
    base_dir: PathBuf,
}

impl FilesystemTool {
    /// Creates a new FilesystemTool with the specified base directory
    ///
    /// The base directory is canonicalized once during construction and stored
    /// for efficient reuse in path validation operations.
    ///
    /// # Arguments
    /// * `base_dir` - The root directory that all filesystem operations are restricted to
    ///
    /// # Panics
    /// Panics if the base directory cannot be canonicalized (doesn't exist or is inaccessible)
    ///
    /// # Note
    /// For async construction, use `new_async()` instead
    pub fn new(base_dir: PathBuf) -> Self {
        // Canonicalize the base directory once for performance
        // In production, this should be validated during app initialization
        let canonical_base = std::fs::canonicalize(&base_dir)
            .unwrap_or_else(|e| panic!("Failed to canonicalize base directory {:?}: {}", base_dir, e));
        
        Self {
            base_dir: canonical_base,
        }
    }

    /// Validates a user-provided path using the centralized validation utilities
    ///
    /// # Arguments
    /// * `user_path` - The path provided by the user/agent
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The canonicalized, validated path
    /// * `Err(ToolError)` - If path is invalid or outside allowed scope
    async fn validate_path_internal(&self, user_path: &str) -> ToolResult<PathBuf> {
        validate_path(&self.base_dir, user_path)
            .await
            .map_err(|e| match e {
                PathValidationError::OutsideBaseDirectory(path) => ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Path '{}' is outside the allowed base directory", path),
                },
                PathValidationError::SystemPathBlocked(path) => ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Access to system path '{}' is not allowed", path),
                },
                PathValidationError::CanonicalizationFailed { path, source } => {
                    ToolError::ExecutionFailed {
                        tool: self.name().to_string(),
                        message: format!("Failed to resolve path '{}': {}", path, source),
                    }
                }
                PathValidationError::InvalidBaseDirectory(msg) => ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: format!("Base directory error: {}", msg),
                },
            })
    }

    /// Reads the contents of a file
    ///
    /// # Arguments
    /// * `path` - The validated, canonicalized path to read
    ///
    /// # Returns
    /// * `Ok(String)` - The file contents as UTF-8 text
    /// * `Err(ToolError)` - If file cannot be read
    async fn read_file(&self, path: &Path) -> ToolResult<String> {
        // Verify it's a file, not a directory
        let metadata = match fs::metadata(path).await {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ToolError::NotFound(format!(
                    "File not found: {}",
                    path.display()
                )));
            }
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: format!("Cannot access file metadata: {}", e),
                });
            }
        };

        if !metadata.is_file() {
            return Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("'{}' is not a file", path.display()),
            });
        }

        // Read file content
        fs::read_to_string(path).await.map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to read file '{}': {}", path.display(), e),
        })
    }

    /// Writes content to a file
    ///
    /// Creates parent directories if needed. Logs a warning if overwriting an existing file.
    ///
    /// # Arguments
    /// * `path` - The validated, canonicalized path to write
    /// * `content` - The content to write
    ///
    /// # Returns
    /// * `Ok(String)` - Success confirmation message
    /// * `Err(ToolError)` - If write fails
    async fn write_file(&self, path: &Path, content: &str) -> ToolResult<String> {
        // Check if file exists (for warning) - use async try_exists for non-blocking
        let exists = tokio::fs::try_exists(path)
            .await
            .unwrap_or(false);
        
        if exists {
            tracing::warn!(
                "File exists, overwriting: {}",
                path.display()
            );
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to create directory '{}': {}", parent.display(), e),
            })?;
        }

        // Write content
        fs::write(path, content).await.map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to write file '{}': {}", path.display(), e),
        })?;

        Ok(format!("Successfully wrote to {}", path.display()))
    }

    /// Lists the contents of a directory
    ///
    /// # Arguments
    /// * `path` - The validated, canonicalized path to list
    ///
    /// # Returns
    /// * `Ok(String)` - JSON array of directory entries with name and type
    /// * `Err(ToolError)` - If path is not a directory or cannot be read
    ///
    /// # Entry Types
    /// - "file": Regular file
    /// - "directory": Directory
    /// - "other": Symlink, pipe, socket, or other special file type
    async fn list_dir(&self, path: &Path) -> ToolResult<String> {
        // Verify it's a directory
        let metadata = fs::metadata(path).await.map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Cannot access directory metadata: {}", e),
        })?;

        if !metadata.is_dir() {
            return Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Path '{}' is not a directory", path.display()),
            });
        }

        // Read directory entries
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(path).await.map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to read directory '{}': {}", path.display(), e),
        })?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to read directory entry: {}", e),
        })? {
            let file_type = entry.file_type().await.ok();
            let entry_type = if file_type.map_or(false, |ft| ft.is_dir()) {
                "directory"
            } else if file_type.map_or(false, |ft| ft.is_file()) {
                "file"
            } else {
                "other"
            };

            entries.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy().to_string(),
                "type": entry_type
            }));
        }

        // Return as JSON array
        serde_json::to_string(&entries).map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to serialize directory listing: {}", e),
        })
    }
}

#[async_trait::async_trait]
impl Tool for FilesystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read, write, and list files in the base directory. Supports three operations: \"read\" to read file contents, \"write\" to create or overwrite files, and \"list\" to list directory contents. All paths are restricted to the base directory for security."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list"],
                    "description": "Filesystem operation to perform: read file, write file, or list directory"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path relative to the base directory"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file (required for write operation)"
                }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Extract operation
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'operation'. Must be 'read', 'write', or 'list'.".to_string(),
            })?;

        // Extract path
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'path'".to_string(),
            })?;

        match operation {
            "read" => {
                let path = self.validate_path_internal(path_str).await?;
                self.read_file(&path).await
            }
            "write" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: "Missing required parameter 'content' for write operation".to_string(),
                    })?;
                let path = self.validate_path_internal(path_str).await?;
                self.write_file(&path, content).await
            }
            "list" => {
                let path = self.validate_path_internal(path_str).await?;
                self.list_dir(&path).await
            }
            _ => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!(
                    "Invalid operation '{}'. Must be 'read', 'write', or 'list'.",
                    operation
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use crate::utils::paths::is_system_path;

    /// Creates a FilesystemTool with a temporary directory as base
    fn create_test_tool() -> (FilesystemTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let tool = FilesystemTool::new(temp_dir.path().to_path_buf());
        (tool, temp_dir)
    }

    #[test]
    fn test_tool_name() {
        let (tool, _temp) = create_test_tool();
        assert_eq!(tool.name(), "filesystem");
    }

    #[test]
    fn test_tool_description() {
        let (tool, _temp) = create_test_tool();
        assert!(tool.description().contains("Read, write, and list files"));
        assert!(tool.description().contains("read"));
        assert!(tool.description().contains("write"));
        assert!(tool.description().contains("list"));
    }

    #[test]
    fn test_tool_parameters_schema() {
        let (tool, _temp) = create_test_tool();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("read")));
        assert!(params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("write")));
        assert!(params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("list")));
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("operation")));
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("path")));
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let (tool, temp) = create_test_tool();

        // Create a test file
        let test_content = "Hello, World!";
        let test_path = temp.path().join("test.txt");
        let mut file = std::fs::File::create(&test_path).unwrap();
        file.write_all(test_content.as_bytes()).unwrap();

        // Read the file using the tool
        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("read"));
        args.insert("path".to_string(), serde_json::json!("test.txt"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_content);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let (tool, _temp) = create_test_tool();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("read"));
        args.insert("path".to_string(), serde_json::json!("nonexistent.txt"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(msg) => {
                assert!(msg.contains("File not found"));
                assert!(msg.contains("nonexistent.txt"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_read_directory_instead_of_file() {
        let (tool, temp) = create_test_tool();

        // Create a directory
        let dir_path = temp.path().join("testdir");
        std::fs::create_dir(&dir_path).unwrap();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("read"));
        args.insert("path".to_string(), serde_json::json!("testdir"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("is not a file"));
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let (tool, temp) = create_test_tool();

        let test_content = "Test content for writing";

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("write"));
        args.insert("path".to_string(), serde_json::json!("newfile.txt"));
        args.insert("content".to_string(), serde_json::json!(test_content));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Successfully wrote"));

        // Verify file was created
        let file_path = temp.path().join("newfile.txt");
        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_write_file_creates_directories() {
        let (tool, temp) = create_test_tool();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("write"));
        args.insert("path".to_string(), serde_json::json!("subdir/nested/file.txt"));
        args.insert("content".to_string(), serde_json::json!("nested content"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());

        // Verify nested directories were created
        let file_path = temp.path().join("subdir/nested/file.txt");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_write_file_overwrites_existing() {
        let (tool, temp) = create_test_tool();

        // Create existing file
        let file_path = temp.path().join("existing.txt");
        std::fs::write(&file_path, "old content").unwrap();

        let new_content = "new content";

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("write"));
        args.insert("path".to_string(), serde_json::json!("existing.txt"));
        args.insert("content".to_string(), serde_json::json!(new_content));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());

        // Verify content was overwritten
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, new_content);
    }

    #[tokio::test]
    async fn test_write_missing_content_parameter() {
        let (tool, _temp) = create_test_tool();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("write"));
        args.insert("path".to_string(), serde_json::json!("file.txt"));
        // Missing "content" parameter

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("content"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_list_directory_success() {
        let (tool, temp) = create_test_tool();

        // Create test files and directories
        std::fs::write(temp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp.path().join("file2.txt"), "content2").unwrap();
        std::fs::create_dir(temp.path().join("subdir")).unwrap();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("list"));
        args.insert("path".to_string(), serde_json::json!("."));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());

        // Parse JSON result
        let entries: Vec<serde_json::Value> = serde_json::from_str(&result.unwrap()).unwrap();

        // Check that we have the expected entries
        let file_names: Vec<String> = entries
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_string())
            .collect();

        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.txt".to_string()));
        assert!(file_names.contains(&"subdir".to_string()));

        // Check types
        let subdir_entry = entries.iter().find(|e| e["name"] == "subdir").unwrap();
        assert_eq!(subdir_entry["type"], "directory");

        let file_entry = entries.iter().find(|e| e["name"] == "file1.txt").unwrap();
        assert_eq!(file_entry["type"], "file");
    }

    #[tokio::test]
    async fn test_list_file_instead_of_directory() {
        let (tool, temp) = create_test_tool();

        // Create a file
        std::fs::write(temp.path().join("testfile.txt"), "content").unwrap();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("list"));
        args.insert("path".to_string(), serde_json::json!("testfile.txt"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("is not a directory"));
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let (tool, _temp) = create_test_tool();

        // Try to access file outside base directory
        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("read"));
        args.insert("path".to_string(), serde_json::json!("../../../etc/passwd"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        
        // Must be PermissionDenied or SystemPathBlocked - NotFound is NOT acceptable for traversal attacks
        match err {
            ToolError::PermissionDenied { message, .. } => {
                assert!(
                    message.contains("outside") || message.contains("system path"),
                    "Expected security-related message, got: {}",
                    message
                );
            }
            _ => panic!("Expected PermissionDenied for path traversal attack, got {:?}", err),
        }
    }

    #[test]
    fn test_is_system_path() {
        // Unix paths
        assert!(is_system_path(Path::new("/etc/passwd")));
        assert!(is_system_path(Path::new("/root/.bashrc")));
        assert!(is_system_path(Path::new("/sys/kernel")));
        assert!(is_system_path(Path::new("/proc/1/status")));
        assert!(is_system_path(Path::new("/boot/grub")));
        assert!(is_system_path(Path::new("/bin/ls")));
        assert!(is_system_path(Path::new("/usr/bin")));

        // Windows paths (cross-platform security)
        assert!(is_system_path(Path::new("C:\\Windows\\System32")));
        assert!(is_system_path(Path::new("C:\\Program Files\\App")));

        // Non-system paths
        assert!(!is_system_path(Path::new("/home/user/file")));
        assert!(!is_system_path(Path::new("/tmp/test")));
        assert!(!is_system_path(Path::new("./relative/path")));
        assert!(!is_system_path(Path::new("C:\\Users\\user\\file.txt")));
    }

    #[tokio::test]
    async fn test_missing_operation_parameter() {
        let (tool, _temp) = create_test_tool();

        let mut args = HashMap::new();
        // Missing "operation" parameter
        args.insert("path".to_string(), serde_json::json!("file.txt"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("operation"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_missing_path_parameter() {
        let (tool, _temp) = create_test_tool();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("read"));
        // Missing "path" parameter

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("path"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let (tool, _temp) = create_test_tool();

        let mut args = HashMap::new();
        args.insert("operation".to_string(), serde_json::json!("delete")); // Invalid operation
        args.insert("path".to_string(), serde_json::json!("file.txt"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("delete"));
                assert!(message.contains("Invalid operation"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }
}
