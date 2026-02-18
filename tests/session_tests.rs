use miniclaw::session::{Message, SessionManager};
use tempfile::TempDir;

#[tokio::test]
async fn test_session_manager_integration() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().to_path_buf();

    let manager = SessionManager::new(sessions_dir.clone());
    manager.initialize().await.unwrap();

    // Create session
    let session = manager
        .get_or_create_session("telegram", "user123")
        .await
        .unwrap();
    assert_eq!(session.session_id, "telegram_user123");

    // Add message
    let message = Message::new("user".to_string(), "Hello integration test".to_string());
    manager
        .add_message(&session.session_id, message)
        .await
        .unwrap();

    // Verify in memory
    let session = manager.get_session("telegram_user123").await.unwrap();
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "Hello integration test");

    // Persist
    manager.save_all_sessions().await.unwrap();

    // Verify on disk (manual check via new manager)
    let manager2 = SessionManager::new(sessions_dir);
    manager2.initialize().await.unwrap();
    let session2 = manager2.get_session("telegram_user123").await.unwrap();
    assert_eq!(session2.messages.len(), 1);
    assert_eq!(session2.messages[0].content, "Hello integration test");
}

#[tokio::test]
async fn test_corrupted_session_recovery_integration() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(&sessions_dir).unwrap();

    // Create a corrupted file
    let file_path = sessions_dir.join("telegram_bad.json");
    std::fs::write(&file_path, "not valid json").unwrap();

    let manager = SessionManager::new(sessions_dir.clone());
    manager.initialize().await.unwrap();

    // Try to get the corrupted session
    let session = manager
        .get_or_create_session("telegram", "bad")
        .await
        .unwrap();
    assert_eq!(session.session_id, "telegram_bad");
    assert!(session.messages.is_empty());

    // Verify corrupted file was moved
    assert!(sessions_dir.join("telegram_bad.json.corrupted").exists());
}
