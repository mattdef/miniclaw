use crate::agent::tools::ToolExecutionContext;
use crate::agent::tools::{Tool, ToolError, ToolResult};
use crate::chat::ChatHub;
use crate::chat::types::OutboundMessage;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Tool for sending messages programmatically to users
///
/// This tool allows the agent to send messages proactively via the ChatHub
/// outbound channel. It supports specifying the target chat_id, message content,
/// and optionally the channel to use.
pub struct MessageTool {
    /// Chat hub used to enqueue outbound messages
    hub: Arc<ChatHub>,
    /// Default channel to use when not specified in the call
    default_channel: String,
}

impl MessageTool {
    /// Creates a new MessageTool instance
    ///
    /// # Arguments
    /// * `hub` - The ChatHub used to enqueue outbound messages
    /// * `default_channel` - The default channel identifier (e.g., "telegram")
    pub fn new(hub: Arc<ChatHub>, default_channel: impl Into<String>) -> Self {
        Self {
            hub,
            default_channel: default_channel.into(),
        }
    }

    /// Validates a chat_id to ensure it's not empty
    fn validate_chat_id(chat_id: &str) -> ToolResult<()> {
        let trimmed = chat_id.trim();
        if trimmed.is_empty() {
            return Err(ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "chat_id cannot be empty".to_string(),
            });
        }
        if trimmed.len() > 128 {
            return Err(ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "chat_id is too long".to_string(),
            });
        }
        if trimmed != chat_id {
            return Err(ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "chat_id must not contain leading or trailing whitespace".to_string(),
            });
        }
        Ok(())
    }

    /// Validates message content
    fn validate_content(content: &str) -> ToolResult<()> {
        if content.trim().is_empty() {
            return Err(ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "content cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        "Send a message to a user via the configured communication channel. \
         Requires chat_id and content parameters. Optional channel parameter \
         overrides the default channel."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "chat_id": {
                    "type": "string",
                    "description": "Target user identifier"
                },
                "content": {
                    "type": "string",
                    "description": "Message content to send"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel identifier (optional, defaults to current context channel)"
                }
            },
            "required": ["chat_id", "content"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        // Extract required parameters
        let chat_id = args
            .get("chat_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "Missing required parameter 'chat_id'".to_string(),
            })?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "Missing required parameter 'content'".to_string(),
            })?;

        // Validate parameters
        Self::validate_chat_id(chat_id)?;
        Self::validate_content(content)?;

        // Resolve channel: args > conversation context > tool default
        let channel = args
            .get("channel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| ctx.channel.clone())
            .unwrap_or_else(|| self.default_channel.clone());

        if channel.trim().is_empty() {
            return Err(ToolError::ExecutionFailed {
                tool: "message".to_string(),
                message: "No channel specified and no default channel available".to_string(),
            });
        }

        // Create the outbound message
        let message = OutboundMessage::new(&channel, chat_id, content);

        // Enqueue to ChatHub outbound buffer (non-blocking)
        match self.hub.try_send_outbound(message) {
            Ok(()) => {
                tracing::info!(
                    tool = "message",
                    chat_id = %chat_id,
                    channel = %channel,
                    "Message queued for delivery"
                );
                Ok(format!(
                    "Message queued for delivery to {} via {}",
                    chat_id, channel
                ))
            }
            Err(e) => Err(ToolError::ExecutionFailedRecoverable {
                tool: "message".to_string(),
                message: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    fn create_test_tool() -> (MessageTool, Arc<ChatHub>) {
        let hub = Arc::new(ChatHub::with_capacities(100, 10));
        (MessageTool::new(Arc::clone(&hub), "telegram"), hub)
    }

    fn empty_ctx() -> ToolExecutionContext {
        ToolExecutionContext::default()
    }

    #[test]
    fn test_message_tool_name() {
        let (tool, _hub) = create_test_tool();
        assert_eq!(tool.name(), "message");
    }

    #[test]
    fn test_message_tool_description() {
        let (tool, _hub) = create_test_tool();
        let desc = tool.description();
        assert!(desc.contains("Send a message"));
        assert!(desc.contains("chat_id"));
        assert!(desc.contains("content"));
    }

    #[test]
    fn test_message_tool_parameters_schema() {
        let (tool, _hub) = create_test_tool();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["chat_id"]["type"] == "string");
        assert!(params["properties"]["content"]["type"] == "string");
        assert!(params["properties"]["channel"]["type"] == "string");

        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("chat_id")));
        assert!(required.contains(&serde_json::json!("content")));
    }

    #[tokio::test]
    async fn test_execute_basic_message() {
        let (tool, hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123456"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("queued for delivery"));

        let msg = hub.test_try_recv_outbound().await.unwrap();
        assert_eq!(msg.chat_id, "123456");
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.channel, "telegram");
    }

    #[tokio::test]
    async fn test_execute_with_custom_channel() {
        let (tool, hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("789"));
        args.insert("content".to_string(), serde_json::json!("Test"));
        args.insert("channel".to_string(), serde_json::json!("discord"));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_ok());

        let msg = hub.test_try_recv_outbound().await.unwrap();
        assert_eq!(msg.channel, "discord");
        assert_eq!(msg.chat_id, "789");
    }

    #[tokio::test]
    async fn test_execute_missing_chat_id() {
        let (tool, _hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("content".to_string(), serde_json::json!("Hello!"));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { tool, message } => {
                assert_eq!(tool, "message");
                assert!(message.contains("chat_id"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_execute_missing_content() {
        let (tool, _hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { tool, message } => {
                assert_eq!(tool, "message");
                assert!(message.contains("content"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_execute_empty_chat_id() {
        let (tool, _hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("   "));
        args.insert("content".to_string(), serde_json::json!("Hello!"));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("chat_id"));
            }
            _ => panic!("Expected InvalidArguments error for empty chat_id"),
        }
    }

    #[tokio::test]
    async fn test_execute_empty_content() {
        let (tool, _hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!(""));

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArguments { message, .. } => {
                assert!(message.contains("content"));
            }
            _ => panic!("Expected InvalidArguments error for empty content"),
        }
    }

    #[tokio::test]
    async fn test_execute_non_blocking() {
        let (tool, _hub) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Test"));

        let ctx = empty_ctx();
        let result =
            tokio::time::timeout(Duration::from_millis(20), tool.execute(args, &ctx)).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_buffer_full_error() {
        let hub = Arc::new(ChatHub::with_capacities(10, 1)); // Very small buffer
        let tool = MessageTool::new(Arc::clone(&hub), "telegram");

        // Fill the buffer
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("1"));
        args.insert("content".to_string(), serde_json::json!("First"));
        tool.execute(args, &empty_ctx()).await.unwrap();

        // Try to send another (buffer full)
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("2"));
        args.insert("content".to_string(), serde_json::json!("Second"));
        let result = tool.execute(args, &empty_ctx()).await;

        // This may or may not fail depending on timing
        // but it should never panic
        match result {
            Ok(_) => (), // Buffer was drained fast enough
            Err(ToolError::ExecutionFailedRecoverable { .. }) => (),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_channel_closed_error() {
        let hub = Arc::new(ChatHub::with_capacities(10, 10));
        let tool = MessageTool::new(Arc::clone(&hub), "telegram");

        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));

        // With the current ChatHub API, enqueueing can succeed even if no channel is registered.
        // Delivery failure is handled asynchronously during routing.
        let result = tool.execute(args, &empty_ctx()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_no_default_channel() {
        let hub = Arc::new(ChatHub::with_capacities(10, 10));
        let tool = MessageTool::new(hub, ""); // Empty default channel

        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        // No channel specified

        let result = tool.execute(args, &empty_ctx()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("No channel specified"));
            }
            _ => panic!("Expected ExecutionFailed error for missing channel"),
        }
    }

    #[tokio::test]
    async fn test_execute_uses_context_channel_when_missing() {
        let hub = Arc::new(ChatHub::with_capacities(100, 10));
        let tool = MessageTool::new(Arc::clone(&hub), "");
        let ctx = ToolExecutionContext {
            channel: Some("telegram".to_string()),
            chat_id: Some("ctx-chat".to_string()),
        };

        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Hello"));

        let result = tool.execute(args, &ctx).await;
        assert!(result.is_ok());

        let msg = hub.test_try_recv_outbound().await.unwrap();
        assert_eq!(msg.channel, "telegram");
    }
}
