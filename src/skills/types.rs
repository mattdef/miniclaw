//! Skills module types for miniclaw
//!
//! This module defines the data structures for skill packages.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for skill operations
#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill directory does not exist: {0:?}")]
    DirectoryNotFound(String),

    #[error("Skill file not found: {0:?}")]
    FileNotFound(String),

    #[error("Invalid skill format in {0}: {1}")]
    InvalidFormat(String, String),

    #[error("Missing required field '{0}' in skill {1}")]
    MissingField(String, String),

    #[error("Failed to read skill file {0}: {1}")]
    ReadError(String, #[source] std::io::Error),

    #[error("Failed to write skill file {0}: {1}")]
    WriteError(String, #[source] std::io::Error),

    #[error("Skill error: {0}")]
    Other(String),
}

/// Represents a skill parameter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Parameter type (string, number, boolean, etc.)
    pub param_type: String,
}

impl SkillParameter {
    /// Create a new skill parameter
    pub fn new(name: &str, description: &str, required: bool, param_type: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required,
            param_type: param_type.to_string(),
        }
    }
}

/// Represents a skill package
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    /// Skill name (from SKILL.md header)
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill parameters
    pub parameters: Vec<SkillParameter>,
    /// Full content of SKILL.md
    pub content: String,
    /// Directory name (may differ from skill name)
    pub directory_name: String,
}

impl Skill {
    /// Create a new skill
    pub fn new(
        name: &str,
        description: &str,
        parameters: Vec<SkillParameter>,
        content: &str,
        directory_name: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            content: content.to_string(),
            directory_name: directory_name.to_string(),
        }
    }

    /// Check if the skill has valid required fields
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && !self.description.is_empty()
    }

    /// Get formatted skill info for context
    pub fn to_context_string(&self) -> String {
        let mut context = format!("### {}\n{}\n\n", self.name, self.description);

        if !self.parameters.is_empty() {
            context.push_str("**Parameters:**\n");
            for param in &self.parameters {
                context.push_str(&format!(
                    "- `{}` ({}, {}): {}\n",
                    param.name,
                    param.param_type,
                    if param.required {
                        "required"
                    } else {
                        "optional"
                    },
                    param.description
                ));
            }
            context.push('\n');
        }

        context
    }
}

/// Summary information about a skill for listing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSummary {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Whether the skill is active (not disabled)
    pub is_active: bool,
    /// Directory name
    pub directory_name: String,
}

impl SkillSummary {
    /// Create a new skill summary
    pub fn new(name: &str, description: &str, is_active: bool, directory_name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            is_active,
            directory_name: directory_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_parameter_new() {
        let param = SkillParameter::new("city", "The city name", true, "string");
        assert_eq!(param.name, "city");
        assert_eq!(param.description, "The city name");
        assert!(param.required);
        assert_eq!(param.param_type, "string");
    }

    #[test]
    fn test_skill_new() {
        let params = vec![SkillParameter::new("param1", "desc", true, "string")];
        let skill = Skill::new(
            "Test Skill",
            "A test skill",
            params,
            "content",
            "test_skill",
        );

        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.description, "A test skill");
        assert_eq!(skill.parameters.len(), 1);
        assert_eq!(skill.content, "content");
        assert_eq!(skill.directory_name, "test_skill");
    }

    #[test]
    fn test_skill_is_valid() {
        let valid_skill = Skill::new("Name", "Description", vec![], "", "dir");
        assert!(valid_skill.is_valid());

        let invalid_skill = Skill::new("", "Description", vec![], "", "dir");
        assert!(!invalid_skill.is_valid());

        let invalid_skill2 = Skill::new("Name", "", vec![], "", "dir");
        assert!(!invalid_skill2.is_valid());
    }

    #[test]
    fn test_skill_to_context_string() {
        let params = vec![
            SkillParameter::new("city", "City name", true, "string"),
            SkillParameter::new("units", "Temperature units", false, "string"),
        ];
        let skill = Skill::new("Weather", "Get weather info", params, "", "weather");

        let context = skill.to_context_string();
        assert!(context.contains("### Weather"));
        assert!(context.contains("Get weather info"));
        assert!(context.contains("city"));
        assert!(context.contains("units"));
        assert!(context.contains("required"));
        assert!(context.contains("optional"));
    }

    #[test]
    fn test_skill_context_string_no_params() {
        let skill = Skill::new("Simple", "Simple skill", vec![], "", "simple");
        let context = skill.to_context_string();
        assert!(context.contains("### Simple"));
        assert!(context.contains("Simple skill"));
        assert!(!context.contains("Parameters:"));
    }

    #[test]
    fn test_skill_summary_new() {
        let summary = SkillSummary::new("Test", "Test desc", true, "test");
        assert_eq!(summary.name, "Test");
        assert_eq!(summary.description, "Test desc");
        assert!(summary.is_active);
        assert_eq!(summary.directory_name, "test");
    }

    #[test]
    fn test_skill_error_display() {
        let err = SkillError::InvalidFormat("test".to_string(), "bad format".to_string());
        assert!(err.to_string().contains("Invalid skill format"));
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("bad format"));
    }
}
