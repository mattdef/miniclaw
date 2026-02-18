use crate::chat::types::{InboundMessage, OutboundMessage};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::sync::mpsc;

const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 100;
const BUFFER_WAIT_MS: u64 = 100;

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

/// Callback type for delivery failure notifications
pub type DeliveryFailureCallback = Arc<dyn Fn(String, OutboundMessage) + Send + Sync>;

pub struct ChatHub {
    inbound_tx: mpsc::Sender<InboundMessage>,
    inbound_rx: Arc<RwLock<mpsc::Receiver<InboundMessage>>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Arc<RwLock<mpsc::Receiver<OutboundMessage>>>,
    channels: Arc<RwLock<HashMap<String, mpsc::Sender<OutboundMessage>>>>,
    delivery_failure_callback: Option<DeliveryFailureCallback>,
    agent_tx: Option<mpsc::Sender<InboundMessage>>,
}

impl ChatHub {
    pub fn new() -> Self {
        Self::with_capacities(100, 100)
    }

    pub fn with_capacities(inbound_capacity: usize, outbound_capacity: usize) -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(inbound_capacity);
        let (outbound_tx, outbound_rx) = mpsc::channel(outbound_capacity);

        Self {
            inbound_tx,
            inbound_rx: Arc::new(RwLock::new(inbound_rx)),
            outbound_tx,
            outbound_rx: Arc::new(RwLock::new(outbound_rx)),
            channels: Arc::new(RwLock::new(HashMap::new())),
            delivery_failure_callback: None,
            agent_tx: None,
        }
    }

    /// Register a callback to be notified when message delivery fails
    pub fn on_delivery_failure(
        &mut self,
        callback: impl Fn(String, OutboundMessage) + Send + Sync + 'static,
    ) {
        self.delivery_failure_callback = Some(Arc::new(callback));
    }

    /// Notify agent of delivery failure
    async fn notify_delivery_failure(&self, error: String, message: OutboundMessage) {
        if let Some(callback) = &self.delivery_failure_callback {
            callback(error, message);
        }
    }

    /// Register a sender for forwarding messages to the AgentLoop.
    /// This connects the ChatHub to the AgentLoop for message processing.
    pub fn register_agent_sender(&mut self, sender: mpsc::Sender<InboundMessage>) {
        tracing::info!("AgentLoop sender registered with ChatHub");
        self.agent_tx = Some(sender);
    }

    pub fn inbound_sender(&self) -> mpsc::Sender<InboundMessage> {
        self.inbound_tx.clone()
    }

    pub fn outbound_sender(&self) -> mpsc::Sender<OutboundMessage> {
        self.outbound_tx.clone()
    }

    pub async fn register_channel(
        &self,
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
                // AC 4: Wait briefly for space before dropping
                tracing::warn!("Outbound buffer full, waiting briefly for space");
                tokio::time::sleep(Duration::from_millis(BUFFER_WAIT_MS)).await;

                // Try again after waiting
                match self.outbound_tx.try_send(msg) {
                    Ok(_) => Ok(()),
                    Err(mpsc::error::TrySendError::Full(msg)) => {
                        tracing::warn!("Outbound buffer still full, dropping oldest message");
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
            // AC 2 & 3: Retry with exponential backoff
            let mut retry_count = 0;
            let mut delay_ms = INITIAL_RETRY_DELAY_MS;

            loop {
                match sender.try_send(message.clone()) {
                    Ok(_) => return Ok(()),
                    Err(mpsc::error::TrySendError::Full(_)) if retry_count < MAX_RETRIES => {
                        tracing::warn!(
                            retry = retry_count + 1,
                            delay_ms = delay_ms,
                            "Channel full, retrying with backoff"
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                        retry_count += 1;
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Failed to send after {} retries: {}", retry_count, e);
                        tracing::error!(error = %error_msg, "Message delivery failed");

                        // AC 3: Notify agent of delivery failure
                        self.notify_delivery_failure(error_msg.clone(), message)
                            .await;

                        return Err(ChatError::SendError(error_msg));
                    }
                }
            }
        } else {
            let error_msg = format!("Channel not found: {}", message.channel);
            tracing::error!(error = %error_msg);

            // AC 3: Notify agent of delivery failure
            self.notify_delivery_failure(error_msg.clone(), message)
                .await;

            Err(ChatError::ChannelNotFound(error_msg))
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
                    // Forward to AgentLoop if connected
                    if let Some(agent_tx) = &self.agent_tx {
                        match agent_tx.try_send(msg) {
                            Ok(_) => {
                                tracing::trace!("Message forwarded to AgentLoop");
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                tracing::warn!("AgentLoop buffer full, message dropped");
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                tracing::error!("AgentLoop receiver closed");
                            }
                        }
                    }
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

    /// Attempts to enqueue an outbound message without awaiting.
    /// If the buffer is full, drops the oldest message and retries once.
    pub fn try_send_outbound(&self, message: OutboundMessage) -> Result<()> {
        match self.outbound_tx.try_send(message) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(msg)) => {
                tracing::warn!("Outbound buffer full, dropping oldest message");
                if let Ok(mut rx) = self.outbound_rx.try_write() {
                    let _ = rx.try_recv();
                }
                self.outbound_tx
                    .try_send(msg)
                    .map_err(|e| ChatError::SendError(e.to_string()))
            }
            Err(e) => Err(ChatError::SendError(e.to_string())),
        }
    }

    #[cfg(test)]
    pub async fn test_try_recv_outbound(&self) -> Option<OutboundMessage> {
        let mut rx = self.outbound_rx.write().await;
        rx.try_recv().ok()
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
            hub.send_inbound(InboundMessage::new("test", "123", format!("msg {}", i)))
                .await
                .unwrap();
        }

        // Send one more - should trigger overflow logic
        hub.send_inbound(InboundMessage::new("test", "123", "overflow"))
            .await
            .unwrap();

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
        hub.send_inbound(InboundMessage::new("test", "123", "   "))
            .await
            .unwrap();

        // Send valid message
        hub.send_inbound(InboundMessage::new("test", "123", "valid"))
            .await
            .unwrap();

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
        hub.reply_to("telegram", "123", "Reply", "mid_456")
            .await
            .unwrap();

        let mut rx = hub.outbound_rx.write().await;
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.content, "Reply");
        assert_eq!(msg.reply_to, Some("mid_456".to_string()));
    }

    #[tokio::test]
    async fn test_buffer_overflow_outbound() {
        let hub = ChatHub::new();

        // Fill the buffer (capacity 100)
        for i in 0..100 {
            hub.send_outbound(OutboundMessage::new("test", "123", format!("msg {}", i)))
                .await
                .unwrap();
        }

        // Send one more - should trigger overflow logic (wait briefly, then drop oldest)
        hub.send_outbound(OutboundMessage::new("test", "123", "overflow"))
            .await
            .unwrap();

        // Check that we can still receive
        let mut rx = hub.outbound_rx.write().await;
        let msg1 = rx.recv().await.unwrap();
        // Since we dropped the oldest (msg 0), the first one should be msg 1
        assert_eq!(msg1.content, "msg 1");
    }

    #[tokio::test]
    async fn test_delivery_failure_notification() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut hub = ChatHub::new();
        let notification_received = Arc::new(AtomicBool::new(false));
        let notification_received_clone = notification_received.clone();

        // Register a delivery failure callback
        hub.on_delivery_failure(move |_error: String, _msg: OutboundMessage| {
            notification_received_clone.store(true, Ordering::SeqCst);
        });

        // Try to route to unregistered channel - should trigger failure notification
        let msg = OutboundMessage::new("unknown", "123", "Test");
        let result = hub.route_outbound(msg).await;

        assert!(result.is_err());
        // Give async callback time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(
            notification_received.load(Ordering::SeqCst),
            "Delivery failure notification should have been received"
        );
    }

    #[test]
    fn test_register_agent_sender() {
        let mut hub = ChatHub::new();
        let (tx, _rx) = mpsc::channel(10);
        hub.register_agent_sender(tx);
        assert!(hub.agent_tx.is_some());
    }

    #[tokio::test]
    async fn test_message_forwarding_to_agent() {
        let mut hub = ChatHub::new();
        let (agent_tx, mut agent_rx) = mpsc::channel(10);
        hub.register_agent_sender(agent_tx);

        // Send message via inbound_sender
        let msg = InboundMessage::new("telegram", "123", "Test message");
        let inbound_tx = hub.inbound_sender();
        inbound_tx.send(msg).await.unwrap();

        // Simulate ChatHub.run() processing by calling recv_inbound
        let received = hub.recv_inbound().await;
        assert!(received.is_some());

        // Verify message was forwarded to agent_rx
        let forwarded = agent_rx.recv().await;
        assert!(forwarded.is_some());
        assert_eq!(forwarded.unwrap().content, "Test message");
    }

    #[tokio::test]
    async fn test_agent_buffer_full_drops_message() {
        let mut hub = ChatHub::new();
        let (agent_tx, mut agent_rx) = mpsc::channel(2);
        hub.register_agent_sender(agent_tx);

        // Fill the agent buffer
        for i in 0..2 {
            let msg = InboundMessage::new("telegram", "123", format!("msg {}", i));
            let inbound_tx = hub.inbound_sender();
            inbound_tx.send(msg).await.unwrap();
            hub.recv_inbound().await; // Process and forward
        }

        // Verify both messages in agent_rx
        assert!(agent_rx.recv().await.is_some());
        assert!(agent_rx.recv().await.is_some());

        // Send third message - should drop due to full buffer
        let msg = InboundMessage::new("telegram", "123", "overflow");
        let inbound_tx = hub.inbound_sender();
        inbound_tx.send(msg).await.unwrap();
        hub.recv_inbound().await; // Process and forward - should drop

        // Agent buffer should be empty (message was dropped)
        assert!(agent_rx.try_recv().is_err());
    }
}
