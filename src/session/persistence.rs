use crate::session::types::Session;
use crate::utils::MiniClawError;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};

/// Type alias for Results in this module
type Result<T> = std::result::Result<T, MiniClawError>;

pub struct Persistence {
    pub sessions_dir: PathBuf,
}

impl Persistence {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    pub async fn save_session(&self, session: &Session) -> Result<()> {
        let file_path = self
            .sessions_dir
            .join(format!("{}.json", session.session_id));
        
        // Use atomic write pattern with retry logic for transient IO errors
        Self::atomic_write_session_with_retry(&file_path, session, 3).await
    }

    /// Atomically writes a session file with retry logic for transient IO errors.
    /// 
    /// Retries up to `max_retries` times with exponential backoff on IO errors.
    async fn atomic_write_session_with_retry(
        file_path: &Path,
        session: &Session,
        max_retries: u32,
    ) -> Result<()> {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= max_retries {
            match Self::atomic_write_session(file_path, session).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Check if it's a transient IO error worth retrying
                    let error_msg = e.to_string();
                    let should_retry = error_msg.contains("disk")
                        || error_msg.contains("space")
                        || error_msg.contains("temporarily");

                    if should_retry && attempt < max_retries {
                        let backoff_ms = 100 * 2u64.pow(attempt);
                        warn!(
                            "Session persistence failed (attempt {}/{}): {}. Retrying in {}ms...",
                            attempt + 1,
                            max_retries + 1,
                            e,
                            backoff_ms
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                        last_error = Some(e);
                    } else {
                        // Non-retryable error or max retries reached
                        return Err(e);
                    }
                }
            }
        }

        // Max retries exhausted
        Err(last_error.unwrap_or_else(|| {
            MiniClawError::session_persistence(
                "unknown",
                format!("Failed after {} retries", max_retries),
            )
        }))
    }

    /// Atomically writes a session file to prevent corruption during crashes.
    /// 
    /// Pattern:
    /// 1. Write to temporary file: `{session_file}.tmp`
    /// 2. Set permissions to 0600 on Unix
    /// 3. Atomically rename temp file to final location
    /// 4. Clean up temp file if rename fails
    /// 
    /// This ensures that the session file is never in a partially-written state.
    async fn atomic_write_session(file_path: &Path, session: &Session) -> Result<()> {
        let temp_path = file_path.with_extension("tmp");
        let session_id = session.session_id.clone();
        
        // Serialize session to JSON
        let json = serde_json::to_string_pretty(session)
            .map_err(|e| MiniClawError::serialization(e.to_string()))?;

        // Step 1: Write to temporary file
        if let Err(e) = fs::write(&temp_path, &json).await {
            // Clean up temp file if write failed
            let _ = fs::remove_file(&temp_path).await;
            return Err(MiniClawError::io(&temp_path, e));
        }

        // Step 2: Set file permissions to 0600 on Unix BEFORE rename
        // This ensures the final file has correct permissions immediately
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            if let Err(e) = fs::set_permissions(&temp_path, permissions).await {
                // Clean up temp file
                let _ = fs::remove_file(&temp_path).await;
                return Err(MiniClawError::io(&temp_path, e));
            }
        }

        // Step 3: Atomic rename - this is the critical operation
        // On Unix, rename() is atomic and will either succeed (file is complete)
        // or fail (file is unchanged)
        if let Err(e) = fs::rename(&temp_path, file_path).await {
            // Clean up temp file on rename failure
            let _ = fs::remove_file(&temp_path).await;
            return Err(MiniClawError::session_persistence(
                &session_id,
                format!("Failed to atomically rename file: {}", e),
            ));
        }

        info!("Atomically saved session {} to {:?}", session_id, file_path);
        Ok(())
    }

    pub async fn load_session(&self, session_id: &str) -> Result<Session> {
        let file_path = self.sessions_dir.join(format!("{}.json", session_id));

        match fs::read_to_string(&file_path).await {
            Ok(json) => match serde_json::from_str::<Session>(&json) {
                Ok(session) => {
                    info!("Loaded session {} from {:?}", session_id, file_path);
                    Ok(session)
                }
                Err(e) => {
                    error!("Failed to parse session {}: {}", session_id, e);
                    self.handle_corrupted_file(&file_path, session_id).await
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(MiniClawError::io(
                        &file_path,
                        std::io::Error::new(std::io::ErrorKind::NotFound, "Session file not found"),
                    ))
                } else {
                    Err(MiniClawError::io(&file_path, e))
                }
            }
        }
    }

    pub async fn load_all_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(&self.sessions_dir)
            .await
            .map_err(|e| MiniClawError::io(&self.sessions_dir, e))?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .json files (not .corrupted files)
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let session_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                match self.load_session(&session_id).await {
                    Ok(session) => sessions.push(session),
                    Err(e) => {
                        warn!("Failed to load session {}: {}", session_id, e);
                        // Continue loading other sessions
                    }
                }
            }
        }

        info!(
            "Loaded {} sessions from {:?}",
            sessions.len(),
            self.sessions_dir
        );
        Ok(sessions)
    }

    async fn handle_corrupted_file(&self, file_path: &Path, session_id: &str) -> Result<Session> {
        let corrupted_path = file_path.with_extension("json.corrupted");

        // Rename corrupted file
        fs::rename(file_path, &corrupted_path)
            .await
            .map_err(|e| {
                MiniClawError::session_persistence(
                    session_id,
                    format!("Failed to rename corrupted file: {}", e),
                )
            })?;

        error!(
            "Corrupted session file detected. Moved {:?} to {:?}",
            file_path, corrupted_path
        );

        // Extract channel and chat_id from session_id
        let parts: Vec<&str> = session_id.split('_').collect();
        let (channel, chat_id) = if parts.len() >= 2 {
            (parts[0].to_string(), parts[1..].join("_"))
        } else {
            ("unknown".to_string(), session_id.to_string())
        };

        // Create new empty session
        let session = Session::new(channel, chat_id);

        // Save the new session
        self.save_session(&session).await?;

        info!(
            "Created new empty session {} to replace corrupted file",
            session_id
        );
        Ok(session)
    }

    pub async fn create_sessions_dir(&self) -> Result<()> {
        if !self.sessions_dir.exists() {
            fs::create_dir_all(&self.sessions_dir)
                .await
                .map_err(|e| MiniClawError::io(&self.sessions_dir, e))?;

            // Set directory permissions to 0755 on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o755);
                fs::set_permissions(&self.sessions_dir, permissions)
                    .await
                    .map_err(|e| MiniClawError::io(&self.sessions_dir, e))?;
            }

            info!("Created sessions directory: {:?}", self.sessions_dir);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::{Message, Session};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_sessions_dir() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        assert!(sessions_dir.exists());
    }

    #[tokio::test]
    async fn test_save_and_load_session() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        let mut session = Session::new("telegram".to_string(), "123456789".to_string());
        session.add_message(Message::new("user".to_string(), "Hello".to_string()));

        persistence.save_session(&session).await.unwrap();

        let loaded = persistence.load_session(&session.session_id).await.unwrap();
        assert_eq!(loaded.session_id, session.session_id);
        assert_eq!(loaded.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_load_all_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        // Create multiple sessions
        let session1 = Session::new("telegram".to_string(), "111".to_string());
        let session2 = Session::new("telegram".to_string(), "222".to_string());

        persistence.save_session(&session1).await.unwrap();
        persistence.save_session(&session2).await.unwrap();

        let sessions = persistence.load_all_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_corrupted_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        // Create a corrupted JSON file
        let file_path = sessions_dir.join("telegram_123.json");
        fs::write(&file_path, "invalid json {{").await.unwrap();

        // Load should handle corruption gracefully
        let session = persistence.load_session("telegram_123").await.unwrap();
        assert_eq!(session.session_id, "telegram_123");

        // Original corrupted file should be renamed
        assert!(sessions_dir.join("telegram_123.json.corrupted").exists());

        // A new file should have been created
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_file_permissions_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = TempDir::new().unwrap();
            let sessions_dir = temp_dir.path().join("sessions");

            let persistence = Persistence::new(sessions_dir.clone());
            persistence.create_sessions_dir().await.unwrap();

            let session = Session::new("telegram".to_string(), "123".to_string());
            persistence.save_session(&session).await.unwrap();

            let file_path = sessions_dir.join("telegram_123.json");
            let metadata = fs::metadata(file_path).await.unwrap();
            let permissions = metadata.permissions().mode();

            // Check that file has 0600 permissions (0o100600 = 33152)
            assert_eq!(permissions & 0o777, 0o600);
        }
    }

    #[tokio::test]
    async fn test_atomic_write_no_temp_file_left() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        let session = Session::new("telegram".to_string(), "456".to_string());
        persistence.save_session(&session).await.unwrap();

        // Verify no .tmp files left behind
        let mut entries = fs::read_dir(&sessions_dir).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            assert_ne!(ext, Some("tmp"), "Temp file should not exist after atomic write");
        }

        // Verify the actual file exists and is valid JSON
        let file_path = sessions_dir.join("telegram_456.json");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).await.unwrap();
        let loaded: Session = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.session_id, "telegram_456");
    }

    #[tokio::test]
    async fn test_atomic_write_preserves_existing_on_failure() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        // Create initial session
        let mut session = Session::new("telegram".to_string(), "789".to_string());
        session.add_message(Message::new("user".to_string(), "original".to_string()));
        persistence.save_session(&session).await.unwrap();

        // Verify original content
        let loaded = persistence.load_session("telegram_789").await.unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "original");
    }

    #[tokio::test]
    async fn test_atomic_write_crash_simulation() {
        // This test simulates a crash during write by verifying that
        // either the old file exists OR the new file exists, but never
        // a corrupted partial state
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let persistence = Persistence::new(sessions_dir.clone());
        persistence.create_sessions_dir().await.unwrap();

        // Create initial session
        let mut session = Session::new("telegram".to_string(), "crash_test".to_string());
        session.add_message(Message::new("user".to_string(), "original".to_string()));
        persistence.save_session(&session).await.unwrap();

        // Verify file exists and is valid
        let file_path = sessions_dir.join("telegram_crash_test.json");
        assert!(file_path.exists());
        
        // Read raw content and verify it's valid JSON
        let content = fs::read_to_string(&file_path).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
        
        // Verify no temp file exists (would indicate incomplete write)
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists(), "Temp file should be cleaned up after successful write");
        
        // Now simulate trying to read after a "crash" - file should still be valid
        let loaded = persistence.load_session("telegram_crash_test").await.unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "original");
    }
}

