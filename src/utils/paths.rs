//! Path validation utilities for secure filesystem operations
//!
//! This module provides cross-platform path validation to prevent path traversal
//! attacks and unauthorized access to system directories. All filesystem tools
//! should use these utilities to ensure consistent security.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during path validation
#[derive(Debug, Error)]
pub enum PathValidationError {
    /// Path is outside the allowed base directory (path traversal attempt)
    #[error("Path '{0}' is outside the allowed base directory")]
    OutsideBaseDirectory(String),

    /// Path is a blocked system path
    #[error("Access to system path '{0}' is not allowed")]
    SystemPathBlocked(String),

    /// Failed to canonicalize path
    #[error("Failed to resolve path '{path}': {source}")]
    CanonicalizationFailed {
        path: String,
        source: std::io::Error,
    },

    /// Base directory is invalid or inaccessible
    #[error("Base directory is invalid or inaccessible: {0}")]
    InvalidBaseDirectory(String),
}

/// Validates that a user-provided path is safe to access
///
/// This function performs comprehensive security checks:
/// 1. Resolves the path relative to the base directory
/// 2. Canonicalizes to resolve symlinks and normalize
/// 3. Verifies the path stays within the base directory
/// 4. Blocks access to sensitive system paths
///
/// # Arguments
/// * `base_dir` - The canonicalized base directory (must already be canonical)
/// * `user_path` - The path provided by the user/agent (relative or absolute)
///
/// # Returns
/// * `Ok(PathBuf)` - The canonicalized, validated path safe to access
/// * `Err(PathValidationError)` - If the path is invalid or unsafe
///
/// # Security
/// This function implements NFR-S3: All paths are canonicalized to prevent
/// path traversal attacks (e.g., "../../../etc/passwd")
pub async fn validate_path(
    base_dir: &Path,
    user_path: &str,
) -> Result<PathBuf, PathValidationError> {
    // Join with base directory to create the target path
    let target_path = base_dir.join(user_path);

    // Try to canonicalize the path
    let canonical = match tokio::fs::canonicalize(&target_path).await {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Path doesn't exist - for write operations, we need special handling
            // Canonicalize as much as we can (parent directories) and validate the result
            validate_non_existent_path(base_dir, user_path).await?
        }
        Err(e) => {
            return Err(PathValidationError::CanonicalizationFailed {
                path: user_path.to_string(),
                source: e,
            });
        }
    };

    // Verify the canonical path is within the base directory
    if !canonical.starts_with(base_dir) {
        return Err(PathValidationError::OutsideBaseDirectory(
            user_path.to_string(),
        ));
    }

    // Check against blocked system paths
    if is_system_path(&canonical) {
        return Err(PathValidationError::SystemPathBlocked(
            canonical.display().to_string(),
        ));
    }

    Ok(canonical)
}

/// Validates a path that doesn't exist yet (for write operations)
///
/// This function:
/// 1. Canonicalizes the deepest existing parent directory
/// 2. Appends the remaining path components
/// 3. Validates the result is within the base directory
///
/// # Arguments
/// * `base_dir` - The canonicalized base directory
/// * `user_path` - The path provided by the user
///
/// # Returns
/// The validated target path (may not exist yet)
async fn validate_non_existent_path(
    base_dir: &Path,
    user_path: &str,
) -> Result<PathBuf, PathValidationError> {
    // Build the target path from the base directory
    let target_path = base_dir.join(user_path);

    // Find the deepest existing parent
    let mut current = target_path.as_path();
    let mut components_to_add = Vec::new();

    while !current.exists() {
        if let Some(file_name) = current.file_name() {
            components_to_add.push(file_name);
            current = current.parent().unwrap_or(Path::new(""));
        } else {
            break;
        }
    }

    // Canonicalize the existing parent
    let canonical_parent = tokio::fs::canonicalize(current)
        .await
        .map_err(|e| PathValidationError::CanonicalizationFailed {
            path: user_path.to_string(),
            source: e,
        })?;

    // Build the final path by appending the non-existent components
    let mut result = canonical_parent;
    for component in components_to_add.iter().rev() {
        result.push(component);
    }

    // Verify it's within the base directory
    if !result.starts_with(base_dir) {
        return Err(PathValidationError::OutsideBaseDirectory(
            user_path.to_string(),
        ));
    }

    // Check against blocked system paths
    if is_system_path(&result) {
        return Err(PathValidationError::SystemPathBlocked(
            result.display().to_string(),
        ));
    }

    Ok(result)
}

/// Checks if a path is a sensitive system path that should be blocked
///
/// This function blocks access to system directories on both Unix and Windows:
///
/// **Unix/Linux:**
/// - /etc, /root, /sys, /proc, /boot
/// - /bin, /sbin, /lib, /lib64, /usr
/// - /dev, /var/log
///
/// **Windows:**
/// - C:\Windows, C:\Program Files, C:\Program Files (x86)
/// - C:\ProgramData\Microsoft, C:\System Volume Information
///
/// # Arguments
/// * `path` - The canonicalized path to check
///
/// # Returns
/// `true` if the path should be blocked, `false` otherwise
pub fn is_system_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let path_lower = path_str.to_lowercase();

    // Unix/Linux system paths
    let unix_blocked_prefixes = [
        "/etc", "/root", "/sys", "/proc", "/boot", "/bin", "/sbin", "/lib", "/lib64", "/usr",
        "/dev", "/var/log",
    ];

    for prefix in &unix_blocked_prefixes {
        if path_str.starts_with(prefix) {
            return true;
        }
    }

    // Windows system paths (case-insensitive)
    let windows_blocked_prefixes = [
        "c:\\windows",
        "c:\\program files",
        "c:\\program files (x86)",
        "c:\\programdata\\microsoft",
        "c:\\system volume information",
    ];

    for prefix in &windows_blocked_prefixes {
        if path_lower.starts_with(prefix) {
            return true;
        }
    }

    false
}

/// Canonicalizes a base directory for use in path validation
///
/// This function should be called once during initialization to prepare
/// a base directory for validation operations.
///
/// # Arguments
/// * `base_dir` - The base directory path to canonicalize
///
/// # Returns
/// * `Ok(PathBuf)` - The canonicalized base directory
/// * `Err(PathValidationError)` - If the base directory is invalid
pub async fn canonicalize_base_dir(base_dir: &Path) -> Result<PathBuf, PathValidationError> {
    tokio::fs::canonicalize(base_dir)
        .await
        .map_err(|e| PathValidationError::InvalidBaseDirectory(format!("{}: {}", base_dir.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_validate_existing_path() {
        let temp = TempDir::new().unwrap();
        let base = tokio::fs::canonicalize(temp.path()).await.unwrap();

        // Create a test file
        let test_file = temp.path().join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        // Validate the path
        let result = validate_path(&base, "test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_validate_non_existent_path() {
        let temp = TempDir::new().unwrap();
        let base = tokio::fs::canonicalize(temp.path()).await.unwrap();

        // Validate a path that doesn't exist yet
        let result = validate_path(&base, "new_file.txt").await;
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert!(validated.starts_with(&base));
        assert!(validated.ends_with("new_file.txt"));
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let temp = TempDir::new().unwrap();
        let base = tokio::fs::canonicalize(temp.path()).await.unwrap();

        // Try to escape with ../../../
        let result = validate_path(&base, "../../../etc/passwd").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            PathValidationError::OutsideBaseDirectory(_) => {},
            PathValidationError::SystemPathBlocked(_) => {},
            other => panic!("Expected OutsideBaseDirectory or SystemPathBlocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nested_path_validation() {
        let temp = TempDir::new().unwrap();
        let base = tokio::fs::canonicalize(temp.path()).await.unwrap();

        // Validate nested path that doesn't exist
        let result = validate_path(&base, "subdir/nested/file.txt").await;
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert!(validated.starts_with(&base));
    }

    #[test]
    fn test_is_system_path_unix() {
        assert!(is_system_path(Path::new("/etc/passwd")));
        assert!(is_system_path(Path::new("/root/.bashrc")));
        assert!(is_system_path(Path::new("/sys/kernel")));
        assert!(is_system_path(Path::new("/proc/1/status")));
        assert!(is_system_path(Path::new("/boot/grub")));
        assert!(is_system_path(Path::new("/bin/ls")));
        assert!(is_system_path(Path::new("/usr/bin")));

        assert!(!is_system_path(Path::new("/home/user/file")));
        assert!(!is_system_path(Path::new("/tmp/test")));
    }

    #[test]
    fn test_is_system_path_windows() {
        // Windows paths (case-insensitive)
        assert!(is_system_path(Path::new("C:\\Windows\\System32")));
        assert!(is_system_path(Path::new("C:\\WINDOWS\\system32")));
        assert!(is_system_path(Path::new("C:\\Program Files\\App")));
        assert!(is_system_path(Path::new("c:\\program files (x86)\\app")));
        assert!(is_system_path(Path::new("C:\\ProgramData\\Microsoft\\Windows")));

        assert!(!is_system_path(Path::new("C:\\Users\\user\\file.txt")));
        assert!(!is_system_path(Path::new("D:\\Data\\file.txt")));
    }

    #[tokio::test]
    async fn test_canonicalize_base_dir() {
        let temp = TempDir::new().unwrap();

        let result = canonicalize_base_dir(temp.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_canonicalize_base_dir_invalid() {
        let result = canonicalize_base_dir(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            PathValidationError::InvalidBaseDirectory(_) => {},
            other => panic!("Expected InvalidBaseDirectory, got {:?}", other),
        }
    }
}
