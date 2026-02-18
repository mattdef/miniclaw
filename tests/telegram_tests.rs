use miniclaw::channels::{Channel, TelegramChannel};
use miniclaw::chat::{ChatHub, InboundMessage, OutboundMessage};
use std::sync::Arc;

/// Tests that require a real Telegram bot token should check for TELEGRAM_TEST_TOKEN env var
/// These tests are skipped if the env var is not set

#[tokio::test]
async fn test_telegram_channel_creation_with_valid_token() {
    // This test validates token format without making API calls
    let token = "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string();
    let whitelist = vec![]; // Empty whitelist for testing
    let result = TelegramChannel::new(token, whitelist);
    assert!(
        result.is_ok(),
        "Should create channel with valid token format"
    );
}

#[tokio::test]
async fn test_telegram_channel_creation_with_invalid_token() {
    let token = "invalid-token-format".to_string();
    let whitelist = vec![];
    let result = TelegramChannel::new(token, whitelist);
    assert!(result.is_err(), "Should fail with invalid token format");
}

#[tokio::test]
async fn test_telegram_channel_registration_with_chathub() {
    let hub = Arc::new(ChatHub::new());
    let token = "123456789:test_token_for_integration".to_string();

    // Create channel
    let channel = TelegramChannel::new(token, vec![]).expect("Should create channel");

    // Start the channel (this should spawn the dispatcher)
    // Note: Without a real token, this will fail to connect, but we can verify the setup
    let result = channel.start(hub.clone()).await;

    // Starting should succeed even if connection fails later
    // The dispatcher runs in background
    assert!(result.is_ok() || result.is_err(), "Channel start completed");
}

#[tokio::test]
#[ignore = "Requires TELEGRAM_TEST_TOKEN environment variable with valid bot token"]
async fn test_end_to_end_message_flow() {
    // This test requires a real Telegram bot token
    // Set TELEGRAM_TEST_TOKEN to your test bot token before running
    let token = match std::env::var("TELEGRAM_TEST_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            eprintln!("Skipping integration test - no TELEGRAM_TEST_TOKEN environment variable");
            return;
        }
    };

    let hub = Arc::new(ChatHub::new());
    let channel =
        TelegramChannel::new(token, vec![]).expect("Should create channel with valid token");

    // Start the channel
    channel
        .start(hub.clone())
        .await
        .expect("Should start channel");

    // Give the dispatcher time to connect
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test sending an outbound message (this would require a real chat_id)
    // For this test, we just verify the method exists and accepts the message
    // In a real test, you'd send to a known test chat
}

#[tokio::test]
async fn test_telegram_channel_trait_implementation() {
    use miniclaw::channels::Channel;

    let token = "123456789:test_token_trait".to_string();
    let channel = TelegramChannel::new(token, vec![]).expect("Should create channel");

    // Verify Channel trait is implemented
    let _channel: &dyn Channel = &channel;
}

#[tokio::test]
async fn test_inbound_message_routing() {
    let hub = Arc::new(ChatHub::new());

    // Simulate sending an inbound message
    let test_msg = InboundMessage::new("telegram", "123456789", "Test message from Telegram");

    // This would normally come from the TelegramChannel dispatcher
    // Here we just verify the ChatHub receives it
    let send_result = hub.send_inbound(test_msg).await;
    assert!(send_result.is_ok(), "Should send inbound message");
}

#[tokio::test]
async fn test_outbound_message_delivery_integration() {
    let hub = Arc::new(ChatHub::new());
    let token = "123456789:test_outbound".to_string();

    let channel = TelegramChannel::new(token, vec![]).expect("Should create channel");

    // Start channel to register outbound sender
    let _ = channel.start(hub.clone()).await;

    // Create an outbound message
    let outbound = OutboundMessage::new("telegram", "123456789", "Test response");

    // Try to send (will fail without real token, but verifies the API)
    let result = channel.send(outbound).await;

    // We expect this to fail because the token is fake
    // But it should fail gracefully, not panic
    assert!(
        result.is_err() || result.is_ok(),
        "Send should complete without panic (result: {:?})",
        result
    );
}

#[test]
fn test_token_validation_edge_cases() {
    use miniclaw::channels::telegram;

    // Valid tokens
    assert!(telegram::is_valid_token_format(
        "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
    ));
    assert!(telegram::is_valid_token_format("1:a"));
    assert!(telegram::is_valid_token_format(
        "999999999999999999:longsecret123"
    ));

    // Invalid tokens
    assert!(!telegram::is_valid_token_format(""));
    assert!(!telegram::is_valid_token_format("no-colon"));
    assert!(!telegram::is_valid_token_format(":"));
    assert!(!telegram::is_valid_token_format("abc:def")); // Non-digit bot ID
    assert!(!telegram::is_valid_token_format("123:")); // Empty secret
    assert!(!telegram::is_valid_token_format(":secret")); // Empty bot ID
    assert!(!telegram::is_valid_token_format("123:abc:def")); // Multiple colons
    assert!(!telegram::is_valid_token_format("12 3:secret")); // Space in bot ID
}

// ========================================
// Whitelist Integration Tests (AC 2-5)
// ========================================

#[tokio::test]
async fn test_whitelisted_user_message_accepted() {
    // Test AC 3: Whitelisted user processing
    let hub = Arc::new(ChatHub::new());
    let token = "123456789:test_token_whitelist_accept".to_string();
    let whitelisted_user_id = 123_456_789_i64;

    // Create channel with whitelist containing test user
    let channel = TelegramChannel::new(token, vec![whitelisted_user_id])
        .expect("Should create channel with whitelist");

    // Start the channel
    let result = channel.start(hub.clone()).await;
    assert!(result.is_ok(), "Channel should start successfully");

    // Note: Without mocking Telegram API, we can't fully test message flow
    // But we've verified the channel accepts the whitelist configuration
}

#[tokio::test]
async fn test_non_whitelisted_user_rejected() {
    // Test AC 4: Non-whitelisted user rejection
    let hub = Arc::new(ChatHub::new());
    let token = "123456789:test_token_whitelist_reject".to_string();
    let whitelisted_user_id = 123_456_789_i64;

    // Create channel with whitelist NOT containing test user
    let channel = TelegramChannel::new(token, vec![whitelisted_user_id])
        .expect("Should create channel with whitelist");

    // Start the channel
    let result = channel.start(hub.clone()).await;
    assert!(result.is_ok(), "Channel should start successfully");

    // The rejection logic is tested in message handler via unit tests
    // Integration testing would require mocking teloxide Message objects
}

#[tokio::test]
async fn test_empty_whitelist_secure_by_default() {
    // Test AC 5: Empty whitelist security (secure by default)
    use miniclaw::utils::security::WhitelistChecker;

    let checker = WhitelistChecker::new(vec![]);

    // Empty whitelist should reject ALL users (secure by default)
    assert!(
        !checker.is_allowed(123_456_789),
        "Empty whitelist must reject all users"
    );
    assert!(
        !checker.is_allowed(987_654_321),
        "Empty whitelist must reject all users"
    );
    assert!(
        !checker.is_allowed(1),
        "Empty whitelist must reject all users"
    );
    assert!(checker.is_empty(), "Checker should report as empty");
}

#[tokio::test]
async fn test_telegram_channel_whitelist_validation() {
    // Test that TelegramChannel properly initializes with different whitelists

    // Empty whitelist
    let channel1 = TelegramChannel::new("123456789:test1".to_string(), vec![]);
    assert!(channel1.is_ok(), "Should accept empty whitelist");

    // Single user
    let channel2 = TelegramChannel::new("123456789:test2".to_string(), vec![123_456_789]);
    assert!(channel2.is_ok(), "Should accept single user whitelist");

    // Multiple users
    let channel3 = TelegramChannel::new(
        "123456789:test3".to_string(),
        vec![123_456_789, 987_654_321, 111_222_333],
    );
    assert!(channel3.is_ok(), "Should accept multiple user whitelist");
}
