//! Skill management tools
//!
//! This module provides tools for creating, listing, reading, and deleting skills:
//! - create_skill: Create a new skill package
//! - list_skills: List all available skills
//! - read_skill: Read the full content of a skill
//! - delete_skill: Delete a user-created skill

use crate::agent::tools::types::{Tool, ToolError, ToolExecutionContext, ToolResult};
use crate::skills::{SkillManagerError, SkillParameter, SkillsManager};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;

/// Tool for creating new skills
pub struct CreateSkillTool {
    workspace_path: PathBuf,
}

impl CreateSkillTool {
    /// Create a new CreateSkillTool
    pub fn new(workspace_path: PathBuf) -> Self {
        let canonical_workspace = std::fs::canonicalize(&workspace_path).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize workspace path {:?}: {}",
                workspace_path, e
            )
        });

        Self {
            workspace_path: canonical_workspace,
        }
    }
}

#[async_trait]
impl Tool for CreateSkillTool {
    fn name(&self) -> &str {
        "create_skill"
    }

    fn description(&self) -> &str {
        "Create a new reusable skill package with name, description, parameters, and implementation"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name (snake_case, unique, max 50 chars)"
                },
                "description": {
                    "type": "string",
                    "description": "What this skill does"
                },
                "parameters": {
                    "type": "array",
                    "description": "Parameter definitions for the skill",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "type": {"type": "string", "enum": ["string", "number", "boolean", "array", "object"]},
                            "description": {"type": "string"},
                            "required": {"type": "boolean"}
                        },
                        "required": ["name", "type", "description", "required"]
                    }
                },
                "implementation": {
                    "type": "string",
                    "description": "Implementation instructions or code for the skill"
                }
            },
            "required": ["name", "description", "implementation"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Get required parameters
        let name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'name'".to_string(),
            }
        })?;

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'description'".to_string(),
            })?;

        let implementation = args
            .get("implementation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'implementation'".to_string(),
            })?;

        // Parse optional parameters
        let parameters =
            if let Some(params_array) = args.get("parameters").and_then(|v| v.as_array()) {
                params_array
                    .iter()
                    .map(|param| {
                        let param_name = param
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let param_type = param
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let param_desc = param
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let param_required = param
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                        SkillParameter::new(&param_name, &param_desc, param_required, &param_type)
                    })
                    .collect()
            } else {
                Vec::new()
            };

        // Create skills manager with stored workspace path
        let skills_manager = SkillsManager::new(self.workspace_path.clone());

        // Load existing skills to check for conflicts
        skills_manager
            .load_skills()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to load existing skills: {}", e),
            })?;

        // Create the skill
        match skills_manager
            .create_skill(
                name.to_string(),
                description.to_string(),
                parameters,
                implementation.to_string(),
            )
            .await
        {
            Ok(skill) => {
                let file_path = self
                    .workspace_path
                    .join("skills")
                    .join(&skill.name)
                    .join("SKILL.md");
                let response = json!({
                    "success": true,
                    "message": "Skill created successfully",
                    "skill": {
                        "name": skill.name,
                        "description": skill.description,
                        "created_at": skill.created_at.to_rfc3339()
                    },
                    "file_path": file_path.to_string_lossy().to_string()
                });
                Ok(response.to_string())
            }
            Err(SkillManagerError::InvalidName { name: _, reason }) => {
                Err(ToolError::InvalidArguments {
                    tool: self.name().to_string(),
                    message: format!("Invalid skill name: {}", reason),
                })
            }
            Err(SkillManagerError::NameConflict { name: _ }) => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!("Skill '{}' already exists", name),
            }),
            Err(SkillManagerError::BuiltInToolConflict { name: _ }) => {
                Err(ToolError::InvalidArguments {
                    tool: self.name().to_string(),
                    message: format!("Name '{}' conflicts with a built-in tool", name),
                })
            }
            Err(e) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to create skill: {}", e),
            }),
        }
    }
}

/// Tool for listing all skills
pub struct ListSkillsTool {
    workspace_path: PathBuf,
}

impl ListSkillsTool {
    /// Create a new ListSkillsTool
    pub fn new(workspace_path: PathBuf) -> Self {
        let canonical_workspace = std::fs::canonicalize(&workspace_path).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize workspace path {:?}: {}",
                workspace_path, e
            )
        });

        Self {
            workspace_path: canonical_workspace,
        }
    }
}

#[async_trait]
impl Tool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }

    fn description(&self) -> &str {
        "List all available skills with their names and descriptions"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        let skills_manager = SkillsManager::new(self.workspace_path.clone());

        // Load skills
        skills_manager
            .load_skills()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to load skills: {}", e),
            })?;

        // Get skill list
        match skills_manager.list_skills().await {
            Ok(skills) => {
                let response = json!({
                    "success": true,
                    "skills": skills.iter().map(|s| {
                        json!({
                            "name": s.name,
                            "description": s.description,
                            "created_at": s.created_at
                        })
                    }).collect::<Vec<_>>()
                });
                Ok(response.to_string())
            }
            Err(e) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to list skills: {}", e),
            }),
        }
    }
}

/// Tool for reading a skill's content
pub struct ReadSkillTool {
    workspace_path: PathBuf,
}

impl ReadSkillTool {
    /// Create a new ReadSkillTool
    pub fn new(workspace_path: PathBuf) -> Self {
        let canonical_workspace = std::fs::canonicalize(&workspace_path).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize workspace path {:?}: {}",
                workspace_path, e
            )
        });

        Self {
            workspace_path: canonical_workspace,
        }
    }
}

#[async_trait]
impl Tool for ReadSkillTool {
    fn name(&self) -> &str {
        "read_skill"
    }

    fn description(&self) -> &str {
        "Read the full content of a skill from its SKILL.md file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the skill to read"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        let name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'name'".to_string(),
            }
        })?;

        let skills_manager = SkillsManager::new(self.workspace_path.clone());

        // Load skills first
        skills_manager
            .load_skills()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to load skills: {}", e),
            })?;

        // Read the skill
        match skills_manager.read_skill(name).await {
            Ok(content) => {
                let response = json!({
                    "success": true,
                    "name": name,
                    "content": content
                });
                Ok(response.to_string())
            }
            Err(SkillManagerError::SkillNotFound { name: _ }) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Skill '{}' not found", name),
            }),
            Err(e) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to read skill: {}", e),
            }),
        }
    }
}

/// Tool for deleting a skill
pub struct DeleteSkillTool {
    workspace_path: PathBuf,
}

impl DeleteSkillTool {
    /// Create a new DeleteSkillTool
    pub fn new(workspace_path: PathBuf) -> Self {
        let canonical_workspace = std::fs::canonicalize(&workspace_path).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize workspace path {:?}: {}",
                workspace_path, e
            )
        });

        Self {
            workspace_path: canonical_workspace,
        }
    }
}

#[async_trait]
impl Tool for DeleteSkillTool {
    fn name(&self) -> &str {
        "delete_skill"
    }

    fn description(&self) -> &str {
        "Delete a user-created skill (cannot delete built-in skills)"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the skill to delete"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        let name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'name'".to_string(),
            }
        })?;

        let skills_manager = SkillsManager::new(self.workspace_path.clone());

        // Load skills first
        skills_manager
            .load_skills()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to load skills: {}", e),
            })?;

        // Delete the skill
        match skills_manager.delete_skill(name).await {
            Ok(()) => {
                let response = json!({
                    "success": true,
                    "message": format!("Skill '{}' deleted successfully", name)
                });
                Ok(response.to_string())
            }
            Err(SkillManagerError::BuiltInSkillDeletion { name: _ }) => {
                Err(ToolError::PermissionDenied {
                    tool: self.name().to_string(),
                    message: format!("Cannot delete built-in skill '{}'", name),
                })
            }
            Err(SkillManagerError::SkillNotFound { name: _ }) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Skill '{}' not found", name),
            }),
            Err(e) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to delete skill: {}", e),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::types::ToolExecutionContext;

    fn create_test_context() -> ToolExecutionContext {
        ToolExecutionContext {
            chat_id: Some("test".to_string()),
            channel: Some("test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_skill_tool_success() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = CreateSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("test_skill"));
        args.insert("description".to_string(), json!("Test description"));
        args.insert("implementation".to_string(), json!("Test implementation"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());

        let response: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response["success"].as_bool().unwrap());
        assert_eq!(response["skill"]["name"], "test_skill");
    }

    #[tokio::test]
    async fn test_create_skill_tool_missing_name() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = CreateSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("description".to_string(), json!("Test description"));
        args.insert("implementation".to_string(), json!("Test implementation"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing required parameter")
        );
    }

    #[tokio::test]
    async fn test_create_skill_tool_invalid_name() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = CreateSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("Invalid-Name"));
        args.insert("description".to_string(), json!("Test description"));
        args.insert("implementation".to_string(), json!("Test implementation"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_skills_tool_empty() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = ListSkillsTool::new(workspace_path);
        let ctx = create_test_context();

        let args = HashMap::new();
        let result = tool.execute(args, &ctx).await.unwrap();

        let response: Value = serde_json::from_str(&result).unwrap();
        assert!(response["success"].as_bool().unwrap());
        assert!(response["skills"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_skills_tool_with_skills() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        // Create a skill first
        let create_tool = CreateSkillTool::new(workspace_path.clone());
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("test_skill"));
        args.insert("description".to_string(), json!("Test description"));
        args.insert("implementation".to_string(), json!("Test implementation"));
        create_tool.execute(args, &ctx).await.unwrap();

        // Now list skills
        let list_tool = ListSkillsTool::new(workspace_path);
        let result = list_tool.execute(HashMap::new(), &ctx).await.unwrap();

        let response: Value = serde_json::from_str(&result).unwrap();
        assert!(response["success"].as_bool().unwrap());
        assert_eq!(response["skills"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_read_skill_tool_success() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        // Create a skill first
        let create_tool = CreateSkillTool::new(workspace_path.clone());
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("test_skill"));
        args.insert("description".to_string(), json!("Test description"));
        args.insert("implementation".to_string(), json!("Test implementation"));
        create_tool.execute(args, &ctx).await.unwrap();

        // Now read the skill
        let read_tool = ReadSkillTool::new(workspace_path);
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("test_skill"));

        let result = read_tool.execute(args, &ctx).await.unwrap();
        let response: Value = serde_json::from_str(&result).unwrap();

        assert!(response["success"].as_bool().unwrap());
        assert!(
            response["content"]
                .as_str()
                .unwrap()
                .contains("Test description")
        );
    }

    #[tokio::test]
    async fn test_read_skill_tool_not_found() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = ReadSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("nonexistent"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_skill_tool_success() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        // Create a skill first
        let create_tool = CreateSkillTool::new(workspace_path.clone());
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("delete_me"));
        args.insert("description".to_string(), json!("To be deleted"));
        args.insert("implementation".to_string(), json!("Implementation"));
        create_tool.execute(args, &ctx).await.unwrap();

        // Now delete the skill
        let delete_tool = DeleteSkillTool::new(workspace_path);
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("delete_me"));

        let result = delete_tool.execute(args, &ctx).await.unwrap();
        let response: Value = serde_json::from_str(&result).unwrap();

        assert!(response["success"].as_bool().unwrap());
        assert!(
            response["message"]
                .as_str()
                .unwrap()
                .contains("deleted successfully")
        );
    }

    #[tokio::test]
    async fn test_delete_skill_tool_builtin() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = DeleteSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("filesystem"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot delete built-in")
        );
    }

    #[tokio::test]
    async fn test_delete_skill_tool_not_found() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = DeleteSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("nonexistent"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_create_skill_with_parameters() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let tool = CreateSkillTool::new(workspace_path);
        let ctx = create_test_context();

        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("weather_lookup"));
        args.insert("description".to_string(), json!("Get weather information"));
        args.insert("implementation".to_string(), json!("Implementation"));

        let params = json!([
            {
                "name": "city",
                "type": "string",
                "description": "City name",
                "required": true
            },
            {
                "name": "units",
                "type": "string",
                "description": "Temperature units",
                "required": false
            }
        ]);
        args.insert("parameters".to_string(), params);

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());

        let response: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response["success"].as_bool().unwrap());
    }
}
