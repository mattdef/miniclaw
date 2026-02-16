use crate::channels::Channel;
use crate::chat::{ChatHub, InboundMessage, OutboundMessage};
use crate::utils::security::WhitelistChecker;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, Update};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub const TELEGRAM_CHANNEL_NAME: &str = "telegram";
const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Invalid token format: {0}. Check @BotFather (https://t.me/BotFather) for a valid token.")]
    InvalidTokenFormat(String),
    
    #[error("Telegram API error: {0}")]
    ApiError(String),
    
    #[error("Channel not initialized")]
    NotInitialized,
    
    #[error("Send error: {0}")]
    SendError(#[from] mpsc::error::SendError<InboundMessage>),
}

/// Telegram channel adapter that connects to Telegram Bot API via teloxide.
///
/// Handles:
/// - Long-polling message receiving (30s timeout)
/// - Text message processing only (MVP)
/// - Outbound message delivery
/// - Token validation
/// - User whitelist checking (NFR-S5)
pub struct TelegramChannel {
    bot: Bot,
    whitelist: WhitelistChecker,
    inbound_tx: Arc<RwLock<Option<mpsc::Sender<InboundMessage>>>>,
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

impl TelegramChannel {
    /// Create a new TelegramChannel with the given bot token and whitelist.
    ///
    /// Validates token format before creating the bot instance.
    /// Token format should be: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
    ///
    /// # Arguments
    /// * `token` - The Telegram bot token from @BotFather
    /// * `allowed_users` - List of Telegram user IDs allowed to interact with the bot
    pub fn new(token: String, allowed_users: Vec<i64>) -> Result<Self> {
        // Validate token format before creating bot
        if !is_valid_token_format(&token) {
            return Err(TelegramError::InvalidTokenFormat(
                "Token must be in format: digits:alphanumeric (e.g., 123456789:ABC...)".to_string()
            ).into());
        }

        let bot = Bot::new(token);
        let whitelist = WhitelistChecker::new(allowed_users);

        Ok(Self {
            bot,
            whitelist,
            inbound_tx: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        })
    }

    /// Shutdown the channel gracefully
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
            tracing::info!("Telegram channel shutdown signal sent");
        }
        Ok(())
    }

    /// Process an inbound Telegram message and convert it to our InboundMessage format.
    ///
    /// Extracts:
    /// - chat_id from message.chat.id
    /// - content from message.text (empty string if None)
    /// - Adds metadata with message_id if available
    fn process_inbound_message(msg: &Message) -> InboundMessage {
        let chat_id = msg.chat.id.0.to_string();
        let content = msg.text().unwrap_or("").to_string();
        
        let mut inbound = InboundMessage::new(TELEGRAM_CHANNEL_NAME, chat_id, content);
        
        // Add metadata
        if let Some(user) = &msg.from {
            inbound = inbound.with_metadata("user_id", serde_json::json!(user.id.0));
            if let Some(username) = &user.username {
                inbound = inbound.with_metadata("username", serde_json::json!(username));
            }
        }
        
        // msg.id is a MessageId(i32)
        inbound = inbound.with_metadata("message_id", serde_json::json!(msg.id.0));

        inbound
    }

    /// Send a message via Telegram API with validation
    async fn send_message(bot: &Bot, message: OutboundMessage) -> Result<()> {
        // Validate message length (Telegram limit is 4096 characters)
        let content = if message.content.len() > TELEGRAM_MAX_MESSAGE_LENGTH {
            tracing::warn!(
                chat_id = %message.chat_id,
                original_length = message.content.len(),
                "Outbound message too long, truncating to {} characters",
                TELEGRAM_MAX_MESSAGE_LENGTH
            );
            message.content.chars().take(TELEGRAM_MAX_MESSAGE_LENGTH).collect::<String>()
        } else {
            message.content.clone()
        };

        // Parse chat_id
        let chat_id: i64 = message.chat_id.parse()
            .with_context(|| format!("Invalid chat_id format: {}", message.chat_id))?;

        // Build send message request
        let send_request = bot.send_message(ChatId(chat_id), &content);

        // Handle reply_to if present (note: not fully implemented in MVP)
        if message.reply_to.is_some() {
            tracing::debug!("reply_to functionality not implemented in MVP");
        }

        // Send the message
        match send_request.await {
            Ok(sent_msg) => {
                tracing::debug!(
                    chat_id = %chat_id,
                    message_id = ?sent_msg.id.0,
                    "Sent message to Telegram"
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to send Telegram message to chat {}: {}", chat_id, e);
                tracing::error!("{}", error_msg);
                Err(TelegramError::ApiError(error_msg).into())
            }
        }
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    async fn start(&self, hub: Arc<ChatHub>) -> Result<()> {
        // Store inbound sender for use by message handlers
        let inbound_sender = hub.inbound_sender();
        {
            let mut tx = self.inbound_tx.write().await;
            *tx = Some(inbound_sender);
        }

        // Create shutdown channel for graceful termination
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        {
            let mut tx = self.shutdown_tx.write().await;
            *tx = Some(shutdown_tx);
        }

        // Create channel for receiving outbound messages from ChatHub
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<OutboundMessage>(100);
        
        // Register this channel with the ChatHub so it can route messages to us
        hub.register_channel(TELEGRAM_CHANNEL_NAME, outbound_tx).await?;

        // Clone bot for the dispatcher
        let bot = self.bot.clone();
        let inbound_tx = self.inbound_tx.clone();

        // Clone whitelist for the handler
        let whitelist = self.whitelist.clone();

        // Spawn the inbound message handler (dispatcher)
        tokio::spawn(async move {
            // Create dispatcher that handles only text messages (Update::Message)
            let handler = Update::filter_message()
                .endpoint(move |msg: Message, _bot: Bot| {
                    let inbound_tx = inbound_tx.clone();
                    let whitelist = whitelist.clone();
                    async move {
                        // Extract user_id and check whitelist (NFR-S5)
                        let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
                        
                        if let Some(user_id) = user_id {
                            if !whitelist.is_allowed(user_id) {
                                // User not whitelisted - silently drop message (AC 4)
                                tracing::debug!("Message from non-whitelisted user {}", user_id);
                                return Ok::<(), TelegramError>(());
                            }
                        } else {
                            // No user info - reject for security
                            tracing::debug!("Message without user info, dropping");
                            return Ok::<(), TelegramError>(());
                        }

                        // Process the message
                        let inbound = Self::process_inbound_message(&msg);

                        // Log receipt
                        tracing::info!(
                            chat_id = %msg.chat.id.0,
                            content_preview = %inbound.content.chars().take(50).collect::<String>(),
                            "Received Telegram message"
                        );

                        // Send to ChatHub
                        let tx = {
                            let guard = inbound_tx.read().await;
                            guard.clone()
                        };

                        if let Some(tx) = tx {
                            if let Err(e) = tx.send(inbound).await {
                                tracing::error!("Failed to send inbound message to ChatHub: {}", e);
                            }
                        } else {
                            tracing::error!("Inbound channel not initialized");
                        }

                        Ok::<(), TelegramError>(())
                    }
                });

            // Build dispatcher with shutdown support
            let mut dispatcher = Dispatcher::builder(bot, handler)
                .default_handler(|_upd| async move {
                    // Log non-message updates at debug level (MVP ignores these)
                    tracing::debug!("Ignoring non-message update (MVP only handles text messages)");
                })
                .build();

            // Run dispatcher with shutdown signal
            tokio::select! {
                _ = dispatcher.dispatch() => {
                    tracing::info!("Telegram dispatcher stopped");
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Telegram dispatcher received shutdown signal");
                }
            }
        });

        // Spawn outbound message handler
        let bot_for_outbound = self.bot.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(message) = outbound_rx.recv() => {
                        // Validate and send the message
                        if let Err(e) = Self::send_message(&bot_for_outbound, message).await {
                            tracing::error!("Failed to send outbound message: {}", e);
                        }
                    }
                    else => break,
                }
            }
            tracing::info!("Telegram outbound handler stopped");
        });

        tracing::info!("Telegram channel connected and listening for messages");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        Self::send_message(&self.bot, message).await
    }
}

/// Validates Telegram bot token format.
///
/// Expected format: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
/// - Bot ID must be all digits
/// - Must contain exactly one colon separator
/// - Secret part must be non-empty
pub fn is_valid_token_format(token: &str) -> bool {
    // Split by colon
    let parts: Vec<&str> = token.split(':').collect();
    
    // Must have exactly 2 parts
    if parts.len() != 2 {
        return false;
    }
    
    let bot_id = parts[0];
    let secret = parts[1];
    
    // Bot ID must be all digits
    if bot_id.is_empty() || !bot_id.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    
    // Secret must be non-empty
    if secret.is_empty() {
        return false;
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_token_format() {
        assert!(is_valid_token_format("123456789:ABCdefGHIjklMNOpqrsTUVwxyz"));
        assert!(is_valid_token_format("987654321:xyz123"));
        assert!(is_valid_token_format("1:a"));
    }

    #[test]
    fn test_invalid_token_format() {
        // Empty token
        assert!(!is_valid_token_format(""));
        
        // No colon
        assert!(!is_valid_token_format("invalid"));
        
        // Non-digit bot ID
        assert!(!is_valid_token_format("abc:def"));
        
        // Empty secret
        assert!(!is_valid_token_format("123:"));
        
        // Multiple colons
        assert!(!is_valid_token_format("123:abc:def"));
        
        // Empty bot ID
        assert!(!is_valid_token_format(":secret"));
        
        // Whitespace in bot ID
        assert!(!is_valid_token_format("123 456:secret"));
    }

    #[test]
    fn test_telegram_channel_creation_with_whitelist() {
        // Test creating a channel with empty whitelist
        let channel = TelegramChannel::new(
            "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            vec![]
        );
        assert!(channel.is_ok());

        // Test creating a channel with whitelisted users
        let channel = TelegramChannel::new(
            "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            vec![123_456_789, 987_654_321]
        );
        assert!(channel.is_ok());
    }

    #[test]
    fn test_telegram_channel_creation_invalid_token() {
        // Should fail with invalid token
        let channel = TelegramChannel::new(
            "invalid-token".to_string(),
            vec![]
        );
        assert!(channel.is_err());
    }

    #[test]
    fn test_telegram_error_display() {
        let err = TelegramError::InvalidTokenFormat("test".to_string());
        assert!(err.to_string().contains("Invalid token format"));

        let err = TelegramError::ApiError("API failed".to_string());
        assert!(err.to_string().contains("API error"));

        let err = TelegramError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));
    }
}
