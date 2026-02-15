use thiserror::Error;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::chat::types::{InboundMessage, OutboundMessage};

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Channel send failed: {0}")]
    SendError(String),
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ChatError>;

pub struct ChatHub {
    inbound_tx: mpsc::Sender<InboundMessage>,
    inbound_rx: Arc<RwLock<mpsc::Receiver<InboundMessage>>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Arc<RwLock<mpsc::Receiver<OutboundMessage>>>,
    channels: Arc<RwLock<HashMap<String, mpsc::Sender<OutboundMessage>>>>,
}

impl ChatHub {
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(100);
        let (outbound_tx, outbound_rx) = mpsc::channel(100);
        
        Self {
            inbound_tx,
            inbound_rx: Arc::new(RwLock::new(inbound_rx)),
            outbound_tx,
            outbound_rx: Arc::new(RwLock::new(outbound_rx)),
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn inbound_sender(&self) -> mpsc::Sender<InboundMessage> {
        self.inbound_tx.clone()
    }

    pub fn outbound_sender(&self) -> mpsc::Sender<OutboundMessage> {
        self.outbound_tx.clone()
    }

    pub async fn register_channel(&self,
        name: impl Into<String>,
        sender: mpsc::Sender<OutboundMessage>,
    ) -> Result<()> {
        let mut channels = self.channels.write().await;
        channels.insert(name.into(), sender);
        Ok(())
    }

    pub async fn send_inbound(&self, mut message: InboundMessage) -> Result<()> {
        if !message.sanitize() {
            tracing::debug!(
                channel = %message.channel,
                chat_id = %message.chat_id,
                "Ignoring empty or whitespace-only message"
            );
            return Ok(());
        }

        match self.inbound_tx.try_send(message) {
            Ok(_) => Ok(()),
            Err(mpsc::error::TrySendError::Full(msg)) => {
                tracing::warn!("Inbound buffer full, dropping oldest message");
                if let Ok(mut rx) = self.inbound_rx.try_write() {
                    let _ = rx.try_recv();
                }
                self.inbound_tx
                    .send(msg)
                    .await
                    .map_err(|e| ChatError::SendError(e.to_string()))
            }
            Err(e) => Err(ChatError::SendError(e.to_string())),
        }
    }

    pub async fn send_outbound(&self, message: OutboundMessage) -> Result<()> {
        match self.outbound_tx.try_send(message) {
            Ok(_) => Ok(()),
            Err(mpsc::error::TrySendError::Full(msg)) => {
                tracing::warn!("Outbound buffer full, dropping oldest message");
                if let Ok(mut rx) = self.outbound_rx.try_write() {
                    let _ = rx.try_recv();
                }
                self.outbound_tx
                    .send(msg)
                    .await
                    .map_err(|e| ChatError::SendError(e.to_string()))
            }
            Err(e) => Err(ChatError::SendError(e.to_string())),
        }
    }

    /// Convenience method to send a reply back to a specific channel and chat.
    pub async fn reply(
        &self,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<()> {
        let message = OutboundMessage::new(channel, chat_id, content);
        self.send_outbound(message).await
    }

    /// Convenience method to send a threaded reply.
    pub async fn reply_to(
        &self,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
        message_id: impl Into<String>,
    ) -> Result<()> {
        let message = OutboundMessage::new(channel, chat_id, content).reply_to(message_id);
        self.send_outbound(message).await
    }

    pub async fn route_outbound(&self, message: OutboundMessage) -> Result<()> {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(&message.channel) {
            sender
                .send(message)
                .await
                .map_err(|e| ChatError::SendError(e.to_string()))
        } else {
            Err(ChatError::ChannelNotFound(message.channel))
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut shutdown_signal = std::pin::pin!(tokio::signal::ctrl_c());

        loop {
            tokio::select! {
                Some(msg) = self.recv_inbound() => {
                    tracing::debug!(
                        channel = %msg.channel,
                        chat_id = %msg.chat_id,
                        "Received inbound message"
                    );
                }
                Some(msg) = self.recv_outbound() => {
                    tracing::debug!(
                        channel = %msg.channel,
                        chat_id = %msg.chat_id,
                        "Routing outbound message"
                    );
                    if let Err(e) = self.route_outbound(msg).await {
                        tracing::error!(error = %e, "Failed to route outbound message");
                    }
                }
                _ = &mut shutdown_signal => {
                    tracing::info!("Received shutdown signal, shutting down gracefully...");
                    self.shutdown().await?;
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Draining ChatHub channels...");

        // Drain inbound
        let mut inbound_rx = self.inbound_rx.write().await;
        while let Ok(msg) = inbound_rx.try_recv() {
            tracing::debug!(
                channel = %msg.channel,
                chat_id = %msg.chat_id,
                "Drained inbound message"
            );
        }

        // Drain outbound and route them
        let mut outbound_rx = self.outbound_rx.write().await;
        while let Ok(msg) = outbound_rx.try_recv() {
            tracing::debug!(
                channel = %msg.channel,
                chat_id = %msg.chat_id,
                "Drained outbound message"
            );
            let _ = self.route_outbound(msg).await;
        }

        tracing::info!("ChatHub shutdown complete");
        Ok(())
    }

    async fn recv_inbound(&self) -> Option<InboundMessage> {
        let mut rx = self.inbound_rx.write().await;
        rx.recv().await
    }

    async fn recv_outbound(&self) -> Option<OutboundMessage> {
        let mut rx = self.outbound_rx.write().await;
        rx.recv().await
    }
}

impl Default for ChatHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_hub_creation() {
        let hub = ChatHub::new();
        let _inbound = hub.inbound_sender();
        let _outbound = hub.outbound_sender();
    }

    #[tokio::test]
    async fn test_channel_registration() {
        let hub = ChatHub::new();
        let (tx, _rx) = mpsc::channel(10);
        
        hub.register_channel("test", tx).await.unwrap();
    }

    #[tokio::test]
    async fn test_send_inbound() {
        let hub = ChatHub::new();
        let msg = InboundMessage::new("telegram", "123", "Test message");
        
        hub.send_inbound(msg).await.unwrap();
    }

    #[tokio::test]
    async fn test_route_outbound() {
        let hub = ChatHub::new();
        let (tx, mut rx) = mpsc::channel(10);
        
        hub.register_channel("telegram", tx).await.unwrap();
        
        let msg = OutboundMessage::new("telegram", "123", "Test reply");
        hub.route_outbound(msg).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        assert_eq!(received.content, "Test reply");
    }

    #[tokio::test]
    async fn test_route_outbound_unregistered_channel() {
        let hub = ChatHub::new();
        let msg = OutboundMessage::new("unknown", "123", "Test reply");
        
        let result = hub.route_outbound(msg).await;
        assert!(matches!(result, Err(ChatError::ChannelNotFound(_))));
    }

    #[tokio::test]
    async fn test_buffer_overflow_inbound() {
        let hub = ChatHub::new();
        
        // Fill the buffer (capacity 100)
        for i in 0..100 {
            hub.send_inbound(InboundMessage::new("test", "123", format!("msg {}", i))).await.unwrap();
        }
        
        // Send one more - should trigger overflow logic
        hub.send_inbound(InboundMessage::new("test", "123", "overflow")).await.unwrap();
        
        // Check that we can still receive
        let mut rx = hub.inbound_rx.write().await;
        let msg1 = rx.recv().await.unwrap();
        // Since we dropped the oldest (msg 0), the first one should be msg 1
        assert_eq!(msg1.content, "msg 1");
    }

    #[tokio::test]
    async fn test_simple_send_receive() {
        let hub = ChatHub::new();
        
        // Simple send should work
        let msg = InboundMessage::new("telegram", "123", "Test message");
        hub.send_inbound(msg).await.unwrap();
    }

    #[tokio::test]
    async fn test_inbound_ignoring_whitespace() {
        let hub = ChatHub::new();
        
        // Send whitespace message
        hub.send_inbound(InboundMessage::new("test", "123", "   ")).await.unwrap();
        
        // Send valid message
        hub.send_inbound(InboundMessage::new("test", "123", "valid")).await.unwrap();
        
        let mut rx = hub.inbound_rx.write().await;
        let received = rx.try_recv().unwrap();
        assert_eq!(received.content, "valid");
        assert!(rx.try_recv().is_err()); // Should be empty
    }

    #[tokio::test]
    async fn test_reply_helpers() {
        let hub = ChatHub::new();
        hub.reply("telegram", "123", "Hello").await.unwrap();
        
        let mut rx = hub.outbound_rx.write().await;
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.channel, "telegram");
    }

    #[tokio::test]
    async fn test_reply_to_helper() {
        let hub = ChatHub::new();
        hub.reply_to("telegram", "123", "Reply", "mid_456").await.unwrap();
        
        let mut rx = hub.outbound_rx.write().await;
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.content, "Reply");
        assert_eq!(msg.reply_to, Some("mid_456".to_string()));
    }
}
