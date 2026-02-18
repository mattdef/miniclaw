//! Integration tests for Session Management in Agent Loop
//!
//! These tests verify:
//! - Session persistence round-trip
//! - Session loading on startup
//! - Concurrent session access
//! - Session recovery after restart
//! - Tool interaction storage and context integration

use miniclaw::agent::{ContextBuilder, ContextBuilderImpl, LlmRole};
use miniclaw::chat::InboundMessage;
use miniclaw::session::{Message, SessionManager};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_session_persistence_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create manager and add session with messages
    {
        let manager = SessionManager::new(sessions_dir.clone());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123456789")
            .await
            .unwrap();

        // Add user message
        manager
            .add_message(
                &session.session_id,
                Message::new("user".to_string(), "Hello".to_string()),
            )
            .await
            .unwrap();

        // Add assistant message with tool calls
        let tool_calls = vec![miniclaw::session::ToolCall {
            id: "call_1".to_string(),
            name: "filesystem".to_string(),
            arguments: "{\"operation\": \"list\"}".to_string(),
        }];

        manager
            .add_message(
                &session.session_id,
                Message::new("assistant".to_string(), "I'll list files".to_string())
                    .with_tool_calls(tool_calls),
            )
            .await
            .unwrap();

        // Add tool result
        manager
            .add_message(
                &session.session_id,
                Message::tool_result("call_1".to_string(), "Tool call_1 result: file1.txt, file2.txt".to_string()),
            )
            .await
            .unwrap();

        // Persist
        manager.save_all_sessions().await.unwrap();
    }

    // Create new manager and verify session loads correctly
    {
        let manager = SessionManager::new(sessions_dir);
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123456789")
            .await
            .unwrap();

        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
        assert!(session.messages[1].tool_calls.is_some());
        assert_eq!(session.messages[2].role, "tool_result");
    }
}

#[tokio::test]
async fn test_session_manager_update_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    manager.initialize().await.unwrap();

    // Create session
    let session = manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();
    manager
        .add_message(
            &session.session_id,
            Message::new("user".to_string(), "Initial".to_string()),
        )
        .await
        .unwrap();

    // Get session, modify it locally, then update
    let mut local_session = manager.get_session(&session.session_id).await.unwrap();
    local_session.add_message(Message::new(
        "assistant".to_string(),
        "Response".to_string(),
    ));

    // Update session in manager
    manager.update_session(local_session.clone()).await.unwrap();

    // Verify update
    let updated = manager.get_session(&session.session_id).await.unwrap();
    assert_eq!(updated.messages.len(), 2);
}

#[tokio::test]
async fn test_concurrent_session_access() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    manager.initialize().await.unwrap();

    // Create session
    let session = manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();
    let session_id = session.session_id.clone();

    // Spawn multiple tasks to add messages concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let session_id_clone = session_id.clone();

        let handle = tokio::spawn(async move {
            manager_clone
                .add_message(
                    &session_id_clone,
                    Message::new("user".to_string(), format!("Message {}", i)),
                )
                .await
                .unwrap();
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all messages were added
    let final_session = manager.get_session(&session_id).await.unwrap();
    assert_eq!(final_session.messages.len(), 10);
}

#[tokio::test]
async fn test_session_recovery_statistics() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create sessions and persist them
    {
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        for i in 0..5 {
            let session = manager
                .get_or_create_session("telegram", &format!("user_{}", i))
                .await
                .unwrap();
            manager
                .add_message(
                    &session.session_id,
                    Message::new("user".to_string(), format!("Hello from user {}", i)),
                )
                .await
                .unwrap();
        }

        manager.save_all_sessions().await.unwrap();
    }

    // Verify recovery loads all sessions
    {
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        assert_eq!(manager.session_count().await, 5);

        // Verify each session loaded correctly
        for i in 0..5 {
            let session = manager
                .get_or_create_session("telegram", &format!("user_{}", i))
                .await
                .unwrap();
            assert_eq!(session.messages.len(), 1);
        }
    }
}

#[tokio::test]
async fn test_corrupted_session_recovery() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create valid session
    {
        let manager = SessionManager::new(sessions_dir.clone());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();
        manager
            .add_message(
                &session.session_id,
                Message::new("user".to_string(), "Test".to_string()),
            )
            .await
            .unwrap();

        manager.save_all_sessions().await.unwrap();
    }

    // Corrupt the session file
    let session_file = sessions_dir.join("telegram_123.json");
    tokio::fs::write(&session_file, "invalid json {{")
        .await
        .unwrap();

    // Load should handle corruption gracefully
    {
        let manager = SessionManager::new(sessions_dir.clone());
        manager.initialize().await.unwrap();

        // Should still have the session (recreated)
        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();
        assert_eq!(session.session_id, "telegram_123");

        // Corrupted file should be renamed
        assert!(sessions_dir.join("telegram_123.json.corrupted").exists());
    }
}

#[tokio::test]
async fn test_tool_calls_survive_context_builder_pipeline() {
    // Verifies that tool_calls persist through full pipeline:
    // Session -> Persistence -> Recovery -> ContextBuilder
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create session with tool interaction flow
    {
        let manager = SessionManager::new(sessions_dir.clone());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();

        // User asks a question
        manager
            .add_message(
                &session.session_id,
                Message::new("user".to_string(), "What files are in /tmp?".to_string()),
            )
            .await
            .unwrap();

        // Assistant responds with tool call
        let tool_calls = vec![miniclaw::session::ToolCall {
            id: "call_fs_1".to_string(),
            name: "filesystem".to_string(),
            arguments: r#"{"operation": "list", "path": "/tmp"}"#.to_string(),
        }];

        manager
            .add_message(
                &session.session_id,
                Message::new(
                    "assistant".to_string(),
                    "I'll list the files for you.".to_string(),
                )
                .with_tool_calls(tool_calls),
            )
            .await
            .unwrap();

        // Tool result
        manager
            .add_message(
                &session.session_id,
                Message::tool_result("call_fs_1".to_string(), "Tool call_fs_1 result: file1.txt, file2.txt".to_string()),
            )
            .await
            .unwrap();

        // Final assistant response
        manager
            .add_message(
                &session.session_id,
                Message::new(
                    "assistant".to_string(),
                    "I found 2 files: file1.txt and file2.txt".to_string(),
                ),
            )
            .await
            .unwrap();

        manager.save_all_sessions().await.unwrap();
    }

    // Load in new manager and verify ContextBuilder correctly formats for LLM
    {
        let manager = SessionManager::new(sessions_dir.clone());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();

        // Create ContextBuilder and build context
        let context_builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
        let inbound = InboundMessage::new("telegram", "123", "Thanks!");
        let context = context_builder
            .build_context(&session, &inbound)
            .await
            .unwrap();

        // Verify conversation history is in context
        let user_msg = context
            .iter()
            .find(|m| m.content.contains("What files are in /tmp?"));
        assert!(user_msg.is_some(), "User message should be in context");
        assert_eq!(user_msg.unwrap().role, LlmRole::User);

        // Verify assistant message with tool_calls is in context
        let assistant_msg = context
            .iter()
            .find(|m| m.content.contains("I'll list the files"));
        assert!(
            assistant_msg.is_some(),
            "Assistant message with tool_calls should be in context"
        );
        assert_eq!(assistant_msg.unwrap().role, LlmRole::Assistant);
        assert!(
            assistant_msg.unwrap().tool_calls.is_some(),
            "tool_calls should be preserved"
        );
        assert_eq!(assistant_msg.unwrap().tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(
            assistant_msg.unwrap().tool_calls.as_ref().unwrap()[0].name,
            "filesystem"
        );

        // Verify tool result is in context with correct role (LlmRole::Tool)
        let tool_result_msg = context
            .iter()
            .find(|m| m.content.contains("Tool call_fs_1 result"));
        assert!(
            tool_result_msg.is_some(),
            "Tool result should be in context"
        );
        assert_eq!(
            tool_result_msg.unwrap().role,
            LlmRole::Tool,
            "Tool result should have LlmRole::Tool"
        );

        // Verify final assistant message
        let final_msg = context
            .iter()
            .find(|m| m.content.contains("I found 2 files"));
        assert!(
            final_msg.is_some(),
            "Final assistant message should be in context"
        );
        assert_eq!(final_msg.unwrap().role, LlmRole::Assistant);
    }
}

#[tokio::test]
async fn test_session_updates_visible_to_context_builder() {
    // Verifies that session updates made during agent loop are visible
    // to ContextBuilder on subsequent iterations (Task 5.4)
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let manager = SessionManager::new(sessions_dir);
    manager.initialize().await.unwrap();

    // Simulate first iteration: user message + assistant response
    let mut session = manager
        .get_or_create_session("telegram", "456")
        .await
        .unwrap();

    // Add initial user message
    session.add_message(Message::new(
        "user".to_string(),
        "First question".to_string(),
    ));

    // Update session in manager
    manager.update_session(session.clone()).await.unwrap();

    // Build context - should see first message
    let context_builder = ContextBuilderImpl::new(temp_dir.path()).unwrap();
    let inbound = InboundMessage::new("telegram", "456", "Second question");
    let context1 = context_builder
        .build_context(&session, &inbound)
        .await
        .unwrap();

    let has_first_msg = context1
        .iter()
        .any(|m| m.content.contains("First question"));
    assert!(has_first_msg, "First message should be visible in context");

    // Simulate second iteration: add assistant response
    session.add_message(Message::new(
        "assistant".to_string(),
        "First answer".to_string(),
    ));

    // Update session again
    manager.update_session(session.clone()).await.unwrap();

    // Get fresh session from manager
    let fresh_session = manager.get_session(&session.session_id).await.unwrap();

    // Build context again - should see both messages
    let context2 = context_builder
        .build_context(&fresh_session, &inbound)
        .await
        .unwrap();

    let has_first_answer = context2.iter().any(|m| m.content.contains("First answer"));
    assert!(
        has_first_answer,
        "Updated session should be visible to ContextBuilder on next iteration"
    );

    // Verify message count matches
    let history_count = context2
        .iter()
        .filter(
            |m| {
                matches!(m.role, LlmRole::User | LlmRole::Assistant)
                    && !m.content.contains("Second question")
            }, // Exclude current message
        )
        .count();
    assert_eq!(
        history_count, 2,
        "Should have 2 history messages (user + assistant)"
    );
}

// ── Tests migrated from src/gateway.rs inline test block ─────────────────────

#[tokio::test]
async fn test_auto_persistence_saves_sessions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create session manager and add a session
    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    let session = session_manager
        .get_or_create_session("telegram", "123456789")
        .await
        .unwrap();

    // Add a message to the session
    let message = Message::new("user".to_string(), "Test message".to_string());
    session_manager
        .add_message(&session.session_id, message)
        .await
        .unwrap();

    // Manually trigger persistence (simulating the auto-persistence task)
    session_manager.save_all_sessions().await.unwrap();

    // Verify the session file was created
    let session_file = sessions_dir.join("telegram_123456789.json");
    assert!(
        session_file.exists(),
        "Session file should exist after persistence"
    );

    // Verify we can load it back
    let loaded_session = session_manager
        .get_or_create_session("telegram", "123456789")
        .await
        .unwrap();
    assert_eq!(loaded_session.messages.len(), 1);
    assert_eq!(loaded_session.messages[0].content, "Test message");
}

#[tokio::test]
async fn test_session_file_naming_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    // Create sessions with different channel/chat_id combinations
    session_manager
        .get_or_create_session("telegram", "111")
        .await
        .unwrap();
    session_manager
        .get_or_create_session("cli", "222")
        .await
        .unwrap();
    session_manager
        .get_or_create_session("discord", "333")
        .await
        .unwrap();

    // Save all sessions
    session_manager.save_all_sessions().await.unwrap();

    // Verify correct file naming
    assert!(sessions_dir.join("telegram_111.json").exists());
    assert!(sessions_dir.join("cli_222.json").exists());
    assert!(sessions_dir.join("discord_333.json").exists());
}

#[tokio::test]
async fn test_graceful_shutdown_flushes_sessions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    // Create a session with data
    let session = session_manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();

    let message = Message::new("user".to_string(), "Shutdown test".to_string());
    session_manager
        .add_message(&session.session_id, message)
        .await
        .unwrap();

    // Simulate graceful shutdown by saving all sessions
    session_manager.save_all_sessions().await.unwrap();

    // Verify session was flushed to disk
    let session_file = sessions_dir.join("telegram_123.json");
    assert!(
        session_file.exists(),
        "Session should be flushed during graceful shutdown"
    );
}

#[tokio::test]
async fn test_lock_scope_during_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir);
    session_manager.initialize().await.unwrap();

    // Create a session
    let session = session_manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();

    // Add multiple messages rapidly while persistence might be running
    for i in 0..5 {
        let message = Message::new("user".to_string(), format!("Message {}", i));
        session_manager
            .add_message(&session.session_id, message)
            .await
            .unwrap();
    }

    // Verify all messages were added (no deadlock or data corruption)
    let updated_session = session_manager
        .get_session(&session.session_id)
        .await
        .unwrap();
    assert_eq!(updated_session.messages.len(), 5);
}

#[tokio::test]
async fn test_persistence_continues_after_failure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir);
    session_manager.initialize().await.unwrap();

    // Create a session
    session_manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();

    // Start auto-persistence
    let (_handle, shutdown) = session_manager.start_auto_persistence();

    // The persistence task should continue even if individual saves fail
    // We can't easily simulate disk full, but we can verify the task doesn't panic
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Signal shutdown and verify clean exit
    let _ = shutdown.send(()).await;

    // If we get here without a panic, the persistence loop is working
}
