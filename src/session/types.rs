use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_MESSAGES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub channel: String,
    pub chat_id: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub messages: VecDeque<Message>,
}

impl Session {
    pub fn new(channel: String, chat_id: String) -> Self {
        let session_id = format!("{}_{}", channel, chat_id);
        let now = Utc::now();
        Self {
            session_id,
            channel,
            chat_id,
            created_at: now,
            last_accessed: now,
            messages: VecDeque::with_capacity(MAX_MESSAGES),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
        self.last_accessed = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    pub fn new(role: String, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: Utc::now(),
            tool_calls: None,
        }
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Creates a tool result message
    /// Note: Uses "tool_result" role (not "tool") to distinguish session storage
    /// from LLM message roles. ContextBuilder translates this to LlmRole::Tool.
    pub fn tool_result(content: String) -> Self {
        Self {
            role: "tool_result".to_string(),
            content,
            timestamp: Utc::now(),
            tool_calls: None,
        }
    }

    /// Checks if this message is from a user
    pub fn is_user(&self) -> bool {
        self.role == "user"
    }

    /// Checks if this message is from the assistant
    pub fn is_assistant(&self) -> bool {
        self.role == "assistant"
    }

    /// Checks if this message is a tool result
    pub fn is_tool_result(&self) -> bool {
        self.role == "tool_result"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("telegram".to_string(), "123456789".to_string());
        assert_eq!(session.session_id, "telegram_123456789");
        assert_eq!(session.channel, "telegram");
        assert_eq!(session.chat_id, "123456789");
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_add_message() {
        let mut session = Session::new("telegram".to_string(), "123456789".to_string());
        let message = Message::new("user".to_string(), "Hello".to_string());
        session.add_message(message);

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Hello");
    }

    #[test]
    fn test_fifo_rotation() {
        let mut session = Session::new("telegram".to_string(), "123456789".to_string());

        // Add 51 messages
        for i in 0..51 {
            let message = Message::new("user".to_string(), format!("Message {}", i));
            session.add_message(message);
        }

        // Should have 50 messages, first one removed
        assert_eq!(session.messages.len(), 50);
        assert_eq!(session.messages[0].content, "Message 1");
        assert_eq!(session.messages[49].content, "Message 50");
    }

    #[test]
    fn test_message_creation() {
        let message = Message::new("assistant".to_string(), "Hello!".to_string());
        assert_eq!(message.role, "assistant");
        assert_eq!(message.content, "Hello!");
        assert!(message.tool_calls.is_none());
    }

    #[test]
    fn test_message_with_tool_calls() {
        let tool_calls = vec![ToolCall {
            id: "call_1".to_string(),
            name: "filesystem".to_string(),
            arguments: "{\"path\": \"/tmp\"}".to_string(),
        }];
        let message = Message::new("assistant".to_string(), "Processing".to_string())
            .with_tool_calls(tool_calls);

        assert!(message.tool_calls.is_some());
        assert_eq!(message.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_serialization() {
        let session = Session::new("telegram".to_string(), "123456789".to_string());
        let json = serde_json::to_string(&session).unwrap();

        // Verify snake_case field names
        assert!(json.contains("session_id"));
        assert!(json.contains("chat_id"));
        assert!(json.contains("created_at"));
        assert!(json.contains("last_accessed"));
        assert!(json.contains("messages"));

        // Verify ISO 8601 format
        assert!(json.contains("T") && json.contains("Z"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "session_id": "telegram_123456789",
            "channel": "telegram",
            "chat_id": "123456789",
            "created_at": "2026-02-15T10:30:00Z",
            "last_accessed": "2026-02-15T15:45:00Z",
            "messages": []
        }"#;

        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.session_id, "telegram_123456789");
        assert_eq!(session.channel, "telegram");
        assert_eq!(session.chat_id, "123456789");
    }
}
