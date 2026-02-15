//! Skills loader module for miniclaw
//!
//! This module handles skill discovery, loading, and parsing from the skills directory.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing;

use super::types::{Skill, SkillError, SkillParameter, SkillSummary};

/// Name of the skill definition file
const SKILL_FILENAME: &str = "SKILL.md";

/// Initialize the skills directory structure
///
/// Creates the skills directory inside the workspace if it doesn't exist.
///
/// # Arguments
/// * `workspace_path` - The workspace directory path (e.g., ~/.miniclaw/workspace)
/// * `verbose` - If true, logs progress information
///
/// # Returns
/// * `Ok(())` - Directory created successfully
/// * `Err` - If directory creation fails
///
/// # Example
/// ```no_run
/// use miniclaw::skills::initialize_skills_directory;
/// use std::path::PathBuf;
///
/// let workspace_path = PathBuf::from("/home/user/.miniclaw/workspace");
/// initialize_skills_directory(&workspace_path, true).unwrap();
/// ```
pub fn initialize_skills_directory(workspace_path: &Path, verbose: bool) -> Result<()> {
    let skills_path = workspace_path.join("skills");

    if !skills_path.exists() {
        fs::create_dir_all(&skills_path)
            .map_err(|e| SkillError::WriteError(skills_path.display().to_string(), e))?;

        // Set directory permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&skills_path, permissions)
                .map_err(|e| SkillError::WriteError(skills_path.display().to_string(), e))?;
        }

        if verbose {
            tracing::info!(path = %skills_path.display(), "Created skills directory");
        }
    } else if verbose {
        tracing::debug!(path = %skills_path.display(), "Skills directory already exists");
    }

    Ok(())
}

/// Get the path to the skills directory
///
/// # Arguments
/// * `workspace_path` - The workspace directory path
///
/// # Returns
/// * `PathBuf` - Path to skills directory
pub fn get_skills_directory(workspace_path: &Path) -> PathBuf {
    workspace_path.join("skills")
}

/// Check if the skills directory exists
///
/// # Arguments
/// * `workspace_path` - The workspace directory path
///
/// # Returns
/// * `true` - If the skills directory exists
/// * `false` - Otherwise
pub fn skills_directory_exists(workspace_path: &Path) -> bool {
    get_skills_directory(workspace_path).exists()
}

/// Discover all valid skill packages in the skills directory
///
/// Scans the skills directory and returns paths to all subdirectories
/// that contain a SKILL.md file. Hidden directories (starting with '.')
/// are filtered out.
///
/// # Arguments
/// * `skills_dir` - The skills directory path
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - List of valid skill directory paths
/// * `Err` - If the skills directory doesn't exist or can't be read
///
/// # Example
/// ```no_run
/// use miniclaw::skills::discover_skills;
/// use std::path::PathBuf;
///
/// let skills_dir = PathBuf::from("/home/user/.miniclaw/workspace/skills");
/// let skill_paths = discover_skills(&skills_dir).unwrap();
/// ```
pub fn discover_skills(skills_dir: &Path) -> Result<Vec<PathBuf>> {
    if !skills_dir.exists() {
        return Err(SkillError::DirectoryNotFound(skills_dir.display().to_string()).into());
    }

    let mut skill_paths = Vec::new();

    let entries = fs::read_dir(skills_dir)
        .map_err(|e| SkillError::ReadError(skills_dir.display().to_string(), e))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| SkillError::ReadError(skills_dir.display().to_string(), e))?;

        let path = entry.path();

        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories (dot prefix)
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str.starts_with('.') {
                    tracing::debug!(dir = %name_str, "Skipping hidden skill directory");
                    continue;
                }
            }
        }

        // Check if SKILL.md exists
        let skill_file = path.join(SKILL_FILENAME);
        if skill_file.exists() {
            skill_paths.push(path);
        } else {
            tracing::warn!(path = %path.display(), "Skill directory missing SKILL.md");
        }
    }

    Ok(skill_paths)
}

/// Parse a single skill from its directory
///
/// Reads and parses the SKILL.md file in the given skill directory.
///
/// # Arguments
/// * `skill_path` - Path to the skill directory
///
/// # Returns
/// * `Ok(Skill)` - Parsed skill
/// * `Err` - If the skill file doesn't exist or has invalid format
///
/// # Example
/// ```no_run
/// use miniclaw::skills::load_skill;
/// use std::path::PathBuf;
///
/// let skill_path = PathBuf::from("/home/user/.miniclaw/workspace/skills/weather");
/// let skill = load_skill(&skill_path).unwrap();
/// ```
pub fn load_skill(skill_path: &Path) -> Result<Skill> {
    let skill_file = skill_path.join(SKILL_FILENAME);

    if !skill_file.exists() {
        return Err(SkillError::FileNotFound(skill_file.display().to_string()).into());
    }

    let content = fs::read_to_string(&skill_file)
        .map_err(|e| SkillError::ReadError(skill_file.display().to_string(), e))?;

    parse_skill(&content, skill_path)
}

/// Parse skill content from SKILL.md text
///
/// Parses the markdown content to extract skill name, description, and parameters.
///
/// # Arguments
/// * `content` - The SKILL.md content
/// * `skill_path` - Path to the skill directory (for error messages)
///
/// # Returns
/// * `Ok(Skill)` - Parsed skill
/// * `Err` - If the content has invalid format
fn parse_skill(content: &str, skill_path: &Path) -> Result<Skill> {
    let directory_name = skill_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Parse skill name from header (e.g., "# Skill: Weather")
    let name = content
        .lines()
        .find(|line| line.starts_with("# Skill:"))
        .map(|line| line.trim_start_matches("# Skill:").trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            // Fallback: use first h1 without "Skill:" prefix
            content
                .lines()
                .find(|line| line.starts_with("# "))
                .map(|line| line.trim_start_matches("# ").trim().to_string())
        })
        .ok_or_else(|| SkillError::MissingField("name".to_string(), directory_name.clone()))?;

    // Parse description from "## Description" section
    let mut description = String::new();
    let mut in_description = false;

    for line in content.lines() {
        if line.trim() == "## Description" {
            in_description = true;
            continue;
        }
        if in_description {
            if line.starts_with("## ") {
                break;
            }
            if !line.trim().is_empty() || !description.is_empty() {
                if !description.is_empty() {
                    description.push(' ');
                }
                description.push_str(line.trim());
            }
        }
    }

    description = description.trim().to_string();

    if description.is_empty() {
        return Err(
            SkillError::MissingField("description".to_string(), directory_name.clone()).into(),
        );
    }

    // Parse parameters from "## Parameters" section
    let parameters = parse_parameters(content, &directory_name)?;

    Ok(Skill {
        name,
        description,
        parameters,
        content: content.to_string(),
        directory_name,
    })
}

/// Parse parameters from the SKILL.md content
///
/// Expected format:
/// ```markdown
/// ## Parameters
/// - `param1` (string, required): Description
/// - `param2` (number, optional): Description
/// ```
fn parse_parameters(content: &str, directory_name: &str) -> Result<Vec<SkillParameter>> {
    let mut parameters = Vec::new();
    let mut in_parameters = false;

    for line in content.lines() {
        if line.trim() == "## Parameters" {
            in_parameters = true;
            continue;
        }
        if in_parameters {
            if line.starts_with("## ") {
                break;
            }
            // Parse parameter line: - `name` (type, required|optional): description
            if line.trim().starts_with("- `") {
                if let Some(param) = parse_parameter_line(line) {
                    parameters.push(param);
                }
            }
        }
    }

    Ok(parameters)
}

/// Parse a single parameter line
///
/// Format: - `name` (type, required|optional): description
fn parse_parameter_line(line: &str) -> Option<SkillParameter> {
    // Remove leading "- `" and get the rest
    let rest = line.trim().strip_prefix("- `")?;

    // Split on closing backtick
    let parts: Vec<&str> = rest.splitn(2, '`').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].to_string();

    // Parse (type, required|optional): description
    let remaining = parts[1].trim();
    if !remaining.starts_with('(') {
        return None;
    }

    // Find closing parenthesis
    let close_idx = remaining.find(')')?;
    let type_info = &remaining[1..close_idx];
    let description = remaining[close_idx + 1..]
        .trim()
        .strip_prefix(':')
        .unwrap_or("")
        .trim()
        .to_string();

    // Parse type info: "type, required" or "type, optional"
    let type_parts: Vec<&str> = type_info.split(',').map(|s| s.trim()).collect();
    let param_type = type_parts.first().unwrap_or(&"string").to_string();
    let required = type_parts
        .get(1)
        .map(|s| s.trim() == "required")
        .unwrap_or(false);

    Some(SkillParameter {
        name,
        description,
        required,
        param_type,
    })
}

/// Load all valid skills from the skills directory
///
/// Discovers all skills and loads them, skipping invalid ones with warnings.
///
/// # Arguments
/// * `skills_dir` - The skills directory path
///
/// # Returns
/// * `Ok(Vec<Skill>)` - List of successfully loaded skills
/// * `Err` - If the skills directory doesn't exist
///
/// # Example
/// ```no_run
/// use miniclaw::skills::load_all_skills;
/// use std::path::PathBuf;
///
/// let skills_dir = PathBuf::from("/home/user/.miniclaw/workspace/skills");
/// let skills = load_all_skills(&skills_dir).unwrap();
/// ```
pub fn load_all_skills(skills_dir: &Path) -> Result<Vec<Skill>> {
    let skill_paths = discover_skills(skills_dir)?;
    let mut skills = Vec::new();

    for path in skill_paths {
        match load_skill(&path) {
            Ok(skill) => {
                if skill.is_valid() {
                    skills.push(skill);
                } else {
                    tracing::warn!(
                        path = %path.display(),
                        "Skill has invalid required fields, skipping"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "Failed to load skill, skipping");
            }
        }
    }

    Ok(skills)
}

/// List all available skills with their status
///
/// Returns a summary of all skills including disabled ones (hidden directories).
///
/// # Arguments
/// * `skills_dir` - The skills directory path
///
/// # Returns
/// * `Ok(Vec<SkillSummary>)` - List of skill summaries
/// * `Err` - If the skills directory doesn't exist
///
/// # Example
/// ```no_run
/// use miniclaw::skills::list_available_skills;
/// use std::path::PathBuf;
///
/// let skills_dir = PathBuf::from("/home/user/.miniclaw/workspace/skills");
/// let summaries = list_available_skills(&skills_dir).unwrap();
/// ```
pub fn list_available_skills(skills_dir: &Path) -> Result<Vec<SkillSummary>> {
    if !skills_dir.exists() {
        return Err(SkillError::DirectoryNotFound(skills_dir.display().to_string()).into());
    }

    let mut summaries = Vec::new();

    let entries = fs::read_dir(skills_dir)
        .map_err(|e| SkillError::ReadError(skills_dir.display().to_string(), e))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| SkillError::ReadError(skills_dir.display().to_string(), e))?;

        let path = entry.path();

        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Check if hidden (disabled)
        let is_active = !dir_name.starts_with('.');

        // Check if SKILL.md exists
        let skill_file = path.join(SKILL_FILENAME);
        if skill_file.exists() {
            // Try to parse for name and description
            match load_skill(&path) {
                Ok(skill) => {
                    summaries.push(SkillSummary::new(
                        &skill.name,
                        &skill.description,
                        is_active,
                        &dir_name,
                    ));
                }
                Err(_) => {
                    // Even if parsing fails, list it with directory name
                    summaries.push(SkillSummary::new(
                        &dir_name,
                        "(Failed to parse skill file)",
                        is_active,
                        &dir_name,
                    ));
                }
            }
        }
    }

    // Sort by name for consistent ordering
    summaries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(summaries)
}

/// Check if a skill exists (active or disabled)
///
/// # Arguments
/// * `skills_dir` - The skills directory path
/// * `skill_name` - The skill name or directory name
///
/// # Returns
/// * `true` - If the skill exists
/// * `false` - Otherwise
pub fn skill_exists(skills_dir: &Path, skill_name: &str) -> bool {
    if !skills_dir.exists() {
        return false;
    }

    // Check active skill
    let active_path = skills_dir.join(skill_name);
    if active_path.join(SKILL_FILENAME).exists() {
        return true;
    }

    // Check disabled skill (dot prefix)
    let disabled_path = skills_dir.join(format!(".{}", skill_name));
    if disabled_path.join(SKILL_FILENAME).exists() {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill_md(name: &str, desc: &str) -> String {
        format!(
            r#"# Skill: {name}

## Description
{desc}

## Parameters
- `param1` (string, required): First parameter
- `param2` (number, optional): Second parameter

## Usage
Example usage here.
"#,
            name = name,
            desc = desc
        )
    }

    fn setup_test_skills_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).unwrap();

        // Create valid skill
        let weather_dir = skills_dir.join("weather");
        fs::create_dir(&weather_dir).unwrap();
        fs::write(
            weather_dir.join("SKILL.md"),
            create_test_skill_md("Weather", "Get weather information"),
        )
        .unwrap();

        // Create another valid skill
        let reminder_dir = skills_dir.join("reminder");
        fs::create_dir(&reminder_dir).unwrap();
        fs::write(
            reminder_dir.join("SKILL.md"),
            create_test_skill_md("Reminder", "Set reminders"),
        )
        .unwrap();

        // Create disabled skill (hidden directory)
        let disabled_dir = skills_dir.join(".disabled");
        fs::create_dir(&disabled_dir).unwrap();
        fs::write(
            disabled_dir.join("SKILL.md"),
            create_test_skill_md("Disabled", "Disabled skill"),
        )
        .unwrap();

        // Create directory without SKILL.md
        let empty_dir = skills_dir.join("empty");
        fs::create_dir(&empty_dir).unwrap();

        (temp_dir, skills_dir)
    }

    #[test]
    fn test_initialize_skills_directory() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        fs::create_dir(&workspace_path).unwrap();

        let skills_path = workspace_path.join("skills");
        assert!(!skills_path.exists());

        initialize_skills_directory(&workspace_path, false).unwrap();

        assert!(skills_path.exists());
        assert!(skills_path.is_dir());
    }

    #[test]
    fn test_initialize_skills_directory_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path();

        // Create skills dir manually first
        let skills_path = workspace_path.join("skills");
        fs::create_dir(&skills_path).unwrap();

        // Should not fail if already exists
        initialize_skills_directory(workspace_path, false).unwrap();
        assert!(skills_path.exists());
    }

    #[test]
    fn test_get_skills_directory() {
        let workspace = PathBuf::from("/home/user/.miniclaw/workspace");
        let skills_dir = get_skills_directory(&workspace);
        assert_eq!(
            skills_dir,
            PathBuf::from("/home/user/.miniclaw/workspace/skills")
        );
    }

    #[test]
    fn test_skills_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path();

        assert!(!skills_directory_exists(workspace_path));

        fs::create_dir(workspace_path.join("skills")).unwrap();
        assert!(skills_directory_exists(workspace_path));
    }

    #[test]
    fn test_discover_skills() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        let paths = discover_skills(&skills_dir).unwrap();

        // Should find weather and reminder, but not .disabled or empty
        assert_eq!(paths.len(), 2);

        let dir_names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();

        assert!(dir_names.contains(&"weather".to_string()));
        assert!(dir_names.contains(&"reminder".to_string()));
        assert!(!dir_names.contains(&".disabled".to_string()));
        assert!(!dir_names.contains(&"empty".to_string()));
    }

    #[test]
    fn test_discover_skills_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let missing_dir = temp_dir.path().join("nonexistent");

        let result = discover_skills(&missing_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_skill() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();
        let weather_path = skills_dir.join("weather");

        let skill = load_skill(&weather_path).unwrap();

        assert_eq!(skill.name, "Weather");
        assert_eq!(skill.description, "Get weather information");
        assert_eq!(skill.directory_name, "weather");
        assert_eq!(skill.parameters.len(), 2);
        assert!(skill.is_valid());
    }

    #[test]
    fn test_load_skill_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("fake_skill");
        fs::create_dir(&skill_path).unwrap();

        let result = load_skill(&skill_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_simple_header() {
        let content = r#"# Simple Skill

## Description
A simple test skill.
"#;

        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("simple");

        let skill = parse_skill(content, &skill_path).unwrap();

        assert_eq!(skill.name, "Simple Skill");
        assert_eq!(skill.description, "A simple test skill.");
        assert!(skill.parameters.is_empty());
    }

    #[test]
    fn test_parse_skill_with_skill_prefix() {
        let content = r#"# Skill: My Skill

## Description
Description here.
"#;

        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("my_skill");

        let skill = parse_skill(content, &skill_path).unwrap();
        assert_eq!(skill.name, "My Skill");
    }

    #[test]
    fn test_parse_skill_missing_name() {
        let content = r#"## Description
No header here.
"#;

        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("bad");

        let result = parse_skill(content, &skill_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_missing_description() {
        let content = r#"# Skill: Name Only

## Parameters
- `p` (string, required): Param
"#;

        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("no_desc");

        let result = parse_skill(content, &skill_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_parameter_line() {
        let line = "- `city` (string, required): The city name";
        let param = parse_parameter_line(line).unwrap();

        assert_eq!(param.name, "city");
        assert_eq!(param.param_type, "string");
        assert!(param.required);
        assert_eq!(param.description, "The city name");
    }

    #[test]
    fn test_parse_parameter_line_optional() {
        let line = "- `units` (string, optional): Temperature units";
        let param = parse_parameter_line(line).unwrap();

        assert_eq!(param.name, "units");
        assert!(!param.required);
    }

    #[test]
    fn test_parse_parameter_line_invalid() {
        assert!(parse_parameter_line("Not a parameter").is_none());
        assert!(parse_parameter_line("- `name` no parens").is_none());
    }

    #[test]
    fn test_load_all_skills() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        let skills = load_all_skills(&skills_dir).unwrap();

        // Should load weather and reminder (2 valid skills)
        // Should skip .disabled (hidden) and empty (no SKILL.md)
        assert_eq!(skills.len(), 2);

        let names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"Weather".to_string()));
        assert!(names.contains(&"Reminder".to_string()));
    }

    #[test]
    fn test_load_all_skills_with_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).unwrap();

        // Create valid skill
        let valid_dir = skills_dir.join("valid");
        fs::create_dir(&valid_dir).unwrap();
        fs::write(
            valid_dir.join("SKILL.md"),
            create_test_skill_md("Valid", "A valid skill"),
        )
        .unwrap();

        // Create invalid skill (missing description)
        let invalid_dir = skills_dir.join("invalid");
        fs::create_dir(&invalid_dir).unwrap();
        fs::write(
            invalid_dir.join("SKILL.md"),
            "# Skill: Invalid\n\n## Parameters\n- `p` (string, required): Param\n",
        )
        .unwrap();

        let skills = load_all_skills(&skills_dir).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "Valid");
    }

    #[test]
    fn test_list_available_skills() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        let summaries = list_available_skills(&skills_dir).unwrap();

        // Should list all 3 skills (including .disabled)
        assert_eq!(summaries.len(), 3);

        let active_count = summaries.iter().filter(|s| s.is_active).count();
        let disabled_count = summaries.iter().filter(|s| !s.is_active).count();

        assert_eq!(active_count, 2); // weather, reminder
        assert_eq!(disabled_count, 1); // .disabled
    }

    #[test]
    fn test_list_available_skills_empty() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).unwrap();

        let summaries = list_available_skills(&skills_dir).unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_skill_exists() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        assert!(skill_exists(&skills_dir, "weather"));
        assert!(skill_exists(&skills_dir, "reminder"));
        assert!(!skill_exists(&skills_dir, "nonexistent"));

        // skill_exists finds both active and disabled skills
        // Disabled skill stored as ".disabled" can be found by "disabled"
        assert!(skill_exists(&skills_dir, "disabled"));
        assert!(skill_exists(&skills_dir, ".disabled"));
    }

    #[test]
    fn test_skill_exists_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let missing_dir = temp_dir.path().join("nonexistent");

        assert!(!skill_exists(&missing_dir, "anything"));
    }
}
