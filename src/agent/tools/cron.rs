//! Cron tool for the agent
//!
//! This tool provides task scheduling capabilities allowing the agent to
//! schedule commands for later execution, either as one-time tasks (FireAt)
//! or recurring tasks (Interval).
//!
//! # Security
//! Commands are validated against the same blacklist as the exec tool
//! to prevent execution of dangerous system commands.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent::tools::types::{Tool, ToolError, ToolExecutionContext, ToolResult};
use crate::cron::CronScheduler;

/// Tool for scheduling tasks
///
/// Provides scheduling capabilities with support for:
/// - One-time tasks (FireAt) that execute at a specific time
/// - Recurring tasks (Interval) that execute every N minutes (minimum 2)
/// - Job listing to view scheduled tasks
/// - Job cancellation to remove scheduled tasks
#[derive(Debug, Clone)]
pub struct CronTool {
    scheduler: CronScheduler,
}

/// Response format for schedule operations
#[derive(Serialize, Deserialize)]
struct ScheduleResponse {
    success: bool,
    job_id: String,
    message: String,
    next_execution: String,
}

/// Response format for list operations
#[derive(Serialize, Deserialize)]
struct ListResponse {
    success: bool,
    jobs: Vec<JobResponse>,
}

/// Individual job response format
#[derive(Serialize, Deserialize)]
struct JobResponse {
    id: String,
    job_type: String,
    command: String,
    next_execution: String,
    execution_count: u32,
    status: String,
}

/// Response format for cancel operations
#[derive(Serialize, Deserialize)]
struct CancelResponse {
    success: bool,
    message: String,
}

impl CronTool {
    /// Creates a new CronTool with the provided scheduler
    ///
    /// # Arguments
    /// * `scheduler` - The CronScheduler instance to use for job management
    pub fn new(scheduler: CronScheduler) -> Self {
        Self { scheduler }
    }

    /// Schedules a FireAt job
    async fn schedule_fire_at(
        &self,
        command: String,
        time: String,
        args: Option<Vec<String>>,
    ) -> ToolResult<String> {
        match self.scheduler.schedule_fire_at(command, time, args).await {
            Ok(result) => {
                let response = ScheduleResponse {
                    success: true,
                    job_id: result.job_id,
                    message: result.message,
                    next_execution: result.next_execution.to_rfc3339(),
                };
                Ok(serde_json::to_string(&response).unwrap())
            }
            Err(e) => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: e,
            }),
        }
    }

    /// Schedules an Interval job
    async fn schedule_interval(
        &self,
        command: String,
        minutes: u32,
        args: Option<Vec<String>>,
    ) -> ToolResult<String> {
        match self.scheduler.schedule_interval(command, minutes, args).await {
            Ok(result) => {
                let response = ScheduleResponse {
                    success: true,
                    job_id: result.job_id,
                    message: result.message,
                    next_execution: result.next_execution.to_rfc3339(),
                };
                Ok(serde_json::to_string(&response).unwrap())
            }
            Err(e) => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: e,
            }),
        }
    }

    /// Lists all scheduled jobs
    async fn list_jobs(&self) -> ToolResult<String> {
        let result = self.scheduler.list_jobs().await;

        let jobs: Vec<JobResponse> = result
            .jobs
            .into_iter()
            .map(|info| JobResponse {
                id: info.id,
                job_type: info.job_type,
                command: info.command,
                next_execution: info.next_execution.to_rfc3339(),
                execution_count: info.execution_count,
                status: info.status,
            })
            .collect();

        let response = ListResponse {
            success: true,
            jobs,
        };

        Ok(serde_json::to_string(&response).unwrap())
    }

    /// Cancels a scheduled job
    async fn cancel_job(&self,
        job_id: String,
    ) -> ToolResult<String> {
        match self.scheduler.cancel_job(&job_id).await {
            Ok(result) => {
                let response = CancelResponse {
                    success: true,
                    message: result.message,
                };
                Ok(serde_json::to_string(&response).unwrap())
            }
            Err(e) => {
                let response = CancelResponse {
                    success: false,
                    message: e,
                };
                Ok(serde_json::to_string(&response).unwrap())
            }
        }
    }
}

#[async_trait::async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule tasks for later execution. Supports one-time tasks (FireAt) and recurring tasks (Interval). \
         Use 'schedule' action to create jobs, 'list' to view scheduled jobs, and 'cancel' to remove jobs. \
         FireAt requires an ISO 8601 datetime (e.g., '2026-02-16T10:00:00Z'). \
         Interval requires minutes >= 2. Commands are executed with the same security restrictions as exec tool."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["schedule", "list", "cancel"],
                    "description": "Action to perform: schedule a job, list all jobs, or cancel a job"
                },
                "job_type": {
                    "type": "string",
                    "enum": ["fire_at", "interval"],
                    "description": "Type of job to schedule (required for 'schedule' action)"
                },
                "command": {
                    "type": "string",
                    "description": "Command to execute (required for 'schedule' action)"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command arguments as an array (optional)"
                },
                "time": {
                    "type": "string",
                    "description": "ISO 8601 datetime for FireAt jobs (e.g., '2026-02-16T10:00:00Z')"
                },
                "minutes": {
                    "type": "integer",
                    "minimum": 2,
                    "description": "Interval in minutes for recurring jobs (minimum 2)"
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID to cancel (required for 'cancel' action)"
                }
            },
            "required": ["action"],
            "allOf": [
                {
                    "if": {
                        "properties": { "action": { "const": "schedule" } }
                    },
                    "then": {
                        "required": ["job_type", "command"],
                        "allOf": [
                            {
                                "if": {
                                    "properties": { "job_type": { "const": "fire_at" } }
                                },
                                "then": {
                                    "required": ["time"]
                                }
                            },
                            {
                                "if": {
                                    "properties": { "job_type": { "const": "interval" } }
                                },
                                "then": {
                                    "required": ["minutes"]
                                }
                            }
                        ]
                    }
                },
                {
                    "if": {
                        "properties": { "action": { "const": "cancel" } }
                    },
                    "then": {
                        "required": ["job_id"]
                    }
                }
            ]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Get action parameter
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing required parameter 'action'".to_string(),
            })?;

        match action {
            "schedule" => {
                let job_type = args
                    .get("job_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: "Missing required parameter 'job_type' for schedule action".to_string(),
                    })?;

                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: "Missing required parameter 'command' for schedule action".to_string(),
                    })?;

                let command = command.to_string();

                let cmd_args = args
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });

                match job_type {
                    "fire_at" => {
                        let time = args
                            .get("time")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| ToolError::InvalidArguments {
                                tool: self.name().to_string(),
                                message: "Missing required parameter 'time' for FireAt job".to_string(),
                            })?;

                        self.schedule_fire_at(command, time.to_string(), cmd_args)
                            .await
                    }
                    "interval" => {
                        let minutes = args
                            .get("minutes")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32)
                            .ok_or_else(|| ToolError::InvalidArguments {
                                tool: self.name().to_string(),
                                message: "Missing or invalid parameter 'minutes' for Interval job".to_string(),
                            })?;

                        self.schedule_interval(command, minutes, cmd_args).await
                    }
                    _ => Err(ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: format!("Invalid job_type: {}", job_type),
                    }),
                }
            }
            "list" => self.list_jobs().await,
            "cancel" => {
                let job_id = args
                    .get("job_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: "Missing required parameter 'job_id' for cancel action".to_string(),
                    })?;

                self.cancel_job(job_id.to_string()).await
            }
            _ => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!("Invalid action: {}", action),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cron_tool_name() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        assert_eq!(tool.name(), "cron");
    }

    #[test]
    fn test_cron_tool_description() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let desc = tool.description();
        assert!(desc.contains("Schedule tasks"));
        assert!(desc.contains("FireAt"));
        assert!(desc.contains("Interval"));
    }

    #[test]
    fn test_cron_tool_parameters() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let params = tool.parameters();

        // Check structure
        assert_eq!(params.get("type").unwrap(), "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());

        // Check action enum
        let properties = params.get("properties").unwrap();
        let action = properties.get("action").unwrap();
        let action_enum = action.get("enum").unwrap().as_array().unwrap();
        assert!(action_enum.contains(&json!("schedule")));
        assert!(action_enum.contains(&json!("list")));
        assert!(action_enum.contains(&json!("cancel")));
    }

    #[tokio::test]
    async fn test_cron_tool_execute_schedule_fire_at() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let execute_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("schedule"));
            map.insert("job_type".to_string(), json!("fire_at"));
            map.insert("command".to_string(), json!("echo"));
            map.insert("time".to_string(), json!(execute_at));
            map.insert("args".to_string(), json!(["hello"]));
            map
        };

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());
        
        let response: ScheduleResponse = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response.success);
        assert!(response.job_id.starts_with("job_"));
    }

    #[tokio::test]
    async fn test_cron_tool_execute_schedule_interval() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("schedule"));
            map.insert("job_type".to_string(), json!("interval"));
            map.insert("command".to_string(), json!("ls"));
            map.insert("minutes".to_string(), json!(5));
            map
        };

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());
        
        let response: ScheduleResponse = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response.success);
        assert!(response.message.contains("every 5 minutes"));
    }

    #[tokio::test]
    async fn test_cron_tool_execute_list() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("list"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());
        
        let response: ListResponse = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response.success);
        assert!(response.jobs.is_empty());
    }

    #[tokio::test]
    async fn test_cron_tool_execute_cancel() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        // First schedule a job
        let execute_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let schedule_args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("schedule"));
            map.insert("job_type".to_string(), json!("fire_at"));
            map.insert("command".to_string(), json!("echo"));
            map.insert("time".to_string(), json!(execute_at));
            map
        };

        let result = tool.execute(schedule_args, &ctx).await.unwrap();
        let schedule_response: ScheduleResponse = serde_json::from_str(&result).unwrap();
        let job_id = schedule_response.job_id;

        // Now cancel it
        let cancel_args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("cancel"));
            map.insert("job_id".to_string(), json!(job_id));
            map
        };

        let result = tool.execute(cancel_args, &ctx).await;
        assert!(result.is_ok());
        
        let response: CancelResponse = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_cron_tool_execute_missing_action() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = HashMap::new();
        let result = tool.execute(args, &ctx).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("Missing required parameter 'action'"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_cron_tool_execute_invalid_action() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("invalid"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("Invalid action"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_cron_tool_execute_schedule_missing_job_type() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("schedule"));
            map.insert("command".to_string(), json!("echo"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("Missing required parameter 'job_type'"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_cron_tool_execute_fire_at_missing_time() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("schedule"));
            map.insert("job_type".to_string(), json!("fire_at"));
            map.insert("command".to_string(), json!("echo"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("Missing required parameter 'time'"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_cron_tool_execute_cancel_missing_job_id() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("cancel"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("Missing required parameter 'job_id'"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_cron_tool_execute_cancel_nonexistent() {
        let scheduler = CronScheduler::new();
        let tool = CronTool::new(scheduler);
        let ctx = ToolExecutionContext::default();

        let args = {
            let mut map = HashMap::new();
            map.insert("action".to_string(), json!("cancel"));
            map.insert("job_id".to_string(), json!("nonexistent"));
            map
        };

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok()); // Returns success: false, not an error
        
        let response: CancelResponse = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(!response.success);
        assert!(response.message.contains("not found"));
    }
}
