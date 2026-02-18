//! Workspace module for miniclaw
//!
//! This module handles the creation, management, and loading of the workspace structure
//! including all markdown configuration files (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md).

pub mod templates;

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Workspace directory does not exist: {0:?}")]
    DirectoryNotFound(PathBuf),

    #[error("Failed to create workspace directory: {0:?}")]
    DirectoryCreationFailed(PathBuf, #[source] std::io::Error),

    #[error("Failed to create workspace file {0}: {1:?}")]
    FileCreationFailed(String, PathBuf, #[source] std::io::Error),

    #[error("Failed to set permissions for {0}: {1:?}")]
    PermissionError(String, #[source] std::io::Error),

    #[error("Workspace error: {0}")]
    Other(String),
}

/// Represents the loaded workspace context for the agent
#[derive(Debug, Clone, Default)]
pub struct WorkspaceContext {
    /// Content of SOUL.md - agent personality
    pub soul: String,
    /// Content of AGENTS.md - agent behavior guidelines
    pub agents: String,
    /// Content of USER.md - user profile and preferences
    pub user: String,
    /// Content of TOOLS.md - tool documentation
    pub tools: String,
    /// Content of HEARTBEAT.md - scheduled tasks
    pub heartbeat: String,
}

impl WorkspaceContext {
    /// Create a new empty WorkspaceContext
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all context fields have content
    pub fn is_complete(&self) -> bool {
        !self.soul.is_empty()
            && !self.agents.is_empty()
            && !self.user.is_empty()
            && !self.tools.is_empty()
            && !self.heartbeat.is_empty()
    }

    /// Get the number of loaded files
    pub fn loaded_count(&self) -> usize {
        let mut count = 0;
        if !self.soul.is_empty() {
            count += 1;
        }
        if !self.agents.is_empty() {
            count += 1;
        }
        if !self.user.is_empty() {
            count += 1;
        }
        if !self.tools.is_empty() {
            count += 1;
        }
        if !self.heartbeat.is_empty() {
            count += 1;
        }
        count
    }
}

/// Initialize the complete workspace structure
///
/// Creates the workspace directory and all markdown files with default content.
/// If files already exist, they are preserved (not overwritten).
///
/// # Arguments
/// * `base_path` - The base miniclaw directory (e.g., ~/.miniclaw)
/// * `verbose` - If true, prints progress information
///
/// # Returns
/// * `Ok(())` - Workspace initialized successfully
/// * `Err` - If directory or file creation fails
///
/// # Example
/// ```no_run
/// use miniclaw::workspace::initialize_workspace;
/// use std::path::PathBuf;
///
/// let base_path = PathBuf::from("/home/user/.miniclaw");
/// initialize_workspace(&base_path, true).unwrap();
/// ```
pub fn initialize_workspace(base_path: &Path, verbose: bool) -> Result<()> {
    let workspace_path = base_path.join("workspace");

    if verbose {
        tracing::debug!(path = %workspace_path.display(), "Initializing workspace");
    }

    // Create workspace directory if it doesn't exist
    if !workspace_path.exists() {
        fs::create_dir_all(&workspace_path)
            .map_err(|e| WorkspaceError::DirectoryCreationFailed(workspace_path.clone(), e))?;

        // Set directory permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&workspace_path, permissions).map_err(|e| {
                WorkspaceError::PermissionError("workspace directory".to_string(), e)
            })?;
        }

        if verbose {
            tracing::info!(path = %workspace_path.display(), "Created workspace directory");
        }
    } else if verbose {
        tracing::info!(path = %workspace_path.display(), "Workspace directory already exists");
    }

    // Create all markdown files
    for (filename, content) in templates::WORKSPACE_FILES {
        let file_path = workspace_path.join(filename);

        if !file_path.exists() {
            fs::write(&file_path, content).map_err(|e| {
                WorkspaceError::FileCreationFailed(filename.to_string(), file_path.clone(), e)
            })?;

            // Set file permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(0o644);
                fs::set_permissions(&file_path, permissions)
                    .map_err(|e| WorkspaceError::PermissionError(filename.to_string(), e))?;
            }

            if verbose {
                tracing::info!(path = %file_path.display(), "Created file");
            }
        } else if verbose {
            tracing::info!(path = %file_path.display(), "File already exists (preserved)");
        }
    }

    // Initialize skills directory (Story 2.4)
    crate::skills::initialize_skills_directory(&workspace_path, verbose)?;

    // Initialize sessions directory (Story 2.5)
    initialize_sessions_directory(&workspace_path, verbose)?;

    if verbose {
        tracing::info!("Workspace initialization complete");
    }

    Ok(())
}

/// Initialize the sessions directory for persistent session storage
///
/// Creates the sessions subdirectory within the workspace.
///
/// # Arguments
/// * `workspace_path` - The workspace directory path
/// * `verbose` - If true, prints progress information
///
/// # Returns
/// * `Ok(())` - Directory created successfully
/// * `Err` - If directory creation fails
fn initialize_sessions_directory(workspace_path: &Path, verbose: bool) -> Result<()> {
    let sessions_path = workspace_path.join("sessions");

    if !sessions_path.exists() {
        fs::create_dir_all(&sessions_path)
            .map_err(|e| WorkspaceError::DirectoryCreationFailed(sessions_path.clone(), e))?;

        // Set directory permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&sessions_path, permissions).map_err(|e| {
                WorkspaceError::PermissionError("sessions directory".to_string(), e)
            })?;
        }

        if verbose {
            tracing::info!(path = %sessions_path.display(), "Created sessions directory");
        }
    } else if verbose {
        tracing::info!(path = %sessions_path.display(), "Sessions directory already exists");
    }

    Ok(())
}

/// Load all workspace files into a WorkspaceContext
///
/// Reads all markdown files from the workspace directory. If a file is missing
/// or cannot be read, that field will be empty but the operation continues.
///
/// # Arguments
/// * `base_path` - The base miniclaw directory (e.g., ~/.miniclaw)
///
/// # Returns
/// * `Ok(WorkspaceContext)` - Context with loaded content
/// * `Err` - If the workspace directory doesn't exist
///
/// # Example
/// ```no_run
/// use miniclaw::workspace::load_workspace_context;
/// use std::path::PathBuf;
///
/// let base_path = PathBuf::from("/home/user/.miniclaw");
/// let context = load_workspace_context(&base_path).unwrap();
/// println!("Loaded {} files", context.loaded_count());
/// ```
pub fn load_workspace_context(base_path: &Path) -> Result<WorkspaceContext> {
    let workspace_path = base_path.join("workspace");

    if !workspace_path.exists() {
        return Err(WorkspaceError::DirectoryNotFound(workspace_path).into());
    }

    let mut context = WorkspaceContext::new();

    // Load SOUL.md
    let soul_path = workspace_path.join("SOUL.md");
    if soul_path.exists() {
        match fs::read_to_string(&soul_path) {
            Ok(content) => context.soul = content,
            Err(e) => {
                tracing::warn!(path = %soul_path.display(), error = %e, "Failed to read SOUL.md")
            }
        }
    }

    // Load AGENTS.md
    let agents_path = workspace_path.join("AGENTS.md");
    if agents_path.exists() {
        match fs::read_to_string(&agents_path) {
            Ok(content) => context.agents = content,
            Err(e) => {
                tracing::warn!(path = %agents_path.display(), error = %e, "Failed to read AGENTS.md")
            }
        }
    }

    // Load USER.md
    let user_path = workspace_path.join("USER.md");
    if user_path.exists() {
        match fs::read_to_string(&user_path) {
            Ok(content) => context.user = content,
            Err(e) => {
                tracing::warn!(path = %user_path.display(), error = %e, "Failed to read USER.md")
            }
        }
    }

    // Load TOOLS.md
    let tools_path = workspace_path.join("TOOLS.md");
    if tools_path.exists() {
        match fs::read_to_string(&tools_path) {
            Ok(content) => context.tools = content,
            Err(e) => {
                tracing::warn!(path = %tools_path.display(), error = %e, "Failed to read TOOLS.md")
            }
        }
    }

    // Load HEARTBEAT.md
    let heartbeat_path = workspace_path.join("HEARTBEAT.md");
    if heartbeat_path.exists() {
        match fs::read_to_string(&heartbeat_path) {
            Ok(content) => context.heartbeat = content,
            Err(e) => {
                tracing::warn!(path = %heartbeat_path.display(), error = %e, "Failed to read HEARTBEAT.md")
            }
        }
    }

    Ok(context)
}

/// Check and recreate missing workspace files
///
/// Scans the workspace for missing files and recreates them with default content.
/// Existing files are always preserved and never modified.
///
/// # Arguments
/// * `base_path` - The base miniclaw directory (e.g., ~/.miniclaw)
/// * `verbose` - If true, prints progress information
///
/// # Returns
/// * `Ok(())` - Repair completed successfully
/// * `Err` - If file creation fails
///
/// # Example
/// ```no_run
/// use miniclaw::workspace::repair_workspace;
/// use std::path::PathBuf;
///
/// let base_path = PathBuf::from("/home/user/.miniclaw");
/// repair_workspace(&base_path, true).unwrap();
/// ```
pub fn repair_workspace(base_path: &Path, verbose: bool) -> Result<()> {
    let workspace_path = base_path.join("workspace");

    if !workspace_path.exists() {
        return Err(WorkspaceError::DirectoryNotFound(workspace_path).into());
    }

    if verbose {
        tracing::info!("Checking workspace for missing files...");
    }

    let mut recreated_count = 0;

    for (filename, content) in templates::WORKSPACE_FILES {
        let file_path = workspace_path.join(filename);

        if !file_path.exists() {
            fs::write(&file_path, content).map_err(|e| {
                WorkspaceError::FileCreationFailed(filename.to_string(), file_path.clone(), e)
            })?;

            // Set file permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(0o644);
                let _ = fs::set_permissions(&file_path, permissions);
            }

            if verbose {
                tracing::info!(path = %file_path.display(), "Recreated missing file");
            }
            recreated_count += 1;
        }
    }

    if verbose {
        if recreated_count == 0 {
            tracing::info!("All workspace files present. No repairs needed.");
        } else {
            tracing::info!(recreated = recreated_count, "Repair complete");
        }
    }

    Ok(())
}

/// Get the list of workspace files that should exist
///
/// Returns a vector of filenames that are expected in the workspace directory.
///
/// # Returns
/// * `Vec<&str>` - List of workspace filenames
pub fn get_workspace_files() -> Vec<&'static str> {
    templates::WORKSPACE_FILES
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

/// Check if a specific workspace file exists
///
/// # Arguments
/// * `base_path` - The base miniclaw directory
/// * `filename` - The filename to check (e.g., "SOUL.md")
///
/// # Returns
/// * `true` - If the file exists
/// * `false` - If the file doesn't exist or workspace path is invalid
pub fn workspace_file_exists(base_path: &Path, filename: &str) -> bool {
    let workspace_path = base_path.join("workspace");
    let file_path = workspace_path.join(filename);
    file_path.exists()
}

/// Get the full path to a workspace file
///
/// # Arguments
/// * `base_path` - The base miniclaw directory
/// * `filename` - The filename (e.g., "SOUL.md")
///
/// # Returns
/// * `PathBuf` - Full path to the workspace file
pub fn get_workspace_file_path(base_path: &Path, filename: &str) -> PathBuf {
    base_path.join("workspace").join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        (temp_dir, base_path)
    }

    #[test]
    fn test_initialize_workspace_creates_directory() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        assert!(!workspace_path.exists());
        initialize_workspace(&base_path, false).unwrap();
        assert!(workspace_path.exists());
        assert!(workspace_path.is_dir());
    }

    #[test]
    fn test_initialize_workspace_creates_all_files() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        initialize_workspace(&base_path, false).unwrap();

        for (filename, _) in templates::WORKSPACE_FILES {
            let file_path = workspace_path.join(filename);
            assert!(file_path.exists(), "{} should exist", filename);
            assert!(file_path.is_file(), "{} should be a file", filename);
        }

        // Verify sessions directory creation (Story 2.5)
        let sessions_path = workspace_path.join("sessions");
        assert!(sessions_path.exists());
        assert!(sessions_path.is_dir());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&sessions_path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
        }
    }

    #[test]
    fn test_initialize_workspace_preserves_existing_files() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        // Create workspace directory and one file with custom content
        fs::create_dir_all(&workspace_path).unwrap();
        let soul_path = workspace_path.join("SOUL.md");
        fs::write(&soul_path, "Custom SOUL content").unwrap();

        // Initialize workspace
        initialize_workspace(&base_path, false).unwrap();

        // Check that SOUL.md still has custom content
        let content = fs::read_to_string(&soul_path).unwrap();
        assert_eq!(content, "Custom SOUL content");

        // Check that other files were created
        let agents_path = workspace_path.join("AGENTS.md");
        assert!(agents_path.exists());
        let agents_content = fs::read_to_string(&agents_path).unwrap();
        assert!(agents_content.contains("Agent Guidelines"));
    }

    #[test]
    fn test_load_workspace_context_loads_all_files() {
        let (_temp_dir, base_path) = create_temp_workspace();

        initialize_workspace(&base_path, false).unwrap();

        let context = load_workspace_context(&base_path).unwrap();

        assert!(!context.soul.is_empty());
        assert!(!context.agents.is_empty());
        assert!(!context.user.is_empty());
        assert!(!context.tools.is_empty());
        assert!(!context.heartbeat.is_empty());
        assert!(context.is_complete());
        assert_eq!(context.loaded_count(), 5);
    }

    #[test]
    fn test_load_workspace_context_missing_directory() {
        let (_temp_dir, base_path) = create_temp_workspace();

        let result = load_workspace_context(&base_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_load_workspace_context_partial_files() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        // Create only some files
        fs::create_dir_all(&workspace_path).unwrap();
        fs::write(workspace_path.join("SOUL.md"), "SOUL content").unwrap();
        fs::write(workspace_path.join("AGENTS.md"), "AGENTS content").unwrap();

        let context = load_workspace_context(&base_path).unwrap();

        assert_eq!(context.soul, "SOUL content");
        assert_eq!(context.agents, "AGENTS content");
        assert!(context.user.is_empty());
        assert!(context.tools.is_empty());
        assert!(context.heartbeat.is_empty());
        assert!(!context.is_complete());
        assert_eq!(context.loaded_count(), 2);
    }

    #[test]
    fn test_repair_workspace_recreates_missing_files() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        // Initialize full workspace
        initialize_workspace(&base_path, false).unwrap();

        // Delete two files
        fs::remove_file(workspace_path.join("USER.md")).unwrap();
        fs::remove_file(workspace_path.join("HEARTBEAT.md")).unwrap();

        assert!(!workspace_path.join("USER.md").exists());
        assert!(!workspace_path.join("HEARTBEAT.md").exists());

        // Repair workspace
        repair_workspace(&base_path, false).unwrap();

        // Check files are recreated
        assert!(workspace_path.join("USER.md").exists());
        assert!(workspace_path.join("HEARTBEAT.md").exists());

        // Check they have default content
        let user_content = fs::read_to_string(workspace_path.join("USER.md")).unwrap();
        assert!(user_content.contains("User Profile"));
    }

    #[test]
    fn test_repair_workspace_preserves_existing_files() {
        let (_temp_dir, base_path) = create_temp_workspace();
        let workspace_path = base_path.join("workspace");

        // Initialize and customize one file
        initialize_workspace(&base_path, false).unwrap();
        fs::write(workspace_path.join("SOUL.md"), "Custom SOUL").unwrap();

        // Delete another file
        fs::remove_file(workspace_path.join("TOOLS.md")).unwrap();

        // Repair
        repair_workspace(&base_path, false).unwrap();

        // Custom SOUL should be preserved
        let soul_content = fs::read_to_string(workspace_path.join("SOUL.md")).unwrap();
        assert_eq!(soul_content, "Custom SOUL");

        // TOOLS should be recreated
        let tools_content = fs::read_to_string(workspace_path.join("TOOLS.md")).unwrap();
        assert!(tools_content.contains("Tool Documentation"));
    }

    #[test]
    fn test_repair_workspace_fails_without_directory() {
        let (_temp_dir, base_path) = create_temp_workspace();

        let result = repair_workspace(&base_path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_get_workspace_files() {
        let files = get_workspace_files();
        assert_eq!(files.len(), 5);
        assert!(files.contains(&"SOUL.md"));
        assert!(files.contains(&"AGENTS.md"));
        assert!(files.contains(&"USER.md"));
        assert!(files.contains(&"TOOLS.md"));
        assert!(files.contains(&"HEARTBEAT.md"));
    }

    #[test]
    fn test_workspace_file_exists() {
        let (_temp_dir, base_path) = create_temp_workspace();

        initialize_workspace(&base_path, false).unwrap();

        assert!(workspace_file_exists(&base_path, "SOUL.md"));
        assert!(workspace_file_exists(&base_path, "AGENTS.md"));
        assert!(!workspace_file_exists(&base_path, "NONEXISTENT.md"));
    }

    #[test]
    fn test_get_workspace_file_path() {
        let base_path = PathBuf::from("/home/user/.miniclaw");

        let soul_path = get_workspace_file_path(&base_path, "SOUL.md");
        assert_eq!(
            soul_path,
            PathBuf::from("/home/user/.miniclaw/workspace/SOUL.md")
        );

        let agents_path = get_workspace_file_path(&base_path, "AGENTS.md");
        assert_eq!(
            agents_path,
            PathBuf::from("/home/user/.miniclaw/workspace/AGENTS.md")
        );
    }

    #[test]
    fn test_workspace_context_new() {
        let context = WorkspaceContext::new();
        assert!(context.soul.is_empty());
        assert!(context.agents.is_empty());
        assert!(context.user.is_empty());
        assert!(context.tools.is_empty());
        assert!(context.heartbeat.is_empty());
        assert!(!context.is_complete());
        assert_eq!(context.loaded_count(), 0);
    }

    #[test]
    fn test_workspace_context_is_complete() {
        let mut context = WorkspaceContext::new();
        assert!(!context.is_complete());

        context.soul = "SOUL".to_string();
        assert!(!context.is_complete());

        context.agents = "AGENTS".to_string();
        context.user = "USER".to_string();
        context.tools = "TOOLS".to_string();
        context.heartbeat = "HEARTBEAT".to_string();
        assert!(context.is_complete());
    }

    #[test]
    fn test_workspace_context_loaded_count() {
        let mut context = WorkspaceContext::new();
        assert_eq!(context.loaded_count(), 0);

        context.soul = "SOUL".to_string();
        assert_eq!(context.loaded_count(), 1);

        context.agents = "AGENTS".to_string();
        assert_eq!(context.loaded_count(), 2);
    }
}
