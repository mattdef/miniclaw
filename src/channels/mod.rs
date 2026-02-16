use crate::chat::{ChatHub, OutboundMessage};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub mod telegram;

pub use telegram::{TelegramChannel, TelegramError};

/// Trait that all communication channels must implement.
/// Channels are responsible for:
/// - Receiving inbound messages from external services (e.g., Telegram)
/// - Sending outbound messages back to users
#[async_trait]
pub trait Channel: Send + Sync {
    /// Start the channel and begin listening for inbound messages.
    /// 
    /// This method should:
    /// - Initialize the connection to the external service
    /// - Register outbound message sender with the ChatHub
    /// - Spawn background tasks to handle incoming messages
    /// - Return immediately after setup (don't block)
    ///
    /// # Arguments
    /// * `hub` - The ChatHub instance for routing messages
    async fn start(&self, hub: Arc<ChatHub>) -> Result<()>;

    /// Send an outbound message to the user.
    ///
    /// This method is called by the ChatHub when the agent generates
    /// a response that needs to be delivered to the user.
    ///
    /// # Arguments
    /// * `message` - The outbound message containing recipient and content
    async fn send(&self, message: OutboundMessage) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // Mock channel for testing the trait
    struct MockChannel {
        sender: mpsc::Sender<OutboundMessage>,
    }

    #[async_trait]
    impl Channel for MockChannel {
        async fn start(&self, _hub: Arc<ChatHub>) -> Result<()> {
            Ok(())
        }

        async fn send(&self, message: OutboundMessage) -> Result<()> {
            self.sender.send(message).await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_channel_trait() {
        let (tx, mut rx) = mpsc::channel(10);
        let channel = MockChannel { sender: tx };

        // Test send
        let msg = OutboundMessage::new("test", "123", "Hello");
        channel.send(msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.content, "Hello");
    }
}
