#[cfg(test)]
mod tests {
    use miniclaw::chat::{InboundMessage, OutboundMessage};
    use serde_json::json;

    #[test]
    fn test_inbound_message_creation() {
        let msg = InboundMessage::new("telegram", "123456789", "Hello miniclaw!");

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "123456789");
        assert_eq!(msg.content, "Hello miniclaw!");
        assert!(msg.metadata.is_empty());
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
    }

    #[test]
    fn test_outbound_message_creation() {
        let msg = OutboundMessage {
            channel: "telegram".to_string(),
            chat_id: "123456789".to_string(),
            content: "Hello user!".to_string(),
            reply_to: None,
        };

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "123456789");
        assert_eq!(msg.content, "Hello user!");
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn test_outbound_message_with_reply() {
        let msg = OutboundMessage {
            channel: "telegram".to_string(),
            chat_id: "123456789".to_string(),
            content: "Reply message".to_string(),
            reply_to: Some("msg_123".to_string()),
        };

        assert_eq!(msg.reply_to, Some("msg_123".to_string()));
    }

    #[test]
    fn test_outbound_message_serialization() {
        let msg = OutboundMessage {
            channel: "telegram".to_string(),
            chat_id: "123".to_string(),
            content: "Test".to_string(),
            reply_to: Some("reply_id".to_string()),
        };

        let json_str = serde_json::to_string(&msg).unwrap();
        assert!(json_str.contains("telegram"));
        assert!(json_str.contains("reply_id"));
    }
}
