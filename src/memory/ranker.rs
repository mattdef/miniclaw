//! Memory ranker for searching and ranking memories
//!
//! This module provides keyword-based memory ranking and searching functionality.
//! It searches across both long-term memory (MEMORY.md) and daily notes (YYYY-MM-DD.md)
//! and returns ranked results based on keyword match counts.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::memory::types::MemoryError;

/// Default number of search results to return
pub const DEFAULT_SEARCH_LIMIT: usize = 5;

/// Maximum number of search results allowed
pub const MAX_SEARCH_RESULTS: usize = 20;

/// Default number of days to search in daily notes
pub const DEFAULT_DAILY_NOTE_SEARCH_DAYS: usize = 30;

/// Source of a memory entry
#[derive(Debug, Clone, PartialEq)]
pub enum MemorySource {
    /// From MEMORY.md (long-term memory)
    LongTerm,
    /// From daily note files (YYYY-MM-DD.md)
    DailyNote,
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::LongTerm => write!(f, "Long-term Memory"),
            MemorySource::DailyNote => write!(f, "Daily Note"),
        }
    }
}

/// A ranked memory result
#[derive(Debug, Clone, PartialEq)]
pub struct RankedMemory {
    /// Full content of the memory
    pub content: String,
    /// Relevance score (number of matching keywords)
    pub score: usize,
    /// Source of the memory
    pub source: MemorySource,
    /// Date of the memory (if available)
    pub date: Option<DateTime<Utc>>,
    /// Excerpt for display (truncated content)
    pub excerpt: String,
}

/// Memory ranker for searching and ranking memories
#[derive(Debug, Clone)]
pub struct MemoryRanker {
    /// Workspace path for file operations
    workspace_path: PathBuf,
}

impl MemoryRanker {
    /// Creates a new MemoryRanker
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory containing memory files
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    /// Tokenizes text into searchable keywords
    ///
    /// Converts text to lowercase, splits on whitespace, and removes punctuation.
    ///
    /// # Arguments
    /// * `text` - The text to tokenize
    ///
    /// # Returns
    /// Vector of lowercase tokens without punctuation
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Calculates relevance score for content against query tokens
    ///
    /// Score is the count of unique query tokens found in the content.
    ///
    /// # Arguments
    /// * `content` - The memory content to score
    /// * `query_tokens` - The tokenized query
    ///
    /// # Returns
    /// Relevance score (0 or higher)
    pub fn calculate_score(content: &str, query_tokens: &[String]) -> usize {
        let content_lower = content.to_lowercase();
        query_tokens
            .iter()
            .filter(|token| content_lower.contains(token.as_str()))
            .count()
    }

    /// Creates an excerpt from content (first 150 chars or up to newline)
    ///
    /// # Arguments
    /// * `content` - The full content
    ///
    /// # Returns
    /// Truncated excerpt suitable for display
    pub fn create_excerpt(content: &str) -> String {
        // Try to find first newline
        if let Some(newline_pos) = content.find('\n') {
            if newline_pos > 0 && newline_pos <= 150 {
                return content[..newline_pos].to_string();
            }
        }

        // Otherwise truncate at 150 chars
        if content.len() > 150 {
            format!("{}...", &content[..147])
        } else {
            content.to_string()
        }
    }

    /// Searches long-term memory (MEMORY.md) for matching entries
    ///
    /// # Arguments
    /// * `query_tokens` - Tokenized search query
    ///
    /// # Returns
    /// * `Ok(Vec<RankedMemory>)` - Ranked results from long-term memory
    /// * `Err(MemoryError)` - If reading fails
    pub async fn search_long_term(
        &self,
        query_tokens: &[String],
    ) -> Result<Vec<RankedMemory>, MemoryError> {
        use crate::memory::LongTermMemory;

        tracing::debug!(
            token_count = query_tokens.len(),
            "Searching long-term memory"
        );

        let long_term = LongTermMemory::new(&self.workspace_path);
        let sections = long_term.read_all().await?;

        let mut results = Vec::new();

        for section in sections {
            for entry in section.entries {
                let score = Self::calculate_score(&entry.content, query_tokens);
                if score > 0 {
                    results.push(RankedMemory {
                        content: entry.content.clone(),
                        score,
                        source: MemorySource::LongTerm,
                        date: Some(entry.timestamp),
                        excerpt: Self::create_excerpt(&entry.content),
                    });
                }
            }
        }

        tracing::debug!(
            results_found = results.len(),
            "Long-term memory search complete"
        );
        Ok(results)
    }

    /// Searches daily notes for matching entries
    ///
    /// Searches the last 30 days of daily notes by default.
    ///
    /// # Arguments
    /// * `query_tokens` - Tokenized search query
    ///
    /// # Returns
    /// * `Ok(Vec<RankedMemory>)` - Ranked results from daily notes
    /// * `Err(MemoryError)` - If reading fails
    pub async fn search_daily_notes(
        &self,
        query_tokens: &[String],
    ) -> Result<Vec<RankedMemory>, MemoryError> {
        use crate::memory::daily_notes::read_recent_days;

        tracing::debug!(
            token_count = query_tokens.len(),
            days = DEFAULT_DAILY_NOTE_SEARCH_DAYS,
            "Searching daily notes"
        );

        let sections =
            read_recent_days(&self.workspace_path, DEFAULT_DAILY_NOTE_SEARCH_DAYS).await?;

        let mut results = Vec::new();

        for section in sections {
            for entry in section.entries {
                let score = Self::calculate_score(&entry.content, query_tokens);
                if score > 0 {
                    results.push(RankedMemory {
                        content: entry.content.clone(),
                        score,
                        source: MemorySource::DailyNote,
                        date: Some(entry.timestamp),
                        excerpt: Self::create_excerpt(&entry.content),
                    });
                }
            }
        }

        tracing::debug!(results_found = results.len(), "Daily notes search complete");
        Ok(results)
    }

    /// Performs unified search across all memory sources
    ///
    /// Searches both long-term memory and daily notes, combines results,
    /// and returns top N ranked by relevance score.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return (default: 5, max: 20)
    ///
    /// # Returns
    /// * `Ok(Vec<RankedMemory>)` - Ranked results from all sources
    /// * `Err(MemoryError)` - If search fails
    pub async fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RankedMemory>, MemoryError> {
        let query_tokens = Self::tokenize(query);

        if query_tokens.is_empty() {
            tracing::debug!("Empty query after tokenization, returning no results");
            return Ok(Vec::new());
        }

        tracing::info!(
            token_count = query_tokens.len(),
            limit = limit,
            "Starting unified memory search"
        );

        // Search both sources in parallel
        let (long_term_results, daily_results) = tokio::join!(
            self.search_long_term(&query_tokens),
            self.search_daily_notes(&query_tokens)
        );

        // Combine results
        let mut all_results = Vec::new();
        all_results.extend(long_term_results?);
        all_results.extend(daily_results?);

        tracing::debug!(
            total_results = all_results.len(),
            "Combined results from all sources"
        );

        // Sort by score (descending), then by date (newest first) for ties
        all_results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| b.date.cmp(&a.date)));

        // Apply limit
        let limit = limit.min(MAX_SEARCH_RESULTS);
        let final_results: Vec<RankedMemory> = all_results.into_iter().take(limit).collect();

        tracing::info!(
            returned_count = final_results.len(),
            "Memory search complete"
        );
        Ok(final_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = MemoryRanker::tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_punctuation() {
        let tokens = MemoryRanker::tokenize("Hello, world! How are you?");
        assert_eq!(tokens, vec!["hello", "world", "how", "are", "you"]);
    }

    #[test]
    fn test_tokenize_mixed_case() {
        let tokens = MemoryRanker::tokenize("HELLO World TeSt");
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = MemoryRanker::tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_whitespace_only() {
        let tokens = MemoryRanker::tokenize("   \n\t  ");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_calculate_score_exact_match() {
        let tokens = vec!["hello".to_string(), "world".to_string()];
        let score = MemoryRanker::calculate_score("hello world", &tokens);
        assert_eq!(score, 2);
    }

    #[test]
    fn test_calculate_score_partial_match() {
        let tokens = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
        let score = MemoryRanker::calculate_score("hello there", &tokens);
        assert_eq!(score, 1);
    }

    #[test]
    fn test_calculate_score_no_match() {
        let tokens = vec!["xyz".to_string(), "abc".to_string()];
        let score = MemoryRanker::calculate_score("hello world", &tokens);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_calculate_score_case_insensitive() {
        let tokens = vec!["hello".to_string()];
        let score = MemoryRanker::calculate_score("HELLO World", &tokens);
        assert_eq!(score, 1);
    }

    #[test]
    fn test_calculate_score_empty_tokens() {
        let tokens: Vec<String> = vec![];
        let score = MemoryRanker::calculate_score("hello world", &tokens);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_calculate_score_empty_content() {
        let tokens = vec!["hello".to_string()];
        let score = MemoryRanker::calculate_score("", &tokens);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_create_excerpt_short_content() {
        let content = "Short content";
        let excerpt = MemoryRanker::create_excerpt(content);
        assert_eq!(excerpt, "Short content");
    }

    #[test]
    fn test_create_excerpt_long_content() {
        let content = "a".repeat(200);
        let excerpt = MemoryRanker::create_excerpt(&content);
        assert_eq!(excerpt.len(), 150); // 147 + "..."
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn test_create_excerpt_with_newline() {
        let content = "First line\nSecond line\nThird line";
        let excerpt = MemoryRanker::create_excerpt(content);
        assert_eq!(excerpt, "First line");
    }

    #[test]
    fn test_memory_source_display() {
        assert_eq!(MemorySource::LongTerm.to_string(), "Long-term Memory");
        assert_eq!(MemorySource::DailyNote.to_string(), "Daily Note");
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_SEARCH_LIMIT, 5);
        assert_eq!(MAX_SEARCH_RESULTS, 20);
        assert_eq!(DEFAULT_DAILY_NOTE_SEARCH_DAYS, 30);
    }

    #[test]
    fn test_ranked_memory_creation() {
        let ranked = RankedMemory {
            content: "Test content".to_string(),
            score: 5,
            source: MemorySource::LongTerm,
            date: None,
            excerpt: "Test".to_string(),
        };
        assert_eq!(ranked.content, "Test content");
        assert_eq!(ranked.score, 5);
        assert_eq!(ranked.source, MemorySource::LongTerm);
    }

    // Error path tests
    #[tokio::test]
    async fn test_search_long_term_with_missing_memory_file() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let ranker = MemoryRanker::new(workspace_path);
        let query_tokens = vec!["test".to_string()];

        // Should handle missing MEMORY.md gracefully
        let result = ranker.search_long_term(&query_tokens).await;
        assert!(result.is_ok()); // Should return empty results, not error
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_search_daily_notes_with_no_files() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let ranker = MemoryRanker::new(workspace_path);
        let query_tokens = vec!["test".to_string()];

        // Should handle no daily notes gracefully
        let result = ranker.search_daily_notes(&query_tokens).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_search_all_with_empty_workspace() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let ranker = MemoryRanker::new(workspace_path);

        // Should handle empty workspace gracefully
        let result = ranker.search_all("test query", 5).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_search_all_respects_max_limit() {
        use tempfile::tempdir;
        use tokio::fs;

        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory_dir = workspace_path.join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create MEMORY.md with many matching entries
        let mut content = String::from("# Memory\n\n");
        for i in 0..30 {
            content.push_str(&format!("## 2026-02-{:02}\n\n", i + 1));
            content.push_str(&format!("- test entry number {} at 10:00:00 UTC\n\n", i));
        }
        fs::write(memory_dir.join("MEMORY.md"), content)
            .await
            .unwrap();

        let ranker = MemoryRanker::new(workspace_path);

        // Try to get 100 results, should cap at MAX_SEARCH_RESULTS (20)
        let result = ranker.search_all("test", 100).await.unwrap();
        assert!(result.len() <= MAX_SEARCH_RESULTS);
    }

    #[test]
    fn test_create_excerpt_handles_unicode() {
        let content = "Hello 世界 こんにちは мир";
        let excerpt = MemoryRanker::create_excerpt(content);
        assert!(excerpt.len() <= 150);
    }
}
