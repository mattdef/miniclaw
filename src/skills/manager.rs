//! Skills manager for miniclaw
//!
//! This module provides the SkillsManager for creating, reading, updating, and deleting
//! skills via the skill management tools.

use crate::skills::constants::BUILT_IN_TOOLS;
use crate::skills::types::SkillParameter;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Metadata for a skill (for listing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
}

impl SkillMetadata {
    /// Create new skill metadata
    pub fn new(name: &str, description: &str, created_at: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            created_at: created_at.to_string(),
        }
    }
}

/// A skill definition for management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagedSkill {
    /// Skill name (snake_case)
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill parameters (JSON schema style)
    pub parameters: Vec<SkillParameter>,
    /// Implementation instructions or code
    pub implementation: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ManagedSkill {
    /// Create a new managed skill
    pub fn new(
        name: &str,
        description: &str,
        parameters: Vec<SkillParameter>,
        implementation: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            implementation: implementation.to_string(),
            created_at: Utc::now(),
        }
    }
}

/// Error types for skill management operations
#[derive(thiserror::Error, Debug)]
pub enum SkillManagerError {
    /// Invalid skill name
    #[error("Invalid skill name '{name}': {reason}")]
    InvalidName { name: String, reason: String },

    /// Name conflict with existing skill
    #[error("Skill '{name}' already exists")]
    NameConflict { name: String },

    /// Conflict with built-in tool name
    #[error("Name '{name}' conflicts with built-in tool")]
    BuiltInToolConflict { name: String },

    /// Skill not found
    #[error("Skill '{name}' not found")]
    SkillNotFound { name: String },

    /// Directory creation failed
    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed { path: String, #[source] source: std::io::Error },

    /// File write failed
    #[error("Failed to write file '{path}': {source}")]
    FileWriteFailed { path: String, #[source] source: std::io::Error },

    /// File read failed
    #[error("Failed to read file '{path}': {source}")]
    FileReadFailed { path: String, #[source] source: std::io::Error },

    /// Directory removal failed
    #[error("Failed to remove directory '{path}': {source}")]
    DirectoryRemovalFailed { path: String, #[source] source: std::io::Error },

    /// Built-in skill deletion attempt
    #[error("Cannot delete built-in skill '{name}'")]
    BuiltInSkillDeletion { name: String },

    /// Invalid schema
    #[error("Invalid parameter schema: {0}")]
    InvalidSchema(String),
}

/// Manager for skill CRUD operations
pub struct SkillsManager {
    /// In-memory cache of skills
    skills: Arc<RwLock<HashMap<String, ManagedSkill>>>,
    /// Skills directory path
    skills_dir: PathBuf,
}

impl SkillsManager {
    /// Maximum length for skill names
    const MAX_NAME_LENGTH: usize = 50;

    /// Create a new SkillsManager
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace directory
    pub fn new(workspace_path: PathBuf) -> Self {
        let skills_dir = workspace_path.join("skills");
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            skills_dir,
        }
    }

    /// Get the skills directory path
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    /// Load all skills from disk into memory cache
    ///
    /// Scans the skills directory and loads all SKILL.md files
    /// 
    /// FIX #5: Log errors for invalid skills instead of silently ignoring them
    pub async fn load_skills(&self) -> Result<(), SkillManagerError> {
        let mut skills = self.skills.write().await;
        skills.clear();

        if !self.skills_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.skills_dir)
            .await
            .map_err(|e| SkillManagerError::FileReadFailed {
                path: self.skills_dir.to_string_lossy().to_string(),
                source: e,
            })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| SkillManagerError::FileReadFailed {
            path: self.skills_dir.to_string_lossy().to_string(),
            source: e,
        })? {
            let path = entry.path();
            if path.is_dir() {
                let skill_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if !skill_name.is_empty() && !skill_name.starts_with('.') {
                    let skill_md_path = path.join("SKILL.md");
                    if skill_md_path.exists() {
                        // Parse the skill file to get metadata
                        match tokio::fs::read_to_string(&skill_md_path).await {
                            Ok(content) => {
                                match Self::parse_skill_md(&skill_name, &content) {
                                    Ok(skill) => {
                                        skills.insert(skill_name.clone(), skill);
                                    }
                                    Err(e) => {
                                        // FIX #5: Log parsing errors instead of silently ignoring
                                        tracing::warn!(
                                            skill_name = %skill_name,
                                            error = %e,
                                            "Failed to parse SKILL.md for skill '{}': {}",
                                            skill_name, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                // FIX #5: Log file read errors
                                tracing::warn!(
                                    skill_name = %skill_name,
                                    path = %skill_md_path.display(),
                                    error = %e,
                                    "Failed to read SKILL.md for skill '{}': {}",
                                    skill_name, e
                                );
                            }
                        }
                    }
                }
            }
        }
        
        // FIX #4: Explicitly drop entries to close directory handle
        drop(entries);

        Ok(())
    }

    /// Create a new skill
    ///
    /// # Arguments
    /// * `name` - Skill name (must be snake_case)
    /// * `description` - Skill description
    /// * `parameters` - Parameter definitions
    /// * `implementation` - Implementation instructions
    pub async fn create_skill(
        &self,
        name: String,
        description: String,
        parameters: Vec<SkillParameter>,
        implementation: String,
    ) -> Result<ManagedSkill, SkillManagerError> {
        // Validate name
        Self::validate_skill_name(&name)?;
        Self::check_name_conflict(&name)?;

        // FIX #2: Use write lock for entire operation to prevent race condition
        // FIX #6: Check filesystem existence, not just cache
        let skill_dir = self.skills_dir.join(&name);
        
        // Acquire write lock to prevent concurrent creation
        let mut skills = self.skills.write().await;
        
        // Check uniqueness in cache
        if skills.contains_key(&name) {
            return Err(SkillManagerError::NameConflict { name: name.clone() });
        }
        
        // Check filesystem existence (handles app restart case)
        if skill_dir.exists() {
            return Err(SkillManagerError::NameConflict { name: name.clone() });
        }

        // Create skill directory
        tokio::fs::create_dir_all(&skill_dir)
            .await
            .map_err(|e| SkillManagerError::DirectoryCreationFailed {
                path: skill_dir.to_string_lossy().to_string(),
                source: e,
            })?;

        // Create skill
        let skill = ManagedSkill::new(&name, &description, parameters.clone(), &implementation);

        // Generate SKILL.md content
        let skill_md_content = Self::generate_skill_md(&skill);

        // Write SKILL.md
        let skill_md_path = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_md_path, skill_md_content)
            .await
            .map_err(|e| SkillManagerError::FileWriteFailed {
                path: skill_md_path.to_string_lossy().to_string(),
                source: e,
            })?;

        // Add to cache (still under write lock)
        skills.insert(name.clone(), skill.clone());

        Ok(skill)
    }

    /// List all skills
    ///
    /// Returns metadata for all skills
    pub async fn list_skills(&self) -> Result<Vec<SkillMetadata>, SkillManagerError> {
        let skills = self.skills.read().await;

        let metadata: Vec<SkillMetadata> = skills
            .values()
            .map(|skill| SkillMetadata::new(
                &skill.name,
                &skill.description,
                &skill.created_at.to_rfc3339(),
            ))
            .collect();

        Ok(metadata)
    }

    /// Read a skill's full content
    ///
    /// # Arguments
    /// * `name` - Skill name
    pub async fn read_skill(&self, name: &str) -> Result<String, SkillManagerError> {
        let skills = self.skills.read().await;

        if !skills.contains_key(name) {
            return Err(SkillManagerError::SkillNotFound { name: name.to_string() });
        }

        let skill_md_path = self.skills_dir.join(name).join("SKILL.md");

        let content = tokio::fs::read_to_string(&skill_md_path)
            .await
            .map_err(|e| SkillManagerError::FileReadFailed {
                path: skill_md_path.to_string_lossy().to_string(),
                source: e,
            })?;

        Ok(content)
    }

    /// Delete a skill
    ///
    /// # Arguments
    /// * `name` - Skill name
    /// * `_built_in_tools` - Deprecated parameter (kept for API compatibility)
    /// 
    /// FIX #7: Use centralized BUILT_IN_TOOLS constant as single source of truth
    pub async fn delete_skill(
        &self,
        name: &str,
        _built_in_tools: &[String],
    ) -> Result<(), SkillManagerError> {
        // Check if it's a built-in tool using centralized constant
        if BUILT_IN_TOOLS.contains(&name) {
            return Err(SkillManagerError::BuiltInSkillDeletion { name: name.to_string() });
        }

        // Check if skill exists
        {
            let skills = self.skills.read().await;
            if !skills.contains_key(name) {
                return Err(SkillManagerError::SkillNotFound { name: name.to_string() });
            }
        }

        // Delete skill directory
        let skill_dir = self.skills_dir.join(name);
        tokio::fs::remove_dir_all(&skill_dir)
            .await
            .map_err(|e| SkillManagerError::DirectoryRemovalFailed {
                path: skill_dir.to_string_lossy().to_string(),
                source: e,
            })?;

        // Remove from cache
        {
            let mut skills = self.skills.write().await;
            skills.remove(name);
        }

        Ok(())
    }

    /// Check if a skill exists
    pub async fn skill_exists(&self, name: &str) -> bool {
        let skills = self.skills.read().await;
        skills.contains_key(name)
    }

    /// Validate skill name format
    ///
    /// Must be snake_case: lowercase letters, numbers, underscores
    /// Must start with a letter
    /// Max 50 characters
    /// Cannot contain path separators or traversal sequences
    fn validate_skill_name(name: &str) -> Result<(), SkillManagerError> {
        if name.is_empty() {
            return Err(SkillManagerError::InvalidName {
                name: name.to_string(),
                reason: "Name cannot be empty".to_string(),
            });
        }

        if name.len() > Self::MAX_NAME_LENGTH {
            return Err(SkillManagerError::InvalidName {
                name: name.to_string(),
                reason: format!("Name exceeds maximum length of {} characters", Self::MAX_NAME_LENGTH),
            });
        }

        // Check for path traversal attempts (SECURITY FIX #1)
        if name.contains("..") || name.contains('/') || name.contains('\\') {
            return Err(SkillManagerError::InvalidName {
                name: name.to_string(),
                reason: "Name cannot contain path separators or '..' sequences (security violation)".to_string(),
            });
        }

        // Check first character is a letter
        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_lowercase() {
            return Err(SkillManagerError::InvalidName {
                name: name.to_string(),
                reason: "Name must start with a lowercase letter".to_string(),
            });
        }

        // Check all characters are valid
        for (i, c) in name.chars().enumerate() {
            if i == 0 {
                continue; // Already checked first character
            }
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                return Err(SkillManagerError::InvalidName {
                    name: name.to_string(),
                    reason: format!("Invalid character '{}' at position {}. Only lowercase letters, numbers, and underscores allowed", c, i),
                });
            }
        }

        Ok(())
    }

    /// Check if name conflicts with built-in tools
    fn check_name_conflict(name: &str) -> Result<(), SkillManagerError> {
        if BUILT_IN_TOOLS.contains(&name) {
            return Err(SkillManagerError::BuiltInToolConflict { name: name.to_string() });
        }
        Ok(())
    }

    /// Generate SKILL.md content from a skill
    fn generate_skill_md(skill: &ManagedSkill) -> String {
        let mut content = format!(
            "# Skill: {}\n\n## Description\n{}\n\n## Parameters\n",
            skill.name, skill.description
        );

        if skill.parameters.is_empty() {
            content.push_str("None\n");
        } else {
            content.push_str("\n| Name | Type | Required | Description |\n");
            content.push_str("|------|------|----------|-------------|\n");
            for param in &skill.parameters {
                content.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    param.name,
                    param.param_type,
                    if param.required { "Yes" } else { "No" },
                    param.description
                ));
            }
        }

        content.push_str(&format!(
            "\n## Implementation\n{}\n\n## Metadata\n- **Created**: {}\n- **Version**: 1.0.0\n",
            skill.implementation,
            skill.created_at.to_rfc3339()
        ));

        content
    }

    /// Parse SKILL.md content into a ManagedSkill
    ///
    /// Expected format:
    /// ```markdown
    /// # Skill: {name}
    /// 
    /// ## Description
    /// {description text}
    /// 
    /// ## Parameters
    /// | Name | Type | Required | Description |
    /// |------|------|----------|-------------|
    /// | param1 | string | Yes | Description |
    /// 
    /// ## Implementation
    /// {implementation text}
    /// 
    /// ## Metadata
    /// - **Created**: 2026-02-16T10:00:00Z
    /// - **Version**: 1.0.0
    /// ```
    /// 
    /// FIX #3: Parse created_at from Metadata section instead of using Utc::now()
    fn parse_skill_md(name: &str, content: &str) -> Result<ManagedSkill, SkillManagerError> {
        // Extract description
        let description = content
            .lines()
            .find(|line| line.starts_with("## Description"))
            .and_then(|_line| {
                content.lines()
                    .skip_while(|l| !l.starts_with("## Description"))
                    .nth(1)
            })
            .unwrap_or("")
            .trim()
            .to_string();

        // Extract implementation
        let implementation = content
            .lines()
            .skip_while(|line| !line.starts_with("## Implementation"))
            .skip(1)
            .take_while(|line| !line.starts_with("## Metadata"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        // FIX #3: Parse created_at timestamp from Metadata section
        let created_at = content
            .lines()
            .find(|line| line.contains("**Created**:"))
            .and_then(|line| {
                // Extract timestamp after "**Created**: "
                line.split("**Created**:")
                    .nth(1)
                    .map(|s| s.trim())
            })
            .and_then(|timestamp_str| {
                // Parse ISO 8601 timestamp
                chrono::DateTime::parse_from_rfc3339(timestamp_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .unwrap_or_else(|| {
                // Fallback to current time if parsing fails
                tracing::warn!(
                    skill_name = %name,
                    "Failed to parse created_at timestamp from SKILL.md, using current time"
                );
                Utc::now()
            });

        // Parse parameters from table
        let mut parameters = Vec::new();
        let in_params = content.contains("## Parameters");
        if in_params {
            let params_section: Vec<_> = content
                .lines()
                .skip_while(|line| !line.starts_with("## Parameters"))
                .skip(1)
                .take_while(|line| !line.starts_with("## Implementation"))
                .collect();

            for line in params_section {
                if line.starts_with("| ") && !line.starts_with("| Name") && !line.starts_with("|------") {
                    let parts: Vec<_> = line.split('|').map(|s| s.trim()).collect();
                    if parts.len() >= 4 {
                        let param_name = parts[1].to_string();
                        let param_type = parts[2].to_string();
                        let required = parts[3] == "Yes";
                        let description = parts.get(4).unwrap_or(&"").to_string();

                        parameters.push(SkillParameter::new(
                            &param_name,
                            &description,
                            required,
                            &param_type,
                        ));
                    }
                }
            }
        }

        Ok(ManagedSkill {
            name: name.to_string(),
            description,
            parameters,
            implementation,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_manager() -> (TempDir, SkillsManager) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        tokio::fs::create_dir_all(&workspace_path).await.unwrap();

        let manager = SkillsManager::new(workspace_path);
        (temp_dir, manager)
    }

    #[tokio::test]
    async fn test_skills_manager_new() {
        let (_temp_dir, manager) = setup_test_manager().await;
        assert!(manager.skills_dir().to_string_lossy().contains("skills"));
    }

    #[tokio::test]
    async fn test_create_skill_success() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let skill = manager.create_skill(
            "weather_lookup".to_string(),
            "Get weather information".to_string(),
            vec![],
            "Use the web tool to fetch weather data".to_string(),
        ).await.unwrap();

        assert_eq!(skill.name, "weather_lookup");
        assert_eq!(skill.description, "Get weather information");

        // Verify file was created
        let skill_md_path = manager.skills_dir().join("weather_lookup").join("SKILL.md");
        assert!(skill_md_path.exists());

        // Verify cache
        assert!(manager.skill_exists("weather_lookup").await);
    }

    #[tokio::test]
    async fn test_create_skill_invalid_name_empty() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "".to_string(),
            "Description".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_create_skill_invalid_name_starts_with_number() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "123_skill".to_string(),
            "Description".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start with a lowercase letter"));
    }

    #[tokio::test]
    async fn test_create_skill_invalid_name_uppercase() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "WeatherSkill".to_string(),
            "Description".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start with a lowercase letter"));
    }

    #[tokio::test]
    async fn test_create_skill_invalid_name_special_chars() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "weather-skill".to_string(),
            "Description".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid character"));
    }

    #[tokio::test]
    async fn test_create_skill_name_conflict() {
        let (_temp_dir, manager) = setup_test_manager().await;

        // Create first skill
        manager.create_skill(
            "test_skill".to_string(),
            "First skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        // Try to create second with same name
        let result = manager.create_skill(
            "test_skill".to_string(),
            "Second skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_skill_builtin_conflict() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "filesystem".to_string(),
            "Description".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("conflicts with built-in tool"));
    }

    #[tokio::test]
    async fn test_list_skills_empty() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let skills = manager.list_skills().await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_list_skills_with_data() {
        let (_temp_dir, manager) = setup_test_manager().await;

        manager.create_skill(
            "skill_one".to_string(),
            "First skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        manager.create_skill(
            "skill_two".to_string(),
            "Second skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        let skills = manager.list_skills().await.unwrap();
        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_read_skill_success() {
        let (_temp_dir, manager) = setup_test_manager().await;

        manager.create_skill(
            "test_skill".to_string(),
            "Test description".to_string(),
            vec![],
            "Test implementation".to_string(),
        ).await.unwrap();

        let content = manager.read_skill("test_skill").await.unwrap();
        assert!(content.contains("# Skill: test_skill"));
        assert!(content.contains("Test description"));
        assert!(content.contains("Test implementation"));
    }

    #[tokio::test]
    async fn test_read_skill_not_found() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.read_skill("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_skill_success() {
        let (_temp_dir, manager) = setup_test_manager().await;

        manager.create_skill(
            "delete_me".to_string(),
            "To be deleted".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        assert!(manager.skill_exists("delete_me").await);

        manager.delete_skill("delete_me", &[]).await.unwrap();

        assert!(!manager.skill_exists("delete_me").await);

        let skill_dir = manager.skills_dir().join("delete_me");
        assert!(!skill_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_skill_builtin_protection() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.delete_skill("filesystem", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot delete built-in"));
    }

    #[tokio::test]
    async fn test_delete_skill_not_found() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.delete_skill("nonexistent", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_skills_from_disk() {
        let (temp_dir, manager) = setup_test_manager().await;

        // Create skill manually on disk
        let skill_dir = manager.skills_dir().join("manual_skill");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();

        let skill_md = r#"# Skill: manual_skill

## Description
Manually created skill

## Parameters
None

## Implementation
Manual implementation

## Metadata
- **Created**: 2026-02-16T10:00:00Z
- **Version**: 1.0.0
"#;

        tokio::fs::write(skill_dir.join("SKILL.md"), skill_md).await.unwrap();

        // Load skills
        manager.load_skills().await.unwrap();

        // Verify it was loaded
        assert!(manager.skill_exists("manual_skill").await);
    }

    #[tokio::test]
    async fn test_skill_with_parameters() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let params = vec![
            SkillParameter::new("city", "City name", true, "string"),
            SkillParameter::new("units", "Temperature units", false, "string"),
        ];

        manager.create_skill(
            "weather".to_string(),
            "Get weather".to_string(),
            params,
            "Implementation".to_string(),
        ).await.unwrap();

        let content = manager.read_skill("weather").await.unwrap();
        assert!(content.contains("city"));
        assert!(content.contains("units"));
        assert!(content.contains("City name"));
        assert!(content.contains("Temperature units"));
    }

    // FIX #1: Test path traversal prevention
    #[tokio::test]
    async fn test_create_skill_path_traversal_dotdot() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "../../../etc/passwd".to_string(),
            "Malicious skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separators"));
    }

    #[tokio::test]
    async fn test_create_skill_path_traversal_slash() {
        let (_temp_dir, manager) = setup_test_manager().await;

        let result = manager.create_skill(
            "my/evil/skill".to_string(),
            "Malicious skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separators"));
    }

    // FIX #6: Test filesystem uniqueness check
    #[tokio::test]
    async fn test_create_skill_filesystem_uniqueness_after_restart() {
        let (_temp_dir, manager) = setup_test_manager().await;

        // Create skill
        manager.create_skill(
            "persistent_skill".to_string(),
            "Test skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        // Simulate app restart by creating new manager (empty cache)
        let manager2 = SkillsManager::new(manager.skills_dir().parent().unwrap().to_path_buf());

        // Try to create same skill without loading cache
        let result = manager2.create_skill(
            "persistent_skill".to_string(),
            "Another skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    // FIX #8: Test read_skill error handling
    #[tokio::test]
    async fn test_read_skill_file_deleted() {
        let (_temp_dir, manager) = setup_test_manager().await;

        // Create skill
        manager.create_skill(
            "vanishing_skill".to_string(),
            "Test skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        // Delete SKILL.md file but keep in cache
        let skill_md_path = manager.skills_dir().join("vanishing_skill").join("SKILL.md");
        tokio::fs::remove_file(&skill_md_path).await.unwrap();

        // Try to read - should fail
        let result = manager.read_skill("vanishing_skill").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_read_skill_permission_denied() {
        use std::os::unix::fs::PermissionsExt;
        
        let (_temp_dir, manager) = setup_test_manager().await;

        // Create skill
        manager.create_skill(
            "forbidden_skill".to_string(),
            "Test skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        // Make SKILL.md unreadable (Unix only)
        let skill_md_path = manager.skills_dir().join("forbidden_skill").join("SKILL.md");
        let mut perms = tokio::fs::metadata(&skill_md_path).await.unwrap().permissions();
        perms.set_mode(0o000);
        tokio::fs::set_permissions(&skill_md_path, perms).await.unwrap();

        // Try to read - should fail
        let result = manager.read_skill("forbidden_skill").await;
        assert!(result.is_err());
        
        // Restore permissions for cleanup
        let mut perms = tokio::fs::metadata(&skill_md_path).await.unwrap().permissions();
        perms.set_mode(0o644);
        tokio::fs::set_permissions(&skill_md_path, perms).await.ok();
    }

    // FIX #3: Test created_at parsing
    #[tokio::test]
    async fn test_load_skills_preserves_created_at() {
        let (_temp_dir, manager) = setup_test_manager().await;

        // Create skill with known timestamp
        let skill = manager.create_skill(
            "timestamped_skill".to_string(),
            "Test skill".to_string(),
            vec![],
            "Implementation".to_string(),
        ).await.unwrap();

        let original_timestamp = skill.created_at.to_rfc3339();

        // Reload skills from disk
        manager.load_skills().await.unwrap();

        // Read the skill and verify timestamp was preserved
        let skills = manager.list_skills().await.unwrap();
        let loaded_skill = skills.iter().find(|s| s.name == "timestamped_skill").unwrap();
        
        // Timestamps should match (allowing for slight parsing differences)
        assert!(loaded_skill.created_at.starts_with(&original_timestamp[..19]));
    }
}
