use crate::session::persistence::Persistence;
use crate::session::types::{Message, Session};
use crate::utils::MiniClawError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, warn};

/// Type alias for Results in this module
type Result<T> = std::result::Result<T, MiniClawError>;

pub const PERSISTENCE_INTERVAL_SECS: u64 = 30;

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    persistence: Arc<Persistence>,
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        let persistence = Arc::new(Persistence::new(sessions_dir));
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        // Create sessions directory
        self.persistence.create_sessions_dir().await?;

        // Load existing sessions
        let sessions = self.persistence.load_all_sessions().await?;
        let mut guard = self.sessions.write().await;
        for session in sessions {
            guard.insert(session.session_id.clone(), session);
        }
        drop(guard);

        info!(
            "SessionManager initialized with {} sessions",
            self.sessions.read().await.len()
        );
        Ok(())
    }

    /// Gets an existing session or creates a new one.
    ///
    /// **Lock Scope Pattern**: Acquires read lock briefly in scoped block,
    /// clones session data, then releases lock immediately via scope end.
    /// Never holds lock during I/O operations (disk load).
    pub async fn get_or_create_session(&self, channel: &str, chat_id: &str) -> Result<Session> {
        let session_id = format!("{}_{}", channel, chat_id);

        // Check if session exists in memory (brief read lock)
        {
            let guard = self.sessions.read().await;
            if let Some(session) = guard.get(&session_id) {
                return Ok(session.clone()); // Clone then drop guard
            }
            // Lock released here via scope end
        }

        // Try to load from disk
        match self.persistence.load_session(&session_id).await {
            Ok(session) => {
                let mut guard = self.sessions.write().await;
                guard.insert(session_id.clone(), session.clone());
                Ok(session)
            }
            Err(_) => {
                // Create new session
                let session = Session::new(channel.to_string(), chat_id.to_string());
                let mut guard = self.sessions.write().await;
                guard.insert(session_id, session.clone());
                Ok(session)
            }
        }
    }

    /// Adds a message to an existing session.
    ///
    /// **Lock Scope Pattern**: Acquires write lock briefly, updates messages
    /// vector and last_accessed timestamp, then releases lock immediately.
    /// Lock is held only for HashMap access and message append.
    pub async fn add_message(&self, session_id: &str, message: Message) -> Result<()> {
        let mut guard = self.sessions.write().await;

        if let Some(session) = guard.get_mut(session_id) {
            session.add_message(message); // Update then drop guard
            Ok(())
        } else {
            Err(MiniClawError::session_persistence(
                session_id,
                "Session not found",
            ))
        }
        // Lock released here via function end
    }

    /// Retrieves a session by ID.
    ///
    /// **Lock Scope Pattern**: Acquires read lock, clones session data,
    /// releases lock immediately. Returns cloned data to avoid holding lock.
    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let guard = self.sessions.read().await;
        guard.get(session_id).cloned() // Clone then drop guard
    }

    /// Saves all sessions to disk.
    ///
    /// **Lock Scope Pattern**: Acquires read lock, clones all sessions,
    /// explicitly drops lock, THEN performs I/O. Never holds lock during
    /// disk operations to prevent blocking other threads.
    pub async fn save_all_sessions(&self) -> Result<()> {
        let guard = self.sessions.read().await;
        let sessions: Vec<Session> = guard.values().cloned().collect();
        drop(guard); // Explicit drop before I/O operations

        for session in sessions {
            if let Err(e) = self.persistence.save_session(&session).await {
                error!(
                    session_id = %session.session_id,
                    error = %e,
                    "Failed to save session"
                );
            }
        }

        info!("Saved all sessions to disk");
        Ok(())
    }

    /// Starts a background task that auto-saves sessions every 30 seconds.
    ///
    /// **Lock Scope Pattern**: In the spawned task, acquires read lock,
    /// clones all sessions, drops lock, THEN performs I/O. This prevents
    /// blocking the main thread during periodic saves.
    ///
    /// Returns a JoinHandle for graceful shutdown coordination and a shutdown sender
    /// to signal the task to stop.
    pub fn start_auto_persistence(
        &self,
    ) -> (tokio::task::JoinHandle<()>, tokio::sync::mpsc::Sender<()>) {
        let sessions = Arc::clone(&self.sessions);
        let persistence = Arc::clone(&self.persistence);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(PERSISTENCE_INTERVAL_SECS));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Brief read lock, clone, release before I/O
                        let guard = sessions.read().await;
                        let sessions_vec: Vec<Session> = guard.values().cloned().collect();
                        drop(guard);

                        debug!(session_count = sessions_vec.len(), "Auto-persistence cycle starting");

                        let mut failed_count = 0;
                        for session in sessions_vec {
                            if let Err(e) = persistence.save_session(&session).await {
                                error!(
                                    session_id = %session.session_id,
                                    error = %e,
                                    "Auto-persistence failed for session"
                                );
                                failed_count += 1;
                            }
                        }

                        if failed_count > 0 {
                            warn!(failed_count = failed_count, "Some sessions failed to auto-persist");
                        }

                        info!("Auto-persistence cycle completed");
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Persistence task received shutdown signal, completing...");
                        break;
                    }
                }
            }
        });

        (handle, shutdown_tx)
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Updates (or inserts) a complete session in the manager
    pub async fn update_session(&self, session: Session) -> Result<()> {
        let mut guard = self.sessions.write().await;
        guard.insert(session.session_id.clone(), session);
        Ok(())
    }

    /// Saves a specific session to disk immediately
    pub async fn persist_session(&self, session: &Session) -> Result<()> {
        self.persistence.save_session(session).await
    }

    /// Starts the session cleanup background task
    ///
    /// This spawns a background task that runs daily to clean up expired sessions.
    /// Sessions are considered expired if they haven't been accessed for 30 days.
    ///
    /// Returns a JoinHandle for graceful shutdown coordination and a shutdown sender
    /// to signal the task to stop.
    pub fn start_cleanup_task(
        &self,
    ) -> (tokio::task::JoinHandle<()>, tokio::sync::mpsc::Sender<()>) {
        use crate::session::cleanup::Cleanup;
        let cleanup = Cleanup::new(self.persistence.sessions_dir.clone());
        cleanup.start_cleanup_task()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_manager_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());

        manager.initialize().await.unwrap();

        // Directory should exist
        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_get_or_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();
        assert_eq!(session.session_id, "telegram_123");
        assert_eq!(session.channel, "telegram");
        assert_eq!(session.chat_id, "123");
    }

    #[tokio::test]
    async fn test_add_message() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        let session = manager
            .get_or_create_session("telegram", "123")
            .await
            .unwrap();
        let message = Message::new("user".to_string(), "Hello".to_string());

        manager
            .add_message(&session.session_id, message)
            .await
            .unwrap();

        let updated = manager.get_session(&session.session_id).await.unwrap();
        assert_eq!(updated.messages.len(), 1);
        assert_eq!(updated.messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_persistence_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        // Create manager and add session
        {
            let manager = SessionManager::new(temp_dir.path().to_path_buf());
            manager.initialize().await.unwrap();

            let session = manager
                .get_or_create_session("telegram", "123")
                .await
                .unwrap();
            let message = Message::new("user".to_string(), "Test".to_string());
            manager
                .add_message(&session.session_id, message)
                .await
                .unwrap();

            manager.save_all_sessions().await.unwrap();
        }

        // Create new manager and verify session loads
        {
            let manager = SessionManager::new(temp_dir.path().to_path_buf());
            manager.initialize().await.unwrap();

            let session = manager
                .get_or_create_session("telegram", "123")
                .await
                .unwrap();
            assert_eq!(session.messages.len(), 1);
            assert_eq!(session.messages[0].content, "Test");
        }
    }

    #[tokio::test]
    async fn test_session_count() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        assert_eq!(manager.session_count().await, 0);

        manager
            .get_or_create_session("telegram", "111")
            .await
            .unwrap();
        manager
            .get_or_create_session("telegram", "222")
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 2);
    }
}
