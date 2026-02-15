//! Skills module for miniclaw
//!
//! This module handles skill discovery, loading, and management for the agent.
//! Skills are user-defined capabilities stored as markdown files in the skills directory.

pub mod loader;
pub mod types;

// Re-export commonly used types and functions
pub use loader::{
    discover_skills, get_skills_directory, initialize_skills_directory, list_available_skills,
    load_all_skills, load_skill, skill_exists, skills_directory_exists,
};
pub use types::{Skill, SkillError, SkillParameter, SkillSummary};

use anyhow::Result;
use std::path::Path;

/// Load skills context for the agent
///
/// Loads all valid skills and formats them as context string for inclusion
/// in the agent's system prompt.
///
/// # Arguments
/// * `skills_dir` - The skills directory path
///
/// # Returns
/// * `Ok(String)` - Formatted skills context
/// * `Err` - If the skills directory doesn't exist
///
/// # Example
/// ```no_run
/// use miniclaw::skills::load_skills_context;
/// use std::path::PathBuf;
///
/// let skills_dir = PathBuf::from("/home/user/.miniclaw/workspace/skills");
/// let context = load_skills_context(&skills_dir).unwrap();
/// ```
pub fn load_skills_context(skills_dir: &Path) -> Result<String> {
    let skills = load_all_skills(skills_dir)?;

    if skills.is_empty() {
        return Ok(String::from(
            "## Available Skills\n\nNo skills configured.\n",
        ));
    }

    let mut context = String::from("## Available Skills\n\n");

    for skill in skills {
        context.push_str(&skill.to_context_string());
    }

    Ok(context)
}

/// Get the count of available (active) skills
///
/// # Arguments
/// * `skills_dir` - The skills directory path
///
/// # Returns
/// * `Ok(usize)` - Number of valid skills
/// * `Err` - If the skills directory doesn't exist
pub fn get_skill_count(skills_dir: &Path) -> Result<usize> {
    let skills = load_all_skills(skills_dir)?;
    Ok(skills.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill_md(name: &str, desc: &str) -> String {
        format!(
            r#"# Skill: {name}

## Description
{desc}

## Parameters
- `param1` (string, required): First parameter

## Usage
Example usage.
"#,
            name = name,
            desc = desc
        )
    }

    fn setup_test_skills_dir() -> (TempDir, std::path::PathBuf) {
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

        (temp_dir, skills_dir)
    }

    #[test]
    fn test_load_skills_context() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        let context = load_skills_context(&skills_dir).unwrap();

        assert!(context.contains("## Available Skills"));
        assert!(context.contains("### Weather"));
        assert!(context.contains("### Reminder"));
        assert!(context.contains("Get weather information"));
        assert!(context.contains("Set reminders"));
    }

    #[test]
    fn test_load_skills_context_empty() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).unwrap();

        let context = load_skills_context(&skills_dir).unwrap();

        assert!(context.contains("## Available Skills"));
        assert!(context.contains("No skills configured"));
    }

    #[test]
    fn test_load_skills_context_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let missing_dir = temp_dir.path().join("nonexistent");

        let result = load_skills_context(&missing_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_skill_count() {
        let (_temp_dir, skills_dir) = setup_test_skills_dir();

        let count = get_skill_count(&skills_dir).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_skill_count_empty() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        fs::create_dir(&skills_dir).unwrap();

        let count = get_skill_count(&skills_dir).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_module_reexports() {
        // Test that types are re-exported correctly
        let _param = SkillParameter::new("test", "test desc", true, "string");
        let _skill = Skill::new("Test", "Desc", vec![], "", "test");
        let _summary = SkillSummary::new("Test", "Desc", true, "test");

        // Test that functions are re-exported
        // (We can't easily test them without a real directory, but compilation proves they exist)
    }
}
