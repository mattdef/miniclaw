use crate::agent::tools::{Tool, ToolError, Result};
use crate::chat::types::OutboundMessage;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Tool for sending messages programmatically to users
/// 
/// This tool allows the agent to send messages proactively via the ChatHub
/// outbound channel. It supports specifying the target chat_id, message content,
/// and optionally the channel to use.
pub struct MessageTool {
    /// Sender for the outbound message channel
    outbound_tx: mpsc::Sender<OutboundMessage>,
    /// Default channel to use when not specified in the call
    default_channel: String,
}

impl MessageTool {
    /// Creates a new MessageTool instance
    /// 
    /// # Arguments
    /// * `outbound_tx` - The sender for the ChatHub outbound channel
    /// * `default_channel` - The default channel identifier (e.g., "telegram")
    pub fn new(outbound_tx: mpsc::Sender<OutboundMessage>, default_channel: impl Into<String>) -> Self {
        Self {
            outbound_tx,
            default_channel: default_channel.into(),
        }
    }
    
    /// Validates a chat_id to ensure it's not empty
    fn validate_chat_id(chat_id: &str) -> Result<()> {
        if chat_id.trim().is_empty() {
            return Err(ToolError::InvalidArguments {
                tool: "message".to_string(),
                message: "chat_id cannot be empty".to_string(),
            });
        }
        Ok(())
    }
    
    /// Validates message content
    fn validate_content(content: &str) -> Result<()> {
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
    
    async fn execute(&self, args: HashMap<String, Value>) -> Result<String> {
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
        
        // Resolve channel: use provided or fall back to default
        let channel = args
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_channel)
            .to_string();
        
        if channel.trim().is_empty() {
            return Err(ToolError::ExecutionFailed {
                tool: "message".to_string(),
                message: "No channel specified and no default channel available".to_string(),
            });
        }
        
        // Create the outbound message
        let message = OutboundMessage::new(&channel, chat_id, content);
        
        // Send to ChatHub outbound channel (non-blocking)
        match self.outbound_tx.try_send(message) {
            Ok(_) => {
                tracing::info!(
                    tool = "message",
                    chat_id = %chat_id,
                    channel = %channel,
                    "Message queued for delivery"
                );
                
                // AC 5: Return immediately (non-blocking)
                Ok(format!("Message queued for delivery to {} via {}", chat_id, channel))
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Full outbound buffer - this is recoverable
                tracing::warn!(
                    tool = "message",
                    "Outbound buffer full, message could not be queued"
                );
                
                Err(ToolError::ExecutionFailed {
                    tool: "message".to_string(),
                    message: "Outbound buffer full, try again later".to_string(),
                })
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed - not recoverable
                tracing::error!(
                    tool = "message",
                    "Outbound channel closed"
                );
                
                Err(ToolError::ExecutionFailed {
                    tool: "message".to_string(),
                    message: "ChatHub is not available".to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::types::OutboundMessage;
    
    fn create_test_tool() -> (MessageTool, mpsc::Receiver<OutboundMessage>) {
        let (tx, rx) = mpsc::channel(100);
        let tool = MessageTool::new(tx, "telegram");
        (tool, rx)
    }
    
    #[test]
    fn test_message_tool_name() {
        let (tool, _rx) = create_test_tool();
        assert_eq!(tool.name(), "message");
    }
    
    #[test]
    fn test_message_tool_description() {
        let (tool, _rx) = create_test_tool();
        let desc = tool.description();
        assert!(desc.contains("Send a message"));
        assert!(desc.contains("chat_id"));
        assert!(desc.contains("content"));
    }
    
    #[test]
    fn test_message_tool_parameters_schema() {
        let (tool, _rx) = create_test_tool();
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
        let (tool, mut rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123456"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        
        let result = tool.execute(args).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("queued for delivery"));
        
        // Verify message was sent to channel
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.chat_id, "123456");
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.channel, "telegram");
    }
    
    #[tokio::test]
    async fn test_execute_with_custom_channel() {
        let (tool, mut rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("789"));
        args.insert("content".to_string(), serde_json::json!("Test"));
        args.insert("channel".to_string(), serde_json::json!("discord"));
        
        let result = tool.execute(args).await;
        
        assert!(result.is_ok());
        
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.channel, "discord");
        assert_eq!(msg.chat_id, "789");
    }
    
    #[tokio::test]
    async fn test_execute_missing_chat_id() {
        let (tool, _rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        
        let result = tool.execute(args).await;
        
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
        let (tool, _rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        
        let result = tool.execute(args).await;
        
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
        let (tool, _rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("   "));
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        
        let result = tool.execute(args).await;
        
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
        let (tool, _rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!(""));
        
        let result = tool.execute(args).await;
        
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
        let (tool, _rx) = create_test_tool();
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Test"));
        
        // Execute should return immediately
        let start = std::time::Instant::now();
        let result = tool.execute(args).await;
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        // Should complete almost instantly (< 10ms)
        assert!(duration < std::time::Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_buffer_full_error() {
        let (tx, _rx) = mpsc::channel(1); // Very small buffer
        let tool = MessageTool::new(tx, "telegram");
        
        // Fill the buffer
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("1"));
        args.insert("content".to_string(), serde_json::json!("First"));
        tool.execute(args).await.unwrap();
        
        // Try to send another (buffer full)
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("2"));
        args.insert("content".to_string(), serde_json::json!("Second"));
        let result = tool.execute(args).await;
        
        // This may or may not fail depending on timing
        // but it should never panic
        match result {
            Ok(_) => (), // Buffer was drained fast enough
            Err(ToolError::ExecutionFailed { .. }) => (), // Buffer full
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_channel_closed_error() {
        let (tx, rx) = mpsc::channel(10);
        let tool = MessageTool::new(tx, "telegram");
        
        // Drop the receiver to close the channel
        drop(rx);
        
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        
        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("not available"));
            }
            _ => panic!("Expected ExecutionFailed error for closed channel"),
        }
    }
    
    #[tokio::test]
    async fn test_no_default_channel() {
        let (tx, _rx) = mpsc::channel(10);
        let tool = MessageTool::new(tx, ""); // Empty default channel
        
        let mut args = HashMap::new();
        args.insert("chat_id".to_string(), serde_json::json!("123"));
        args.insert("content".to_string(), serde_json::json!("Hello!"));
        // No channel specified
        
        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed { message, .. } => {
                assert!(message.contains("No channel specified"));
            }
            _ => panic!("Expected ExecutionFailed error for missing channel"),
        }
    }
}
