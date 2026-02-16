//! Memory system for the agent
//!
//! This module provides memory management capabilities including:
//! - Short-term memory (in-memory storage)
//! - Long-term memory (persistent storage in MEMORY.md)
//! - Daily notes (YYYY-MM-DD.md files)
//! - Memory ranking and retrieval

use std::path::PathBuf;

pub mod long_term;
pub mod daily_notes;
pub mod short_term;
pub mod types;

pub use short_term::{ShortTermMemory, MemoryEntry as ShortTermMemoryEntry, MAX_SHORT_TERM_ENTRIES};
pub use long_term::{LongTermMemory, LongTermMemoryEntry, MemorySection};

use types::{MemoryEntry, MemoryError};

/// MemoryStore manages both short-term and long-term memory
///
/// Uses ShortTermMemory for in-memory storage and LongTermMemory for persistent storage.
/// The store can be cloned to share the same memory storage across tasks.
#[derive(Debug, Clone)]
pub struct MemoryStore {
    /// Short-term memory module
    short_term: ShortTermMemory,
    /// Long-term memory module
    long_term: LongTermMemory,
    /// Workspace path for file operations
    workspace_path: PathBuf,
}

impl MemoryStore {
    /// Creates a new MemoryStore with empty memory
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory for file operations
    pub fn new(workspace_path: PathBuf) -> Self {
        let long_term = LongTermMemory::new(&workspace_path);
        Self {
            short_term: ShortTermMemory::new(),
            long_term,
            workspace_path,
        }
    }

    /// Returns a reference to the short-term memory module
    pub fn short_term(&self) -> &ShortTermMemory {
        &self.short_term
    }

    /// Returns a reference to the long-term memory module
    pub fn long_term(&self) -> &LongTermMemory {
        &self.long_term
    }

    /// Appends content to long-term memory (MEMORY.md)
    ///
    /// Also adds the content to short-term memory.
    ///
    /// # Arguments
    /// * `content` - The content to store
    ///
    /// # Returns
    /// * `Ok(String)` - Path to the memory file
    /// * `Err(MemoryError)` - If storage fails
    pub async fn append_to_memory(&self, content: String) -> Result<String, MemoryError> {
        // Use the unified LongTermMemory::append_entry method
        self.long_term.append_entry(&content).await?;
        
        // Add to short-term memory
        self.short_term.add_entry(content).await;
        
        // Return file path
        let file_path = self.workspace_path.join("memory").join("MEMORY.md");
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Creates a daily note
    ///
    /// Also adds the content to short-term memory.
    ///
    /// # Arguments
    /// * `content` - The content to store
    ///
    /// # Returns
    /// * `Ok(String)` - Path to the daily note file
    /// * `Err(MemoryError)` - If creation fails
    pub async fn create_daily_note(&self, content: String) -> Result<String, MemoryError> {
        let (file_path, _entry) = daily_notes::create_daily_note(
            &self.workspace_path,
            content.clone(),
            None::<fn(MemoryEntry)>,
        )
        .await?;
        
        // Add to short-term memory using the new module
        self.short_term.add_entry(content).await;
        
        Ok(file_path)
    }

    /// Gets short-term memory entries
    ///
    /// # Returns
    /// * `Vec<ShortTermMemoryEntry>` - Copy of memory entries
    pub async fn get_short_term_memory(&self) -> Vec<ShortTermMemoryEntry> {
        self.short_term.get_entries().await
    }

    /// Adds content to short-term memory
    ///
    /// # Arguments
    /// * `content` - The content to add
    pub async fn add_short_term_memory(&self, content: String) {
        self.short_term.add_entry(content).await;
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

        store.add_short_term_memory("Test content".to_string()).await;
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
            store.add_short_term_memory(format!("Content {}", i)).await;
        }

        let memory = store.get_short_term_memory().await;
        assert_eq!(memory.len(), 100);
        // Should keep the last 100 entries (FIFO removes first 50)
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
                store1.add_short_term_memory(format!("Task 1 - Entry {}", i)).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 0..10 {
                store2.add_short_term_memory(format!("Task 2 - Entry {}", i)).await;
            }
        });

        let handle3 = tokio::spawn(async move {
            for i in 0..10 {
                store3.add_short_term_memory(format!("Task 3 - Entry {}", i)).await;
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

    #[tokio::test]
    async fn test_short_term_accessor() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Test direct access to short_term module
        store.short_term().add_entry("Direct access".to_string()).await;
        
        assert_eq!(store.short_term().len().await, 1);
        let entries = store.short_term().get_entries().await;
        assert_eq!(entries[0].content, "Direct access");
    }
}