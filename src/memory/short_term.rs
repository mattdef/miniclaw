//! Short-term memory implementation
//!
//! Provides in-memory storage for recent context using a VecDeque with FIFO behavior.
//! Maximum capacity is 100 entries. Not persisted to disk.

use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Maximum number of entries in short-term memory
pub const MAX_SHORT_TERM_ENTRIES: usize = 100;

/// A single memory entry with content and timestamp
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// The content of the memory entry
    pub content: String,
    /// When the entry was created (ISO 8601 UTC)
    pub timestamp: DateTime<Utc>,
}

impl MemoryEntry {
    /// Creates a new memory entry with the current timestamp
    ///
    /// # Arguments
    /// * `content` - The content to store
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
        }
    }
}

/// Short-term memory storage using VecDeque with FIFO behavior
///
/// Thread-safe via Arc<RwLock<VecDeque>>. Maximum 100 entries.
/// When capacity is reached, oldest entries are removed (FIFO).
/// Not persisted to disk - cleared on restart.
#[derive(Debug, Clone)]
pub struct ShortTermMemory {
    /// The entries storage
    entries: Arc<RwLock<VecDeque<MemoryEntry>>>,
}

impl ShortTermMemory {
    /// Creates a new empty ShortTermMemory
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_SHORT_TERM_ENTRIES))),
        }
    }

    /// Adds an entry to short-term memory
    ///
    /// If at capacity, removes the oldest entry (FIFO behavior).
    /// Automatically adds timestamp.
    ///
    /// # Arguments
    /// * `content` - The content to store
    pub async fn add_entry(&self, content: String) {
        let entry = MemoryEntry::new(content);
        let mut entries = self.entries.write().await;

        // FIFO: Remove oldest if at capacity
        if entries.len() >= MAX_SHORT_TERM_ENTRIES {
            entries.pop_front();
        }

        entries.push_back(entry);

        // Log at DEBUG level
        tracing::debug!(
            entry_count = entries.len(),
            max_entries = MAX_SHORT_TERM_ENTRIES,
            "Added entry to short-term memory"
        );
    }

    /// Gets all entries in chronological order (oldest first)
    ///
    /// # Returns
    /// * `Vec<MemoryEntry>` - All entries in chronological order, empty if none
    pub async fn get_entries(&self) -> Vec<MemoryEntry> {
        let entries = self.entries.read().await;
        // Clone entries to return (chronological order maintained - VecDeque is FIFO)
        entries.iter().cloned().collect()
    }

    /// Clears all entries from short-term memory
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        tracing::info!("Short-term memory cleared");
    }

    /// Returns the number of entries in short-term memory
    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    /// Returns true if short-term memory is empty
    pub async fn is_empty(&self) -> bool {
        let entries = self.entries.read().await;
        entries.is_empty()
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: Initialization
    #[tokio::test]
    async fn test_initialization_empty() {
        let memory = ShortTermMemory::new();
        assert_eq!(memory.len().await, 0);
        assert!(memory.is_empty().await);
    }

    // Test 2: Adding single entry
    #[tokio::test]
    async fn test_add_single_entry() {
        let memory = ShortTermMemory::new();
        memory.add_entry("Test content".to_string()).await;

        assert_eq!(memory.len().await, 1);
        assert!(!memory.is_empty().await);

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Test content");
    }

    // Test 3: Adding multiple entries
    #[tokio::test]
    async fn test_add_multiple_entries() {
        let memory = ShortTermMemory::new();

        for i in 0..5 {
            memory.add_entry(format!("Entry {}", i)).await;
        }

        assert_eq!(memory.len().await, 5);

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 5);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.content, format!("Entry {}", i));
        }
    }

    // Test 4: FIFO behavior at capacity (101 entries)
    #[tokio::test]
    async fn test_fifo_behavior_at_capacity() {
        let memory = ShortTermMemory::new();

        // Add 101 entries (1 over capacity)
        for i in 0..101 {
            memory.add_entry(format!("Content {}", i)).await;
        }

        // Should only have 100 entries
        assert_eq!(memory.len().await, 100);

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 100);

        // First entry should be "Content 1" ("Content 0" was removed)
        assert_eq!(entries[0].content, "Content 1");

        // Last entry should be "Content 100"
        assert_eq!(entries[99].content, "Content 100");
    }

    // Test 5: Entry ordering (chronological)
    #[tokio::test]
    async fn test_entry_ordering_chronological() {
        let memory = ShortTermMemory::new();

        memory.add_entry("First".to_string()).await;
        memory.add_entry("Second".to_string()).await;
        memory.add_entry("Third".to_string()).await;

        let entries = memory.get_entries().await;
        assert_eq!(entries[0].content, "First");
        assert_eq!(entries[1].content, "Second");
        assert_eq!(entries[2].content, "Third");
    }

    // Test 6: get_entries returns correct data
    #[tokio::test]
    async fn test_get_entries_returns_correct_data() {
        let memory = ShortTermMemory::new();

        memory.add_entry("Test 1".to_string()).await;
        memory.add_entry("Test 2".to_string()).await;

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "Test 1");
        assert_eq!(entries[1].content, "Test 2");

        // Verify timestamps are present
        assert!(entries[0].timestamp <= entries[1].timestamp);
    }

    // Test 7: clear() removes all entries
    #[tokio::test]
    async fn test_clear_removes_all_entries() {
        let memory = ShortTermMemory::new();

        memory.add_entry("Entry 1".to_string()).await;
        memory.add_entry("Entry 2".to_string()).await;
        assert_eq!(memory.len().await, 2);

        memory.clear().await;

        assert_eq!(memory.len().await, 0);
        assert!(memory.is_empty().await);

        let entries = memory.get_entries().await;
        assert!(entries.is_empty());
    }

    // Test 8: len() returns correct count
    #[tokio::test]
    async fn test_len_returns_correct_count() {
        let memory = ShortTermMemory::new();

        assert_eq!(memory.len().await, 0);

        memory.add_entry("Entry 1".to_string()).await;
        assert_eq!(memory.len().await, 1);

        memory.add_entry("Entry 2".to_string()).await;
        assert_eq!(memory.len().await, 2);

        memory.clear().await;
        assert_eq!(memory.len().await, 0);
    }

    // Test 9: is_empty() returns true/false correctly
    #[tokio::test]
    async fn test_is_empty_returns_correctly() {
        let memory = ShortTermMemory::new();

        assert!(memory.is_empty().await);

        memory.add_entry("Entry".to_string()).await;
        assert!(!memory.is_empty().await);

        memory.clear().await;
        assert!(memory.is_empty().await);
    }

    // Test 10: Concurrent add_entry calls
    #[tokio::test]
    async fn test_concurrent_add_entry() {
        let memory = ShortTermMemory::new();
        let memory1 = memory.clone();
        let memory2 = memory.clone();
        let memory3 = memory.clone();

        let handle1 = tokio::spawn(async move {
            for i in 0..10 {
                memory1.add_entry(format!("Task1-{}", i)).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 0..10 {
                memory2.add_entry(format!("Task2-{}", i)).await;
            }
        });

        let handle3 = tokio::spawn(async move {
            for i in 0..10 {
                memory3.add_entry(format!("Task3-{}", i)).await;
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();
        handle3.await.unwrap();

        // Should have 30 entries
        assert_eq!(memory.len().await, 30);
    }

    // Test 11: Concurrent get_entries during add
    #[tokio::test]
    async fn test_concurrent_get_entries_during_add() {
        let memory = ShortTermMemory::new();
        let memory1 = memory.clone();
        let memory2 = memory.clone();

        // First add some entries
        for i in 0..10 {
            memory.add_entry(format!("Entry {}", i)).await;
        }

        let handle1 = tokio::spawn(async move {
            for i in 10..20 {
                memory1.add_entry(format!("Entry {}", i)).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            // Read multiple times during adds
            for _ in 0..5 {
                let entries = memory2.get_entries().await;
                // Should always have between 10 and 20 entries
                assert!(entries.len() >= 10 && entries.len() <= 20);
                tokio::task::yield_now().await;
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        assert_eq!(memory.len().await, 20);
    }

    // Test 12: Entry with empty content
    #[tokio::test]
    async fn test_entry_with_empty_content() {
        let memory = ShortTermMemory::new();

        memory.add_entry("".to_string()).await;

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "");
    }

    // Test 13: Entry with long content
    #[tokio::test]
    async fn test_entry_with_long_content() {
        let memory = ShortTermMemory::new();

        let long_content = "a".repeat(10000);
        memory.add_entry(long_content.clone()).await;

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, long_content);
    }

    // Test 14: Boundary - exactly 100 entries
    #[tokio::test]
    async fn test_boundary_exactly_100_entries() {
        let memory = ShortTermMemory::new();

        // Add exactly 100 entries
        for i in 0..100 {
            memory.add_entry(format!("Entry {}", i)).await;
        }

        assert_eq!(memory.len().await, 100);

        let entries = memory.get_entries().await;
        assert_eq!(entries[0].content, "Entry 0");
        assert_eq!(entries[99].content, "Entry 99");
    }

    // Test 15: Boundary - 0 entries (empty)
    #[tokio::test]
    async fn test_boundary_zero_entries() {
        let memory = ShortTermMemory::new();

        assert_eq!(memory.len().await, 0);
        assert!(memory.is_empty().await);

        let entries = memory.get_entries().await;
        assert!(entries.is_empty());
    }

    // Test 16: Entry with unicode characters
    #[tokio::test]
    async fn test_entry_with_unicode() {
        let memory = ShortTermMemory::new();

        memory.add_entry("Hello 世界 🌍".to_string()).await;
        memory.add_entry("Café résumé".to_string()).await;

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "Hello 世界 🌍");
        assert_eq!(entries[1].content, "Café résumé");
    }

    // Test 17: Entry with special characters
    #[tokio::test]
    async fn test_entry_with_special_characters() {
        let memory = ShortTermMemory::new();

        memory
            .add_entry("Special: !@#$%^&*()_+-=[]{}|;':\",./<>?".to_string())
            .await;

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].content,
            "Special: !@#$%^&*()_+-=[]{}|;':\",./<>?"
        );
    }

    // Test 18: Multiple clear operations
    #[tokio::test]
    async fn test_multiple_clear_operations() {
        let memory = ShortTermMemory::new();

        memory.add_entry("Entry".to_string()).await;
        memory.clear().await;
        memory.clear().await; // Second clear should not panic

        assert_eq!(memory.len().await, 0);
        assert!(memory.is_empty().await);
    }

    // Test 19: Timestamp verification
    #[tokio::test]
    async fn test_timestamp_is_set() {
        let memory = ShortTermMemory::new();

        let before = Utc::now();
        memory.add_entry("Test".to_string()).await;
        let after = Utc::now();

        let entries = memory.get_entries().await;
        assert_eq!(entries.len(), 1);

        // Timestamp should be between before and after
        assert!(entries[0].timestamp >= before);
        assert!(entries[0].timestamp <= after);
    }

    // Test 20: Clone creates independent reference
    #[tokio::test]
    async fn test_clone_shares_storage() {
        let memory1 = ShortTermMemory::new();
        let memory2 = memory1.clone();

        memory1.add_entry("Shared".to_string()).await;

        // Both should see the same entry
        assert_eq!(memory1.len().await, 1);
        assert_eq!(memory2.len().await, 1);

        let entries = memory2.get_entries().await;
        assert_eq!(entries[0].content, "Shared");
    }
}
