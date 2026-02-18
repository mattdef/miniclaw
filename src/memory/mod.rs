//! Memory system for the agent
//!
//! This module provides memory management capabilities including:
//! - Short-term memory (in-memory storage)
//! - Long-term memory (persistent storage in MEMORY.md)
//! - Daily notes (YYYY-MM-DD.md files)
//! - Memory ranking and retrieval

use std::path::PathBuf;

pub mod daily_notes;
pub mod long_term;
pub mod ranker;
pub mod short_term;
pub mod types;

pub use daily_notes::{
    DAILY_NOTE_RETENTION_DAYS, DEFAULT_RECENT_DAYS, DailyNoteEntry, DailyNoteSection,
};
pub use long_term::{LongTermMemory, LongTermMemoryEntry, MemorySection};
pub use ranker::{
    DEFAULT_SEARCH_LIMIT, MAX_SEARCH_RESULTS, MemoryRanker, MemorySource, RankedMemory,
};
pub use short_term::{
    MAX_SHORT_TERM_ENTRIES, MemoryEntry as ShortTermMemoryEntry, ShortTermMemory,
};

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

    /// Reads recent daily notes from the last N days
    ///
    /// # Arguments
    /// * `days` - Number of days to read (going backwards from today)
    ///
    /// # Returns
    /// * `Ok(Vec<DailyNoteSection>)` - List of daily note sections, sorted chronologically
    /// * `Err(MemoryError)` - If operation fails
    pub async fn read_recent_daily_notes(
        &self,
        days: usize,
    ) -> Result<Vec<daily_notes::DailyNoteSection>, MemoryError> {
        daily_notes::read_recent_days(&self.workspace_path, days).await
    }

    /// Cleans up old daily note files
    ///
    /// Removes daily note files older than 30 days, but always preserves
    /// files from the current month.
    ///
    /// # Returns
    /// * `Ok((usize, usize))` - Number of files deleted and bytes freed
    /// * `Err(MemoryError)` - If cleanup fails
    pub async fn cleanup_daily_notes(&self) -> Result<(usize, usize), MemoryError> {
        daily_notes::cleanup_old_daily_notes(&self.workspace_path).await
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

    /// Searches memories across all sources (long-term and daily notes)
    ///
    /// Uses keyword matching to find relevant memories and returns
    /// ranked results sorted by relevance score.
    ///
    /// By default, searches the last 30 days of daily notes and all
    /// long-term memory entries.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return (default: 5, max: 20)
    ///
    /// # Returns
    /// * `Ok(Vec<RankedMemory>)` - Ranked search results
    /// * `Err(MemoryError)` - If search fails
    pub async fn search_memories(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RankedMemory>, MemoryError> {
        let ranker = MemoryRanker::new(self.workspace_path.clone());
        ranker.search_all(query, limit).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to search memories");
            e
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;
    use tokio::fs;

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

        store
            .add_short_term_memory("Test content".to_string())
            .await;
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
                store1
                    .add_short_term_memory(format!("Task 1 - Entry {}", i))
                    .await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 0..10 {
                store2
                    .add_short_term_memory(format!("Task 2 - Entry {}", i))
                    .await;
            }
        });

        let handle3 = tokio::spawn(async move {
            for i in 0..10 {
                store3
                    .add_short_term_memory(format!("Task 3 - Entry {}", i))
                    .await;
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
                let _ = store1
                    .append_to_memory(format!("Long-term entry {}", i))
                    .await;
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
        store
            .short_term()
            .add_entry("Direct access".to_string())
            .await;

        assert_eq!(store.short_term().len().await, 1);
        let entries = store.short_term().get_entries().await;
        assert_eq!(entries[0].content, "Direct access");
    }

    #[tokio::test]
    async fn test_read_recent_daily_notes() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Create some daily notes
        store
            .create_daily_note("Entry 1".to_string())
            .await
            .unwrap();
        store
            .create_daily_note("Entry 2".to_string())
            .await
            .unwrap();

        // Read recent daily notes
        let sections = store.read_recent_daily_notes(7).await.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].entries.len(), 2);
    }

    #[tokio::test]
    async fn test_read_recent_daily_notes_empty() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        let sections = store.read_recent_daily_notes(7).await.unwrap();
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_daily_notes() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory_dir = workspace_path.join("memory");
        let store = MemoryStore::new(workspace_path.clone());

        // Create today's daily note
        store
            .create_daily_note("Today's entry".to_string())
            .await
            .unwrap();

        // Create an old file (40 days ago)
        let old_date = (Utc::now() - Duration::days(40)).format("%Y-%m-%d");
        let old_file = memory_dir.join(format!("{}.md", old_date));
        fs::write(
            &old_file,
            "# Daily Note\n\n## 10:00:00 UTC\n\nOld entry\n\n---\n",
        )
        .await
        .unwrap();

        // Run cleanup
        let (deleted, bytes_freed) = store.cleanup_daily_notes().await.unwrap();
        assert_eq!(deleted, 1);
        assert!(bytes_freed > 0);

        // Old file should be deleted, today's file should remain
        assert!(!old_file.exists());
        let today_file = memory_dir.join(format!("{}.md", Utc::now().format("%Y-%m-%d")));
        assert!(today_file.exists());
    }

    #[tokio::test]
    async fn test_search_memories_empty() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Search with no memories
        let results = store.search_memories("test", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_memories_with_content() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Add some content to long-term memory
        store
            .append_to_memory("project meeting with team".to_string())
            .await
            .unwrap();
        store
            .append_to_memory("architecture review session".to_string())
            .await
            .unwrap();
        store
            .append_to_memory("daily standup notes".to_string())
            .await
            .unwrap();

        // Search for "project"
        let results = store.search_memories("project", 5).await.unwrap();
        assert!(!results.is_empty());

        // The "project meeting" entry should have score >= 1
        let project_result = results.iter().find(|r| r.content.contains("project"));
        assert!(project_result.is_some());
        assert!(project_result.unwrap().score >= 1);
    }

    #[tokio::test]
    async fn test_search_memories_with_daily_notes() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Create a daily note
        store
            .create_daily_note("project planning session today".to_string())
            .await
            .unwrap();

        // Search should find the daily note
        let results = store.search_memories("project", 5).await.unwrap();
        assert!(!results.is_empty());

        // Should find the daily note
        let daily_result = results
            .iter()
            .find(|r| r.source == crate::memory::MemorySource::DailyNote);
        assert!(daily_result.is_some());
    }

    #[tokio::test]
    async fn test_search_memories_ranking() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Add entries with different relevance
        store
            .append_to_memory("project meeting".to_string())
            .await
            .unwrap();
        store
            .append_to_memory("project architecture review meeting".to_string())
            .await
            .unwrap();
        store
            .append_to_memory("unrelated entry".to_string())
            .await
            .unwrap();

        // Search for "project meeting"
        let results = store.search_memories("project meeting", 5).await.unwrap();

        // Should return at least 2 results
        assert!(results.len() >= 2);

        // First result should have higher score (contains both "project" and "meeting")
        assert!(results[0].score >= results[1].score);
    }

    #[tokio::test]
    async fn test_search_memories_empty_query() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        store
            .append_to_memory("some content".to_string())
            .await
            .unwrap();

        // Empty query should return empty results
        let results = store.search_memories("", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_memories_limit() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let store = MemoryStore::new(workspace_path);

        // Add multiple entries
        for i in 0..10 {
            store
                .append_to_memory(format!("test entry {}", i))
                .await
                .unwrap();
        }

        // Search with limit of 3
        let results = store.search_memories("test", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
