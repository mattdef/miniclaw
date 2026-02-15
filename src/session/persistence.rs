use crate::session::types::Session;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};

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
        let json =
            serde_json::to_string_pretty(session).context("Failed to serialize session to JSON")?;

        fs::write(&file_path, json)
            .await
            .with_context(|| format!("Failed to write session file: {:?}", file_path))?;

        // Set file permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&file_path, permissions)
                .await
                .with_context(|| format!("Failed to set permissions on: {:?}", file_path))?;
        }

        info!("Saved session {} to {:?}", session.session_id, file_path);
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
                    anyhow::bail!("Session file not found: {:?}", file_path)
                }
                Err(e.into())
            }
        }
    }

    pub async fn load_all_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(&self.sessions_dir).await.with_context(|| {
            format!("Failed to read sessions directory: {:?}", self.sessions_dir)
        })?;

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
            .with_context(|| format!("Failed to rename corrupted file: {:?}", file_path))?;

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
                .with_context(|| {
                    format!(
                        "Failed to create sessions directory: {:?}",
                        self.sessions_dir
                    )
                })?;

            // Set directory permissions to 0755 on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o755);
                fs::set_permissions(&self.sessions_dir, permissions)
                    .await
                    .with_context(|| {
                        format!("Failed to set permissions on: {:?}", self.sessions_dir)
                    })?;
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
}
