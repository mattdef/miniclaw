//! Session cleanup module for TTL-based session expiration
//!
//! This module provides automatic cleanup of expired sessions based on a TTL
//! (Time To Live) policy. Sessions that haven't been accessed for 30 days
//! are automatically removed to prevent disk space from growing indefinitely.

use crate::session::types::Session;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info};

/// TTL for session expiration (30 days)
pub const SESSION_TTL_DAYS: i64 = 30;

/// Cleanup interval (24 hours)
pub const CLEANUP_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// Manages session cleanup based on TTL policy
pub struct Cleanup {
    sessions_dir: PathBuf,
}

impl Cleanup {
    /// Creates a new Cleanup instance
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Checks if a session is expired based on last_accessed timestamp
    ///
    /// # Arguments
    /// * `last_accessed` - The timestamp of last session access
    ///
    /// # Returns
    /// * `true` if the session is expired (older than SESSION_TTL_DAYS)
    /// * `false` otherwise
    pub fn is_expired(last_accessed: DateTime<Utc>) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(last_accessed);
        age > Duration::days(SESSION_TTL_DAYS)
    }

    /// Gets the file size in bytes
    async fn get_file_size(&self, path: &Path) -> u64 {
        match fs::metadata(path).await {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                debug!("Failed to get file size for {:?}: {}", path, e);
                0
            }
        }
    }

    /// Scans all session files and returns expired ones
    ///
    /// # Returns
    /// A tuple containing:
    /// - Vector of (session_id, file_path, last_accessed, file_size) for expired sessions
    /// - Total count of session files scanned
    pub async fn scan_expired_sessions(&self) -> Result<(Vec<(String, PathBuf, DateTime<Utc>, u64)>, usize)> {
        let mut expired = Vec::new();
        let mut total_count = 0;

        let mut entries = fs::read_dir(&self.sessions_dir).await.with_context(|| {
            format!(
                "Failed to read sessions directory: {:?}",
                self.sessions_dir
            )
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .json files (not .corrupted files)
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            total_count += 1;

            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Read and parse session file to get last_accessed
            match fs::read_to_string(&path).await {
                Ok(json) => {
                    match serde_json::from_str::<Session>(&json) {
                        Ok(session) => {
                            if Self::is_expired(session.last_accessed) {
                                let file_size = self.get_file_size(&path).await;
                                expired.push((session_id, path, session.last_accessed, file_size));
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse session {}: {}", session_id, e);
                            // Skip corrupted files - they'll be handled by persistence
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read session file {:?}: {}", path, e);
                }
            }
        }

        Ok((expired, total_count))
    }

    /// Deletes a session file and returns the bytes freed
    ///
    /// Re-verifies expiration before deletion to avoid TOCTOU race conditions
    /// where a session might be updated between scan and delete.
    ///
    /// # Arguments
    /// * `path` - Path to the session file
    /// * `session_id` - Session ID for logging
    /// * `file_size` - Pre-calculated file size from scan
    ///
    /// # Returns
    /// * `Ok(bytes_freed)` on success (0 if session was preserved)
    /// * `Err` on failure
    async fn delete_session(&self, path: &Path, session_id: &str, file_size: u64) -> Result<u64> {
        // Re-read session file to verify it's still expired (TOCTOU protection)
        match fs::read_to_string(path).await {
            Ok(json) => {
                match serde_json::from_str::<Session>(&json) {
                    Ok(session) => {
                        if !Self::is_expired(session.last_accessed) {
                            // Session was updated since scan - preserve it!
                            debug!(
                                session_id = %session_id,
                                last_accessed = %session.last_accessed,
                                "Session was updated since scan, preserving"
                            );
                            return Ok(0); // Return 0 bytes freed
                        }
                        
                        // Still expired - safe to delete
                        fs::remove_file(path)
                            .await
                            .with_context(|| format!("Failed to delete session file: {:?}", path))?;

                        let age_days = Utc::now().signed_duration_since(session.last_accessed).num_days();
                        debug!(
                            session_id = %session_id,
                            last_accessed = %session.last_accessed,
                            age_days = age_days,
                            "Deleted expired session"
                        );

                        Ok(file_size)
                    }
                    Err(e) => {
                        // Corrupted file - skip it
                        debug!("Skipping corrupted session file {:?}: {}", path, e);
                        Ok(0)
                    }
                }
            }
            Err(e) => {
                // File might have been deleted already (e.g., by another process)
                debug!("Session file {:?} no longer exists or is unreadable: {}", path, e);
                Ok(0)
            }
        }
    }

    /// Runs the cleanup process
    ///
    /// Scans all sessions, deletes expired ones, and logs the results
    pub async fn run(&self) -> Result<CleanupResult> {
        info!("Starting session cleanup scan");

        let (expired_sessions, sessions_scanned) = self.scan_expired_sessions().await?;
        let mut sessions_deleted = 0;
        let mut bytes_freed: u64 = 0;

        for (session_id, path, _last_accessed, file_size) in expired_sessions {
            match self.delete_session(&path, &session_id, file_size).await {
                Ok(bytes) => {
                    if bytes > 0 {
                        bytes_freed += bytes;
                        sessions_deleted += 1;
                    }
                }
                Err(e) => {
                    error!("Failed to delete session {}: {}", session_id, e);
                    // Continue with other sessions
                }
            }
        }

        info!(
            sessions_scanned = sessions_scanned,
            sessions_deleted = sessions_deleted,
            bytes_freed = bytes_freed,
            "Session cleanup complete"
        );

        Ok(CleanupResult {
            sessions_scanned,
            sessions_deleted,
            bytes_freed,
        })
    }

    /// Starts a background cleanup task that runs daily
    ///
    /// Returns a JoinHandle for graceful shutdown coordination and a shutdown sender
    pub fn start_cleanup_task(
        &self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::mpsc::Sender<()>,
    ) {
        let sessions_dir = self.sessions_dir.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let handle = tokio::spawn(async move {
            let cleanup = Cleanup::new(sessions_dir);
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(CLEANUP_INTERVAL_SECS));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = cleanup.run().await {
                            error!("Session cleanup failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Cleanup task received shutdown signal, completing...");
                        // Complete current cleanup cycle if running
                        break;
                    }
                }
            }
        });

        (handle, shutdown_tx)
    }
}

/// Result of a cleanup operation
#[derive(Debug, Clone, Copy)]
pub struct CleanupResult {
    /// Total number of session files scanned
    pub sessions_scanned: usize,
    /// Number of expired sessions deleted
    pub sessions_deleted: usize,
    /// Total bytes freed from deleted sessions
    pub bytes_freed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::{Message, Session};
    use tempfile::TempDir;

    #[test]
    fn test_is_expired_true() {
        // Session accessed 31 days ago should be expired
        let last_accessed = Utc::now() - Duration::days(31);
        assert!(Cleanup::is_expired(last_accessed));
    }

    #[test]
    fn test_is_expired_false_recent() {
        // Session accessed 1 day ago should NOT be expired
        let last_accessed = Utc::now() - Duration::days(1);
        assert!(!Cleanup::is_expired(last_accessed));
    }

    #[test]
    fn test_is_expired_false_exactly_30_days() {
        // Session accessed exactly 30 days ago should NOT be expired
        // (we use > not >= for the comparison)
        // Use 30 days minus 1 second to avoid timing issues in tests
        let last_accessed = Utc::now() - Duration::days(30) + Duration::seconds(1);
        assert!(!Cleanup::is_expired(last_accessed));
    }

    #[test]
    fn test_is_expired_false_29_days() {
        // Session accessed 29 days ago should NOT be expired
        let last_accessed = Utc::now() - Duration::days(29);
        assert!(!Cleanup::is_expired(last_accessed));
    }

    #[test]
    fn test_is_expired_true_30_days_plus_1_second() {
        // Session accessed 30 days and 1 second ago should be expired
        let last_accessed = Utc::now() - Duration::days(30) - Duration::seconds(1);
        assert!(Cleanup::is_expired(last_accessed));
    }

    #[tokio::test]
    async fn test_scan_expired_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create an expired session (31 days old)
        let mut expired_session = Session::new("telegram".to_string(), "111".to_string());
        expired_session.last_accessed = Utc::now() - Duration::days(31);
        let expired_json = serde_json::to_string(&expired_session).unwrap();
        fs::write(sessions_dir.join("telegram_111.json"), expired_json)
            .await
            .unwrap();

        // Create a recent session (1 day old)
        let mut recent_session = Session::new("telegram".to_string(), "222".to_string());
        recent_session.last_accessed = Utc::now() - Duration::days(1);
        let recent_json = serde_json::to_string(&recent_session).unwrap();
        fs::write(sessions_dir.join("telegram_222.json"), recent_json)
            .await
            .unwrap();

        // Scan for expired sessions
        let (expired, total_count) = cleanup.scan_expired_sessions().await.unwrap();

        // Should find only the expired session
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, "telegram_111");
        assert_eq!(total_count, 2); // Total sessions scanned
    }

    #[tokio::test]
    async fn test_scan_expired_sessions_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir);

        let (expired, total_count) = cleanup.scan_expired_sessions().await.unwrap();
        assert!(expired.is_empty());
        assert_eq!(total_count, 0);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create an expired session file
        let mut session = Session::new("telegram".to_string(), "123".to_string());
        session.last_accessed = Utc::now() - Duration::days(31); // Make it expired
        let json = serde_json::to_string(&session).unwrap();
        let path = sessions_dir.join("telegram_123.json");
        fs::write(&path, &json).await.unwrap();

        let file_size = fs::metadata(&path).await.unwrap().len();

        // Delete the session
        let bytes_freed = cleanup
            .delete_session(&path, "telegram_123", file_size)
            .await
            .unwrap();

        assert_eq!(bytes_freed, file_size);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_run_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create an expired session
        let mut expired_session = Session::new("telegram".to_string(), "old".to_string());
        expired_session.last_accessed = Utc::now() - Duration::days(31);
        let expired_json = serde_json::to_string(&expired_session).unwrap();
        fs::write(sessions_dir.join("telegram_old.json"), expired_json)
            .await
            .unwrap();

        // Create a recent session
        let mut recent_session = Session::new("telegram".to_string(), "new".to_string());
        recent_session.last_accessed = Utc::now() - Duration::days(1);
        let recent_json = serde_json::to_string(&recent_session).unwrap();
        fs::write(sessions_dir.join("telegram_new.json"), recent_json)
            .await
            .unwrap();

        // Run cleanup
        let result = cleanup.run().await.unwrap();

        assert_eq!(result.sessions_scanned, 2);
        assert_eq!(result.sessions_deleted, 1);
        assert!(result.bytes_freed > 0);

        // Verify old session was deleted
        assert!(!sessions_dir.join("telegram_old.json").exists());
        // Verify new session was preserved
        assert!(sessions_dir.join("telegram_new.json").exists());
    }

    #[tokio::test]
    async fn test_run_cleanup_no_expired() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create only recent sessions
        for i in 0..5 {
            let mut session = Session::new("telegram".to_string(), format!("{}", i));
            session.last_accessed = Utc::now() - Duration::days(1);
            let json = serde_json::to_string(&session).unwrap();
            fs::write(sessions_dir.join(format!("telegram_{}.json", i)), json)
                .await
                .unwrap();
        }

        // Run cleanup
        let result = cleanup.run().await.unwrap();

        assert_eq!(result.sessions_scanned, 5);
        assert_eq!(result.sessions_deleted, 0);
        assert_eq!(result.bytes_freed, 0);
    }

    #[tokio::test]
    async fn test_cleanup_skips_corrupted_files() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create a corrupted file
        fs::write(sessions_dir.join("corrupted.json"), "invalid json")
            .await
            .unwrap();

        // Create a valid expired session
        let mut expired_session = Session::new("telegram".to_string(), "expired".to_string());
        expired_session.last_accessed = Utc::now() - Duration::days(31);
        let expired_json = serde_json::to_string(&expired_session).unwrap();
        fs::write(sessions_dir.join("telegram_expired.json"), expired_json)
            .await
            .unwrap();

        // Run cleanup
        let result = cleanup.run().await.unwrap();

        // Should only delete the valid expired session
        assert_eq!(result.sessions_deleted, 1);
        // Corrupted file should still exist (not be processed)
        assert!(sessions_dir.join("corrupted.json").exists());
    }

    #[tokio::test]
    async fn test_start_cleanup_task() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir);

        // Start cleanup task
        let (handle, shutdown) = cleanup.start_cleanup_task();

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Signal shutdown
        let _ = shutdown.send(()).await;

        // Wait for task to complete with timeout
        let timeout = tokio::time::Duration::from_secs(5);
        let result = tokio::time::timeout(timeout, handle).await;

        assert!(result.is_ok(), "Cleanup task should complete within timeout");
    }

    #[tokio::test]
    async fn test_ttl_reset_on_session_access() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        // Create a session with an old last_accessed
        let mut session = Session::new("telegram".to_string(), "123".to_string());
        let old_timestamp = Utc::now() - Duration::days(25);
        session.last_accessed = old_timestamp;
        
        let json = serde_json::to_string(&session).unwrap();
        fs::write(sessions_dir.join("telegram_123.json"), json)
            .await
            .unwrap();

        // Simulate session access by adding a message (which updates last_accessed)
        session.add_message(Message::new("user".to_string(), "Hello".to_string()));
        
        // Verify last_accessed was updated
        assert!(session.last_accessed > old_timestamp);
        assert!(!Cleanup::is_expired(session.last_accessed));
    }

    #[tokio::test]
    async fn test_delete_session_toctou_protection() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).await.unwrap();

        let cleanup = Cleanup::new(sessions_dir.clone());

        // Create an expired session file
        let mut session = Session::new("telegram".to_string(), "999".to_string());
        session.last_accessed = Utc::now() - Duration::days(31);
        let json = serde_json::to_string(&session).unwrap();
        let path = sessions_dir.join("telegram_999.json");
        fs::write(&path, &json).await.unwrap();

        let file_size = fs::metadata(&path).await.unwrap().len();

        // Simulate session being updated between scan and delete
        // (e.g., user sent a message and persistence saved it)
        session.add_message(Message::new("user".to_string(), "Hello".to_string()));
        let updated_json = serde_json::to_string(&session).unwrap();
        fs::write(&path, updated_json).await.unwrap();

        // Try to delete - should return 0 bytes (not deleted due to update)
        let bytes_freed = cleanup
            .delete_session(&path, "telegram_999", file_size)
            .await
            .unwrap();

        assert_eq!(bytes_freed, 0); // Should not delete
        assert!(path.exists()); // File should still exist
    }
}
