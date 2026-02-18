//! Async filesystem helpers
//!
//! Provides thin wrappers around `tokio::fs` to avoid blocking the async
//! runtime with synchronous `std::fs` calls.

use std::io;
use std::path::Path;

/// Returns `true` if the path exists (async, non-blocking).
///
/// This is a drop-in replacement for the blocking `Path::exists()` check.
/// Internally uses `tokio::fs::try_exists` which performs a non-blocking
/// filesystem access.
///
/// # Errors
/// Returns an `io::Error` only on OS-level errors (e.g. permission denied).
/// A missing path returns `Ok(false)`, not an error.
///
/// # Examples
/// ```no_run
/// use miniclaw::utils::fs::file_exists;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// if file_exists("/tmp/MEMORY.md").await? {
///     println!("file found");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn file_exists(path: impl AsRef<Path>) -> io::Result<bool> {
    tokio::fs::try_exists(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_exists_returns_false_for_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.txt");
        assert!(!file_exists(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_file_exists_returns_true_for_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exists.txt");
        tokio::fs::write(&path, b"hello").await.unwrap();
        assert!(file_exists(&path).await.unwrap());
    }
}
