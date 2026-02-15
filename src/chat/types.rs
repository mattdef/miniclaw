use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const MAX_CONTENT_LENGTH: usize = 4000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

impl InboundMessage {
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Sanitizes and validates the message content.
    /// Returns true if the message is valid (not empty/whitespace), false otherwise.
    pub fn sanitize(&mut self) -> bool {
        let trimmed = self.content.trim();
        if trimmed.is_empty() {
            return false;
        }

        if trimmed.len() > MAX_CONTENT_LENGTH {
            tracing::warn!(
                channel = %self.channel,
                chat_id = %self.chat_id,
                original_length = self.content.len(),
                "Message too long, truncating to {}",
                MAX_CONTENT_LENGTH
            );
            // Truncate safely on UTF-8 character boundaries.
            self.content = trimmed.chars().take(MAX_CONTENT_LENGTH).collect();
        } else if trimmed.len() != self.content.len() {
            self.content = trimmed.to_string();
        }

        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

impl OutboundMessage {
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: None,
        }
    }

    pub fn reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_inbound_message_creation() {
        let msg = InboundMessage::new("telegram", "123456789", "Hello miniclaw!");
        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "123456789");
        assert_eq!(msg.content, "Hello miniclaw!");
        assert!(msg.metadata.is_empty());
        // Verify timestamp is recent
        let now = Utc::now();
        assert!(now.signed_duration_since(msg.timestamp).num_seconds() < 5);
    }

    #[test]
    fn test_inbound_message_sanitize_whitespace() {
        let mut msg = InboundMessage::new("telegram", "123", "   ");
        assert!(!msg.sanitize());
    }

    #[test]
    fn test_inbound_message_sanitize_trim() {
        let mut msg = InboundMessage::new("telegram", "123", "  hello  ");
        assert!(msg.sanitize());
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_inbound_message_sanitize_truncate() {
        let long_content = "a".repeat(MAX_CONTENT_LENGTH + 10);
        let mut msg = InboundMessage::new("telegram", "123", long_content);
        assert!(msg.sanitize());
        assert_eq!(msg.content.len(), MAX_CONTENT_LENGTH);
    }

    #[test]
    fn test_inbound_message_with_metadata() {
        let msg = InboundMessage::new("telegram", "987654321", "Test message")
            .with_metadata("user_name", json!("John"))
            .with_metadata("is_bot", json!(false));

        assert_eq!(msg.metadata.get("user_name").unwrap(), "John");
        assert_eq!(msg.metadata.get("is_bot").unwrap(), false);
    }

    #[test]
    fn test_inbound_message_serialization() {
        let msg =
            InboundMessage::new("telegram", "123", "Test").with_metadata("key", json!("value"));

        let json_str = serde_json::to_string(&msg).unwrap();
        assert!(json_str.contains("telegram"));
        assert!(json_str.contains("123"));
        assert!(json_str.contains("Test"));
        assert!(json_str.contains("key"));
    }

    #[test]
    fn test_timestamp_is_iso8601_utc() {
        let msg = InboundMessage::new("telegram", "123", "Test message");
        let json_str = serde_json::to_string(&msg).unwrap();

        // ISO 8601 format should contain 'T' separator and 'Z' suffix for UTC
        // Example: 2026-02-15T14:30:00Z
        assert!(
            json_str.contains("T"),
            "ISO 8601 timestamp should contain 'T' separator"
        );
        assert!(
            json_str.contains("Z"),
            "UTC timestamp should contain 'Z' suffix"
        );
    }

    #[test]
    fn test_outbound_message_creation() {
        let msg = OutboundMessage::new("telegram", "123456789", "Hello user!");
        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "123456789");
        assert_eq!(msg.content, "Hello user!");
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn test_outbound_message_with_reply() {
        let msg =
            OutboundMessage::new("telegram", "123456789", "Reply message").reply_to("msg_123");

        assert_eq!(msg.reply_to, Some("msg_123".to_string()));
    }

    #[test]
    fn test_outbound_message_serialization() {
        let msg = OutboundMessage::new("telegram", "123", "Test").reply_to("reply_id");

        let json_str = serde_json::to_string(&msg).unwrap();
        assert!(json_str.contains("telegram"));
        assert!(json_str.contains("reply_id"));
    }
}
