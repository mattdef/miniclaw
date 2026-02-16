//! Spawn tool for the agent
//!
//! This tool provides background process spawning with security constraints.
//! Commands are validated against a blacklist to prevent dangerous operations.
//! Unlike exec tool, spawn returns immediately with a PID and doesn't wait
//! for process completion (fire-and-forget pattern).

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;
use tokio::process::Command;

use crate::agent::tools::types::{Tool, ToolError, ToolExecutionContext, ToolResult};
use crate::utils::paths::{validate_path, PathValidationError};

/// Blacklisted commands that cannot be spawned for security reasons
/// These commands are considered dangerous and are blocked to prevent system damage
const SPAWN_BLACKLIST: &[&str] = &[
    "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo"
];

/// Tool for spawning background processes
///
/// Provides process spawning with security constraints including a blacklist
/// of dangerous commands and path validation for working directories.
///
/// # Security
/// This tool implements NFR-S4: Commands are validated against a blacklist
/// to prevent execution of dangerous system commands.
#[derive(Debug)]
pub struct SpawnTool {
    /// The canonicalized base directory that all cwd operations are restricted to
    base_dir: PathBuf,
    /// Whether to log stdout/stderr from spawned processes
    log_output: bool,
}

impl SpawnTool {
    /// Creates a new SpawnTool with the specified base directory
    ///
    /// # Arguments
    /// * `base_dir` - The root directory that all cwd operations are restricted to
    /// * `log_output` - Whether to log stdout/stderr from spawned processes
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully created SpawnTool
    /// * `Err(ToolError)` - If base directory cannot be canonicalized
    pub fn new(base_dir: PathBuf, log_output: bool) -> Result<Self, ToolError> {
        // Canonicalize the base directory once for performance
        let canonical_base = std::fs::canonicalize(&base_dir)
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "spawn".to_string(),
                message: format!("Failed to canonicalize base directory {:?}: {}", base_dir, e),
            })?;
        
        Ok(Self {
            base_dir: canonical_base,
            log_output,
        })
    }

    /// Checks if a command is in the blacklist
    ///
    /// Blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
    ///
    /// # Arguments
    /// * `command` - The command to check
    ///
    /// # Returns
    /// `true` if the command is blacklisted, `false` otherwise
    fn is_blacklisted(&self, command: &str) -> bool {
        // Extract the base command name (last component of path)
        let base_cmd = std::path::Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command)
            .to_lowercase();

        SPAWN_BLACKLIST.contains(&base_cmd.as_str())
    }

    /// Validates a working directory path
    ///
    /// # Arguments
    /// * `cwd` - The working directory path to validate
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The canonicalized, validated path
    /// * `Err(ToolError)` - If path is invalid or outside allowed scope
    async fn validate_cwd(&self, cwd: &str) -> ToolResult<PathBuf> {
        validate_path(&self.base_dir, cwd)
            .await
            .map_err(|e| match e {
                PathValidationError::OutsideBaseDirectory(path) => ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Working directory '{}' is outside the allowed base directory", path),
                },
                PathValidationError::SystemPathBlocked(path) => ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Access to system path '{}' is not allowed", path),
                },
                PathValidationError::CanonicalizationFailed { path, source } => {
                    ToolError::ExecutionFailed {
                        tool: self.name().to_string(),
                        message: format!("Failed to resolve working directory '{}': {}", path, source),
                    }
                }
                PathValidationError::InvalidBaseDirectory(msg) => ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: format!("Base directory error: {}", msg),
                },
            })
    }

    /// Spawns a command in the background and returns immediately
    ///
    /// # Arguments
    /// * `command` - The command to spawn
    /// * `args` - Command arguments as an array
    /// * `cwd` - Optional working directory
    ///
    /// # Returns
    /// JSON string with success status, pid, and message
    async fn spawn_command(
        &self,
        command: &str,
        args: &[ String ],
        cwd: Option<&std::path::Path>,
    ) -> ToolResult<String> {
        // Check blacklist
        if self.is_blacklisted(command) {
            return Err(ToolError::PermissionDenied {
                tool: self.name().to_string(),
                message: format!("Command not allowed: {}", command),
            });
        }

        // Build the command
        let mut cmd = Command::new(command);
        cmd.args(args);
        
        // Configure stdout/stderr based on log_output setting
        if self.log_output {
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }
        
        // Set working directory if provided
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            // Classify error types for better debugging
            match e.kind() {
                std::io::ErrorKind::NotFound => ToolError::NotFound(
                    format!("Command '{}' not found in system PATH", command)
                ),
                std::io::ErrorKind::PermissionDenied => ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Permission denied spawning command '{}'", command),
                },
                _ => ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: format!("Failed to spawn command '{}': {}", command, e),
                }
            }
        })?;

        // Get the process ID
        let pid = child.id().unwrap_or(0);
        let log_output = self.log_output;
        let command_name = command.to_string();

        // Spawn a background task to track the process (fire-and-forget)
        tokio::spawn(async move {
            if log_output {
                // Capture output if configured
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                
                match child.wait().await {
                    Ok(status) => {
                        let exit_code = status.code().unwrap_or(-1);
                        
                        // Read and log stdout if available
                        if let Some(mut stdout) = stdout {
                            let mut buf = Vec::new();
                            use tokio::io::AsyncReadExt;
                            let _ = stdout.read_to_end(&mut buf).await;
                            let stdout_str = String::from_utf8_lossy(&buf);
                            if !stdout_str.is_empty() {
                                tracing::info!(
                                    pid = %pid,
                                    command = %command_name,
                                    exit_code = %exit_code,
                                    stdout = %stdout_str,
                                    "Spawned process completed"
                                );
                            } else {
                                tracing::info!(
                                    pid = %pid,
                                    command = %command_name,
                                    exit_code = %exit_code,
                                    "Spawned process completed"
                                );
                            }
                        } else {
                            tracing::info!(
                                pid = %pid,
                                command = %command_name,
                                exit_code = %exit_code,
                                "Spawned process completed"
                            );
                        }
                        
                        // Log stderr if available
                        if let Some(mut stderr) = stderr {
                            let mut buf = Vec::new();
                            use tokio::io::AsyncReadExt;
                            let _ = stderr.read_to_end(&mut buf).await;
                            let stderr_str = String::from_utf8_lossy(&buf);
                            if !stderr_str.is_empty() {
                                tracing::warn!(
                                    pid = %pid,
                                    command = %command_name,
                                    stderr = %stderr_str,
                                    "Spawned process stderr"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            pid = %pid,
                            command = %command_name,
                            error = %e,
                            "Spawned process error"
                        );
                    }
                }
            } else {
                // Just wait for completion without capturing output
                match child.wait().await {
                    Ok(status) => {
                        let exit_code = status.code().unwrap_or(-1);
                        tracing::info!(
                            pid = %pid,
                            command = %command_name,
                            exit_code = %exit_code,
                            "Spawned process completed"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            pid = %pid,
                            command = %command_name,
                            error = %e,
                            "Spawned process error"
                        );
                    }
                }
            }
        });

        // Return immediately with success and PID
        let result = serde_json::json!({
            "success": true,
            "pid": pid,
            "message": "Process spawned successfully"
        });

        serde_json::to_string(&result).map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to serialize spawn result: {}", e),
        })
    }
}

#[async_trait::async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn background processes without blocking. Commands are validated against a blacklist of dangerous operations. \
         Arguments must be provided as an array to prevent shell injection. Optional 'cwd' parameter to set working directory. \
         Returns immediately with process ID. Process runs in background and exit code is logged. \
         Blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to spawn (e.g., 'sleep', 'python', 'npm')"
                },
                "args": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Command arguments as array (prevents shell injection)"
                },
                "cwd": {
                    "type": "string",
                    "description": "Optional working directory path (must be within base directory)"
                }
            },
            "required": ["command", "args"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Extract command
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'command'".to_string(),
            })?;

        // Extract args array
        let args_vec = args
            .get("args")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'args' (must be an array)".to_string(),
            })?;

        // Convert args to Vec<String>, ensuring all are strings
        let args_strings: Result<Vec<String>, _> = args_vec
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: format!("Argument at index {} is not a string: {:?}", i, v),
                    })
            })
            .collect();
        
        let args_strings = args_strings?;

        // Extract optional cwd
        let cwd_path = if let Some(cwd) = args.get("cwd").and_then(|v| v.as_str()) {
            Some(self.validate_cwd(cwd).await?)
        } else {
            None
        };

        // Spawn the command
        self.spawn_command(command, &args_strings, cwd_path.as_deref()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a SpawnTool with a temporary directory as base
    fn create_test_tool(log_output: bool) -> (SpawnTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let tool = SpawnTool::new(temp_dir.path().to_path_buf(), log_output).unwrap();
        (tool, temp_dir)
    }

    #[test]
    fn test_tool_name() {
        let (tool, _temp) = create_test_tool(false);
        assert_eq!(tool.name(), "spawn");
    }

    #[test]
    fn test_tool_description() {
        let (tool, _temp) = create_test_tool(false);
        let desc = tool.description();
        assert!(desc.contains("background"));
        assert!(desc.contains("blacklist"));
        assert!(desc.contains("rm"));
        assert!(desc.contains("sudo"));
    }

    #[test]
    fn test_tool_parameters_schema() {
        let (tool, _temp) = create_test_tool(false);
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["command"]["type"] == "string");
        assert!(params["properties"]["args"]["type"] == "array");
        assert!(params["properties"]["cwd"]["type"] == "string");
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("command")));
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("args")));
    }

    #[test]
    fn test_blacklist_simple_commands() {
        let (tool, _temp) = create_test_tool(false);
        
        assert!(tool.is_blacklisted("rm"));
        assert!(tool.is_blacklisted("sudo"));
        assert!(tool.is_blacklisted("dd"));
        assert!(tool.is_blacklisted("mkfs"));
        assert!(tool.is_blacklisted("shutdown"));
        assert!(tool.is_blacklisted("reboot"));
        assert!(tool.is_blacklisted("passwd"));
        assert!(tool.is_blacklisted("visudo"));
    }

    #[test]
    fn test_blacklist_with_paths() {
        let (tool, _temp) = create_test_tool(false);
        
        assert!(tool.is_blacklisted("/bin/rm"));
        assert!(tool.is_blacklisted("/usr/bin/sudo"));
        assert!(tool.is_blacklisted("./rm"));
    }

    #[test]
    fn test_blacklist_case_insensitive() {
        let (tool, _temp) = create_test_tool(false);
        
        assert!(tool.is_blacklisted("RM"));
        assert!(tool.is_blacklisted("SUDO"));
        assert!(tool.is_blacklisted("Rm"));
        assert!(tool.is_blacklisted("SuDo"));
    }

    #[test]
    fn test_non_blacklisted_commands() {
        let (tool, _temp) = create_test_tool(false);
        
        assert!(!tool.is_blacklisted("ls"));
        assert!(!tool.is_blacklisted("cat"));
        assert!(!tool.is_blacklisted("echo"));
        assert!(!tool.is_blacklisted("git"));
        assert!(!tool.is_blacklisted("cargo"));
        assert!(!tool.is_blacklisted("pwd"));
        assert!(!tool.is_blacklisted("sleep"));
        assert!(!tool.is_blacklisted("python"));
    }

    #[tokio::test]
    async fn test_spawn_simple_command() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("echo"));
        args.insert("args".to_string(), serde_json::json!(["hello", "world"]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["pid"].as_u64().unwrap() > 0);
        assert!(output["message"].as_str().unwrap().contains("spawned successfully"));
    }

    #[tokio::test]
    async fn test_spawn_blacklisted_command() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("sudo"));
        args.insert("args".to_string(), serde_json::json!(["whoami"]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { message, .. } => {
                assert!(message.contains("Command not allowed"));
                assert!(message.contains("sudo"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_command_not_found() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("nonexistent_command_xyz"));
        args.insert("args".to_string(), serde_json::json!([]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(message) => {
                assert!(message.contains("not found"));
                assert!(message.contains("nonexistent_command_xyz"));
            }
            other => panic!("Expected NotFound error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_missing_command_param() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        // Missing "command" parameter
        args.insert("args".to_string(), serde_json::json!([]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("command"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_missing_args_param() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("echo"));
        // Missing "args" parameter

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("args"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_with_cwd() {
        let (tool, temp) = create_test_tool(false);

        // Create a subdirectory
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("pwd"));
        args.insert("args".to_string(), serde_json::json!([]));
        args.insert("cwd".to_string(), serde_json::json!("subdir"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["pid"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_spawn_cwd_outside_base() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("pwd"));
        args.insert("args".to_string(), serde_json::json!([]));
        args.insert("cwd".to_string(), serde_json::json!("../../../etc"));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::PermissionDenied { message, .. } => {
                assert!(message.contains("outside") || message.contains("system path"));
            }
            _ => panic!("Expected PermissionDenied error for path traversal"),
        }
    }

    #[tokio::test]
    async fn test_shell_injection_prevented() {
        let (tool, _temp) = create_test_tool(false);

        // This should be treated as a literal argument, not execute shell commands
        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("echo"));
        args.insert("args".to_string(), serde_json::json!(["; rm -rf /"]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        // Should succeed - the dangerous string is just echoed
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
    }

    #[tokio::test]
    async fn test_args_must_be_strings() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("echo"));
        // Pass a number instead of a string
        args.insert("args".to_string(), serde_json::json!([123, "hello"]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        // Should fail because argument is not a string
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("not a string"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_returns_immediately() {
        let (tool, _temp) = create_test_tool(false);

        let mut args = HashMap::new();
        // Use a command that would take a long time
        args.insert("command".to_string(), serde_json::json!("sleep"));
        args.insert("args".to_string(), serde_json::json!(["5"]));

        let ctx = ToolExecutionContext::default();
        let start = std::time::Instant::now();
        let result = tool.execute(args, &ctx).await;
        let elapsed = start.elapsed();

        // Should return immediately (well under 5 seconds)
        assert!(result.is_ok());
        assert!(elapsed.as_secs() < 2, "Spawn should return immediately, but took {:?}", elapsed);
        
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["pid"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_constructor_with_invalid_directory() {
        // Test that constructor returns error for nonexistent directory
        let invalid_path = PathBuf::from("/nonexistent/directory/that/does/not/exist");
        let result = SpawnTool::new(invalid_path, false);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("Failed to canonicalize"));
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_with_log_output_enabled() {
        let (tool, _temp) = create_test_tool(true);

        let mut args = HashMap::new();
        args.insert("command".to_string(), serde_json::json!("echo"));
        args.insert("args".to_string(), serde_json::json!(["test output"]));

        let ctx = ToolExecutionContext::default();
        let result = tool.execute(args, &ctx).await;

        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["pid"].as_u64().unwrap() > 0);
    }
}
