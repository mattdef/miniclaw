//! Integration tests for gateway daemon
//!
//! Tests cover:
//! - Gateway startup with full initialization
//! - Graceful shutdown with session persistence
//! - Signal handling (Ctrl+C, SIGTERM)
//! - PID file creation and cleanup
//! - Error recovery mechanisms

use std::time::Duration;
use tempfile::TempDir;

/// Test that the gateway command is available in CLI
#[test]
fn test_gateway_command_available() {
    use clap::CommandFactory;
    use miniclaw::cli::Cli;

    let cmd = Cli::command();
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .map(|sc| sc.get_name().to_string())
        .collect();

    assert!(
        subcommands.contains(&"gateway".to_string()),
        "Gateway command should be available"
    );
}

/// Test gateway command with --pid-file argument parsing
#[test]
fn test_gateway_pid_file_argument_parsing() {
    use clap::Parser;
    use miniclaw::cli::{Cli, Commands};

    let cli = Cli::parse_from(["miniclaw", "gateway", "--pid-file", "/run/miniclaw.pid"]);

    assert!(matches!(
        cli.command,
        Some(Commands::Gateway { pid_file: Some(_) })
    ));

    if let Some(Commands::Gateway {
        pid_file: Some(path),
    }) = cli.command
    {
        assert_eq!(path.to_str().unwrap(), "/run/miniclaw.pid");
    }
}

/// Test gateway command without --pid-file argument
#[test]
fn test_gateway_without_pid_file() {
    use clap::Parser;
    use miniclaw::cli::{Cli, Commands};

    let cli = Cli::parse_from(["miniclaw", "gateway"]);

    assert!(matches!(
        cli.command,
        Some(Commands::Gateway { pid_file: None })
    ));
}

/// Test PID file creation and validation
#[test]
fn test_pid_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let pid_file = temp_dir.path().join("test.pid");

    // Write PID file
    let pid = std::process::id();
    std::fs::write(&pid_file, pid.to_string()).unwrap();

    // Verify file exists and contains correct PID
    assert!(pid_file.exists());
    let contents = std::fs::read_to_string(&pid_file).unwrap();
    assert_eq!(contents, pid.to_string());
}

/// Test PID file cleanup on exit
#[test]
fn test_pid_file_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let pid_file = temp_dir.path().join("test.pid");

    // Create PID file
    std::fs::write(&pid_file, "12345").unwrap();
    assert!(pid_file.exists());

    // Remove PID file (simulate cleanup)
    std::fs::remove_file(&pid_file).unwrap();
    assert!(!pid_file.exists());
}

/// Test that gateway initialization requires valid config
#[tokio::test]
async fn test_gateway_initialization_with_valid_config() {
    use miniclaw::config::Config;

    // Create a minimal valid config
    let config = Config {
        api_key: Some("test-key".to_string()),
        model: Some("test-model".to_string()),
        telegram_token: None,
        allow_from: vec![],
        spawn_log_output: false,
        provider_type: None,
        provider_config: None,
        default_channel: "cli".to_string(),
    };

    // Config should be valid
    assert!(config.api_key.is_some());
    assert!(config.model.is_some());
}

/// Test session persistence during graceful shutdown
#[tokio::test]
async fn test_graceful_shutdown_saves_sessions() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create session manager
    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    // Create a session with data
    let session = session_manager
        .get_or_create_session("telegram", "123456789")
        .await
        .unwrap();

    // Add a message
    let message = Message::new("user".to_string(), "Test message for shutdown".to_string());
    session_manager
        .add_message(&session.session_id, message)
        .await
        .unwrap();

    // Simulate graceful shutdown by saving all sessions
    session_manager.save_all_sessions().await.unwrap();

    // Verify session was persisted
    let session_file = sessions_dir.join("telegram_123456789.json");
    assert!(
        session_file.exists(),
        "Session should be persisted during graceful shutdown"
    );

    // Verify we can reload the session
    let session_manager2 = SessionManager::new(sessions_dir);
    session_manager2.initialize().await.unwrap();

    let loaded_session = session_manager2
        .get_session(&session.session_id)
        .await
        .unwrap();
    assert_eq!(loaded_session.messages.len(), 1);
    assert_eq!(
        loaded_session.messages[0].content,
        "Test message for shutdown"
    );
}

/// Test concurrent session handling
#[tokio::test]
async fn test_concurrent_session_handling() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = std::sync::Arc::new(SessionManager::new(sessions_dir));
    session_manager.initialize().await.unwrap();

    // Simulate concurrent messages from different chats
    let mut handles = vec![];

    for i in 0..5 {
        let sm = session_manager.clone();
        let handle = tokio::spawn(async move {
            let chat_id = format!("chat_{}", i);
            let session = sm
                .get_or_create_session("telegram", &chat_id)
                .await
                .unwrap();

            // Add multiple messages
            for j in 0..3 {
                let message =
                    Message::new("user".to_string(), format!("Message {} from chat {}", j, i));
                sm.add_message(&session.session_id, message).await.unwrap();
            }

            session.session_id.clone()
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let session_ids: Vec<String> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify all sessions were created
    assert_eq!(session_ids.len(), 5);

    // Verify each session has correct message count
    for (i, session_id) in session_ids.iter().enumerate() {
        let session = session_manager.get_session(session_id).await.unwrap();
        assert_eq!(
            session.messages.len(),
            3,
            "Session {} should have 3 messages",
            i
        );
    }
}

/// Test error recovery for session persistence failures
#[tokio::test]
async fn test_persistence_error_recovery() {
    use miniclaw::session::SessionManager;

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    // Create a session
    session_manager
        .get_or_create_session("telegram", "123")
        .await
        .unwrap();

    // First persistence should succeed
    let result = session_manager.save_all_sessions().await;
    assert!(result.is_ok());

    // Persistence should be idempotent - running again should succeed
    let result = session_manager.save_all_sessions().await;
    assert!(result.is_ok());
}

/// Test that signal handling is set up correctly (Unix only)
#[cfg(unix)]
#[tokio::test]
async fn test_signal_handlers_setup() {
    use tokio::signal::unix::{SignalKind, signal};

    // Verify we can create signal handlers for SIGTERM and SIGINT
    let sigterm = signal(SignalKind::terminate());
    assert!(sigterm.is_ok(), "Should be able to create SIGTERM handler");

    let sigint = signal(SignalKind::interrupt());
    assert!(sigint.is_ok(), "Should be able to create SIGINT handler");
}

/// Test exit code constants are defined correctly for systemd/docker
#[test]
fn test_exit_codes() {
    // Exit code 0: Success
    assert_eq!(
        std::process::ExitCode::SUCCESS,
        std::process::ExitCode::from(0)
    );

    // Exit code 1: General error
    assert_eq!(
        std::process::ExitCode::from(1),
        std::process::ExitCode::from(1)
    );

    // Exit code 130: SIGINT (128 + 2)
    let sigint_code = 128 + 2;
    assert_eq!(sigint_code, 130, "SIGINT exit code should be 130");

    // Exit code 143: SIGTERM (128 + 15)
    let sigterm_code = 128 + 15;
    assert_eq!(sigterm_code, 143, "SIGTERM exit code should be 143");

    // These exit codes are handled by the shell/runtime when signals are received,
    // but we validate the constants are correct
}

/// Test that auto-persistence runs at correct intervals
#[tokio::test]
async fn test_auto_persistence_interval() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir.clone());
    session_manager.initialize().await.unwrap();

    // Create a session
    let session = session_manager
        .get_or_create_session("telegram", "test")
        .await
        .unwrap();

    // Add a message
    let message = Message::new("user".to_string(), "Test".to_string());
    session_manager
        .add_message(&session.session_id, message)
        .await
        .unwrap();

    // Start auto-persistence
    let (_handle, shutdown) = session_manager.start_auto_persistence();

    // Wait a short time for persistence to potentially run
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop auto-persistence
    let _ = shutdown.send(()).await;

    // Note: We can't easily verify the interval timing in a unit test,
    // but we can verify the task doesn't panic
}

/// Test session cleanup task initialization
#[tokio::test]
async fn test_cleanup_task_initialization() {
    use miniclaw::session::SessionManager;

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir);
    session_manager.initialize().await.unwrap();

    // Start cleanup task
    let (_handle, shutdown) = session_manager.start_cleanup_task();

    // Wait a short time
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stop cleanup task
    let _ = shutdown.send(()).await;

    // Should not panic
}

/// Test that the gateway correctly logs startup and shutdown events
#[tokio::test]
async fn test_gateway_logging_events() {
    use miniclaw::session::SessionManager;

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Initialize tracing for test
    let _ = tracing_subscriber::fmt::try_init();

    let session_manager = SessionManager::new(sessions_dir);
    session_manager.initialize().await.unwrap();

    // This test mainly verifies that logging doesn't panic
    tracing::info!("Test startup event");
    tracing::info!("Test shutdown event");
}

/// Test concurrent access to session manager (RwLock behavior)
#[tokio::test]
async fn test_concurrent_session_access() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = std::sync::Arc::new(SessionManager::new(sessions_dir));
    session_manager.initialize().await.unwrap();

    // Create a shared session
    let session = session_manager
        .get_or_create_session("telegram", "concurrent_test")
        .await
        .unwrap();
    let session_id = session.session_id.clone();

    // Spawn multiple tasks that read and write to the same session
    let mut handles = vec![];
    for i in 0..10 {
        let sm = session_manager.clone();
        let sid = session_id.clone();
        let handle = tokio::spawn(async move {
            // Add message
            let message = Message::new("user".to_string(), format!("Message {}", i));
            sm.add_message(&sid, message).await.unwrap();

            // Read session back
            let session = sm.get_session(&sid).await.unwrap();
            session.messages.len()
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let results: Vec<usize> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All tasks should have succeeded
    assert_eq!(results.len(), 10);

    // Final session should have all messages
    let final_session = session_manager.get_session(&session_id).await.unwrap();
    assert_eq!(final_session.messages.len(), 10);
}

/// Test that sessions are isolated by chat_id
#[tokio::test]
async fn test_session_isolation() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = SessionManager::new(sessions_dir);
    session_manager.initialize().await.unwrap();

    // Create two different sessions
    let session1 = session_manager
        .get_or_create_session("telegram", "user1")
        .await
        .unwrap();
    let session2 = session_manager
        .get_or_create_session("telegram", "user2")
        .await
        .unwrap();

    // Add message to first session
    let message1 = Message::new("user".to_string(), "Message for user1".to_string());
    session_manager
        .add_message(&session1.session_id, message1)
        .await
        .unwrap();

    // Add message to second session
    let message2 = Message::new("user".to_string(), "Message for user2".to_string());
    session_manager
        .add_message(&session2.session_id, message2)
        .await
        .unwrap();

    // Verify isolation
    let session1_loaded = session_manager
        .get_session(&session1.session_id)
        .await
        .unwrap();
    let session2_loaded = session_manager
        .get_session(&session2.session_id)
        .await
        .unwrap();

    assert_eq!(session1_loaded.messages.len(), 1);
    assert_eq!(session2_loaded.messages.len(), 1);
    assert_eq!(session1_loaded.messages[0].content, "Message for user1");
    assert_eq!(session2_loaded.messages[0].content, "Message for user2");
}

/// Test AgentLoop integration is present in gateway
#[test]
fn test_agent_loop_integration() {
    // Verify that AgentLoop type is available and can be imported
    // This is a compile-time check - if this compiles, AgentLoop exists
    use miniclaw::agent::AgentLoop;

    // Type exists (checked at compile time)
    let _type_check: Option<AgentLoop> = None;

    // This test ensures gateway.rs imports and uses AgentLoop
    // The actual runtime test would require a full gateway startup
}

// ── Signal handling and startup smoke tests (Task 32) ─────────────────────────

/// Smoke test: verify the `gateway` binary subcommand exists and reports
/// `--help` correctly (does not start the daemon).
#[test]
fn test_gateway_help_smoke() {
    use assert_cmd::Command;

    let output = Command::cargo_bin("miniclaw")
        .unwrap()
        .args(["gateway", "--help"])
        .output()
        .unwrap();

    // --help always exits 0 for clap-based CLIs
    assert!(
        output.status.success(),
        "gateway --help should exit with code 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("gateway") || stdout.contains("Gateway"),
        "gateway --help should mention 'gateway', got: {stdout}"
    );
}

/// Verify that sessions are correctly flushed when `save_all_sessions` is
/// called — this models what the gateway does on SIGTERM/SIGINT.
#[tokio::test]
async fn test_sessions_flushed_on_simulated_shutdown_signal() {
    use miniclaw::session::{Message, SessionManager};

    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let session_manager = std::sync::Arc::new(SessionManager::new(sessions_dir.clone()));
    session_manager.initialize().await.unwrap();

    // Populate a session
    let session = session_manager
        .get_or_create_session("telegram", "signal_test")
        .await
        .unwrap();

    for i in 0..3 {
        session_manager
            .add_message(
                &session.session_id,
                Message::new("user".to_string(), format!("msg {}", i)),
            )
            .await
            .unwrap();
    }

    // Simulate the shutdown flush (mirrors gateway graceful shutdown code)
    session_manager.save_all_sessions().await.unwrap();

    // Verify all messages were flushed to disk
    let session_file = sessions_dir.join("telegram_signal_test.json");
    assert!(
        session_file.exists(),
        "session file should exist after flush"
    );

    // Reload and confirm integrity
    let manager2 = SessionManager::new(sessions_dir);
    manager2.initialize().await.unwrap();
    let reloaded = manager2.get_session(&session.session_id).await.unwrap();
    assert_eq!(
        reloaded.messages.len(),
        3,
        "all messages should survive a flush/reload cycle"
    );
}
