//! Security utilities shared between exec and spawn tools
//!
//! Provides a shared command blacklist and validation helpers to avoid duplication
//! between the exec and spawn tool implementations.

use std::path::{Path, PathBuf};

use crate::agent::tools::types::{ToolError, ToolResult};
use crate::utils::paths::{PathValidationError, validate_path};

/// Commands that cannot be executed or spawned for security reasons.
/// This list is shared between the exec and spawn tools.
///
/// # Security
/// Implements NFR-S4: Commands are validated against this blacklist to prevent
/// execution of dangerous system commands.
pub const COMMAND_BLACKLIST: &[&str] = &[
    "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo",
];

/// Checks if a command is in the shared blacklist.
///
/// Extracts the base command name (last path component) and checks
/// case-insensitively against [`COMMAND_BLACKLIST`].
///
/// # Arguments
/// * `command` - The command string to check (may include a path prefix)
///
/// # Returns
/// `true` if the command is blacklisted, `false` otherwise
pub fn is_blacklisted(command: &str) -> bool {
    let base_cmd = std::path::Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command)
        .to_lowercase();

    COMMAND_BLACKLIST.contains(&base_cmd.as_str())
}

/// Validates a working directory path against a base directory.
///
/// # Arguments
/// * `tool_name` - Name of the calling tool (used in error messages)
/// * `base_dir` - The canonicalized base directory that cwd must remain within
/// * `cwd` - The working directory string to validate
///
/// # Returns
/// * `Ok(PathBuf)` - The canonicalized, validated path
/// * `Err(ToolError)` - If the path is invalid or outside the allowed scope
pub async fn validate_cwd(tool_name: &str, base_dir: &Path, cwd: &str) -> ToolResult<PathBuf> {
    validate_path(base_dir, cwd).await.map_err(|e| match e {
        PathValidationError::OutsideBaseDirectory(path) => ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            message: format!(
                "Working directory '{}' is outside the allowed base directory",
                path
            ),
        },
        PathValidationError::SystemPathBlocked(path) => ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            message: format!("Access to system path '{}' is not allowed", path),
        },
        PathValidationError::CanonicalizationFailed { path, source } => {
            ToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                message: format!("Failed to resolve working directory '{}': {}", path, source),
            }
        }
        PathValidationError::InvalidBaseDirectory(msg) => ToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            message: format!("Base directory error: {}", msg),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blacklisted_simple_commands() {
        assert!(is_blacklisted("rm"));
        assert!(is_blacklisted("sudo"));
        assert!(is_blacklisted("dd"));
        assert!(is_blacklisted("mkfs"));
        assert!(is_blacklisted("shutdown"));
        assert!(is_blacklisted("reboot"));
        assert!(is_blacklisted("passwd"));
        assert!(is_blacklisted("visudo"));
    }

    #[test]
    fn test_is_blacklisted_with_paths() {
        assert!(is_blacklisted("/bin/rm"));
        assert!(is_blacklisted("/usr/bin/sudo"));
        assert!(is_blacklisted("./rm"));
    }

    #[test]
    fn test_is_blacklisted_case_insensitive() {
        assert!(is_blacklisted("RM"));
        assert!(is_blacklisted("SUDO"));
        assert!(is_blacklisted("Rm"));
        assert!(is_blacklisted("SuDo"));
    }

    #[test]
    fn test_is_not_blacklisted() {
        assert!(!is_blacklisted("ls"));
        assert!(!is_blacklisted("cat"));
        assert!(!is_blacklisted("echo"));
        assert!(!is_blacklisted("git"));
        assert!(!is_blacklisted("cargo"));
        assert!(!is_blacklisted("pwd"));
        assert!(!is_blacklisted("sleep"));
        assert!(!is_blacklisted("python"));
    }
}
