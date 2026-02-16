//! Memory system for the agent
//!
//! This module provides memory management capabilities including:
//! - Short-term memory (in-memory storage)
//! - Long-term memory (persistent storage in MEMORY.md)
//! - Daily notes (YYYY-MM-DD.md files)
//! - Memory ranking and retrieval

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

pub mod long_term;
pub mod daily_notes;
pub mod types;

use types::{MemoryEntry, MemoryError};

/// Maximum number of entries to keep in short-term memory
const MAX_SHORT_TERM_ENTRIES: usize = 100;

/// MemoryStore manages both short-term and long-term memory
///
/// Uses Arc<RwLock<VecDeque>> for thread-safe concurrent access to memory entries.
/// The store can be cloned to share the same memory storage across tasks.
#[derive(Debug, Clone)]
pub struct MemoryStore {
    /// Short-term memory: VecDeque of recent memory entries
    entries: Arc<RwLock<VecDeque<MemoryEntry>>>,
    /// Workspace path for file operations
    workspace_path: PathBuf,
}

impl MemoryStore {
    /// Creates a new MemoryStore with empty memory
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory for file operations
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            workspace_path,
        }
    }

    /// Appends content to long-term memory (MEMORY.md)
    ///
    /// # Arguments
    /// * `content` - The content to store
    ///
    /// # Returns
    /// * `Ok(String)` - Path to the memory file
    /// * `Err(MemoryError)` - If storage fails
    pub async fn append_to_memory(&self, content: String) -> Result<String, MemoryError> {
        let entries = self.entries.clone();
        let (file_path, entry) = long_term::append_to_memory(
            &self.workspace_path,
            content,
            None::<fn(MemoryEntry)>, // Don't use callback
        )
        .await?;
        
        // Add to short-term memory directly
        let mut entries_guard = entries.write().await;
        entries_guard.push_back(entry);
        if entries_guard.len() > MAX_SHORT_TERM_ENTRIES {
            entries_guard.pop_front();
        }
        drop(entries_guard);
        
        Ok(file_path)
    }

    /// Creates a daily note
    ///
    /// # Arguments
    /// * `content` - The content to store
    ///
    /// # Returns
    /// * `Ok(String)` - Path to the daily note file
    /// * `Err(MemoryError)` - If creation fails
    pub async fn create_daily_note(&self, content: String) -> Result<String, MemoryError> {
        let entries = self.entries.clone();
        let (file_path, entry) = daily_notes::create_daily_note(
            &self.workspace_path,
            content,
            None::<fn(MemoryEntry)>, // Don't use callback
        )
        .await?;
        
        // Add to short-term memory directly
        let mut entries_guard = entries.write().await;
        entries_guard.push_back(entry);
        if entries_guard.len() > MAX_SHORT_TERM_ENTRIES {
            entries_guard.pop_front();
        }
        drop(entries_guard);
        
        Ok(file_path)
    }

    /// Gets short-term memory entries
    ///
    /// # Returns
    /// * `Vec<MemoryEntry>` - Copy of memory entries
    pub async fn get_short_term_memory(&self) -> Vec<MemoryEntry> {
        let entries = self.entries.read().await;
        entries.iter().cloned().collect()
    }

    /// Adds entry to short-term memory
    ///
    /// # Arguments
    /// * `entry` - The memory entry to add
    pub async fn add_short_term_memory(&self, entry: MemoryEntry) {
        let mut entries = self.entries.write().await;
        entries.push_back(entry);
        // Keep only last MAX_SHORT_TERM_ENTRIES entries
        if entries.len() > MAX_SHORT_TERM_ENTRIES {
            entries.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memory_store_creation() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);
        assert_eq!(store.get_short_term_memory().await.len(), 0);
    }

    #[tokio::test]
    async fn test_short_term_memory() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        let entry = MemoryEntry {
            content: "Test content".to_string(),
            timestamp: chrono::Utc::now(),
            memory_type: types::MemoryType::LongTerm,
            file_path: None,
        };

        store.add_short_term_memory(entry).await;
        let memory = store.get_short_term_memory().await;
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].content, "Test content");
    }

    #[tokio::test]
    async fn test_short_term_memory_limit() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Add more than 100 entries
        for i in 0..150 {
            let entry = MemoryEntry {
                content: format!("Content {}", i),
                timestamp: chrono::Utc::now(),
                memory_type: types::MemoryType::LongTerm,
                file_path: None,
            };
            store.add_short_term_memory(entry).await;
        }

        let memory = store.get_short_term_memory().await;
        assert_eq!(memory.len(), 100);
        // Should keep the last 100 entries
        assert_eq!(memory[0].content, "Content 50");
        assert_eq!(memory[99].content, "Content 149");
    }

    #[tokio::test]
    async fn test_concurrent_memory_access() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Clone store for concurrent access
        let store1 = store.clone();
        let store2 = store.clone();
        let store3 = store.clone();

        // Spawn multiple tasks that add entries concurrently
        let handle1 = tokio::spawn(async move {
            for i in 0..10 {
                let entry = MemoryEntry {
                    content: format!("Task 1 - Entry {}", i),
                    timestamp: chrono::Utc::now(),
                    memory_type: types::MemoryType::LongTerm,
                    file_path: None,
                };
                store1.add_short_term_memory(entry).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 0..10 {
                let entry = MemoryEntry {
                    content: format!("Task 2 - Entry {}", i),
                    timestamp: chrono::Utc::now(),
                    memory_type: types::MemoryType::LongTerm,
                    file_path: None,
                };
                store2.add_short_term_memory(entry).await;
            }
        });

        let handle3 = tokio::spawn(async move {
            for i in 0..10 {
                let entry = MemoryEntry {
                    content: format!("Task 3 - Entry {}", i),
                    timestamp: chrono::Utc::now(),
                    memory_type: types::MemoryType::LongTerm,
                    file_path: None,
                };
                store3.add_short_term_memory(entry).await;
            }
        });

        // Wait for all tasks to complete
        handle1.await.unwrap();
        handle2.await.unwrap();
        handle3.await.unwrap();

        // Verify all entries were added
        let memory = store.get_short_term_memory().await;
        assert_eq!(memory.len(), 30);
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Clone store for concurrent access
        let store1 = store.clone();
        let store2 = store.clone();

        // Spawn tasks that write to files concurrently
        let handle1 = tokio::spawn(async move {
            for i in 0..5 {
                let _ = store1.append_to_memory(format!("Long-term entry {}", i)).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 0..5 {
                let _ = store2.create_daily_note(format!("Daily note {}", i)).await;
            }
        });

        // Wait for completion
        handle1.await.unwrap();
        handle2.await.unwrap();

        // Verify short-term memory was populated
        let memory = store.get_short_term_memory().await;
        assert_eq!(memory.len(), 10);
    }
}