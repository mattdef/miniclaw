# AGENTS.md - miniclaw

Guidelines for AI agents working on the miniclaw codebase.

## Project Overview

- **Language**: Rust (Edition 2024, MSRV 1.85)
- **Type**: CLI tool and daemon for edge AI agents
- **Architecture**: Async tokio-based with modular components

## Build/Test/Lint Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test --all                    # Run all tests
cargo test <test_name>              # Run single test by name
cargo test --test performance_tests # Run specific test file
cargo test <module>::<test_name>    # Run specific test in module

# Lint
cargo fmt -- --check                # Check formatting
cargo fmt                           # Fix formatting
cargo clippy --all-targets -- -D warnings  # Run linter (strict)

# Check
cargo check                         # Fast syntax/type check
cargo check --benches              # Verify benchmarks compile

# Benchmarks
cargo bench                         # Run Criterion benchmarks
```

## Code Style Guidelines

### Import Ordering

```rust
// 1. Standard library (std, core, alloc)
use std::sync::Arc;
use std::collections::HashMap;

// 2. Third-party crates
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// 3. Internal modules (crate::)
use crate::config::Config;
use crate::providers::LlmProvider;
```

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Structs | PascalCase | `AgentLoop`, `ToolRegistry` |
| Traits | PascalCase | `LlmProvider`, `Tool` |
| Enums | PascalCase | `LlmRole::System` |
| Functions | snake_case | `build_context()`, `execute()` |
| Constants | UPPER_SNAKE_CASE | `MAX_ITERATIONS` |
| Modules | snake_case | `agent_loop`, `tool_registry` |
| Type Aliases | PascalCase | `ToolResult`, `BoxedProvider` |

### Error Handling

Use layered error handling:

1. **thiserror** for domain errors:
```rust
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ProviderError {
    #[error("Network error: {message}")]
    Network { message: String },
    #[error("Authentication error: {message}")]
    Auth { message: String },
}
```

2. **anyhow** for application errors:
```rust
use anyhow::{Context, Result};

pub async fn run(config: &Config) -> Result<()> {
    let provider = create_provider(config)
        .context("Failed to create LLM provider")?;
    Ok(())
}
```

3. **Custom Result types** per module:
```rust
pub type Result<T> = std::result::Result<T, AgentError>;
```

### Async Patterns

- Use `tokio` as the runtime
- Use `async-trait` for async traits
- Prefer `Arc<RwLock<T>>` for shared mutable state
- Use `Arc<dyn Trait>` for dependency injection

### Documentation

- Use `//!` for module-level docs
- Include `# Arguments`, `# Returns`, `# Examples` sections
- Document "why", not just "what"

```rust
/// Executes the agent loop with the configured provider.
///
/// # Arguments
/// * `config` - Application configuration
/// * `provider` - LLM provider implementation
///
/// # Returns
/// Result indicating success or failure
///
/// # Examples
/// ```
/// let result = run_agent_loop(&config, provider).await?;
/// ```
pub async fn run_agent_loop(config: &Config, provider: Arc<dyn LlmProvider>) -> Result<()> {
```

### Logging

Use `tracing` with structured fields:

```rust
info!(version = env!(CARGO_PKG_VERSION"), "Starting gateway");
error!(error = ?e, "Failed to initialize provider");
debug!(session_id = %id, "Session loaded");
```

Log to stderr, not stdout.

### Testing

- Unit tests: inline in `#[cfg(test)]` modules
- Integration tests: in `/tests/` directory
- Use `tempfile` for test files
- Use `assert_cmd` and `predicates` for CLI tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_loop() {
        // Arrange
        let config = Config::default();
        
        // Act
        let result = run_agent_loop(&config, mock_provider()).await;
        
        // Assert
        assert!(result.is_ok());
    }
}
```

### Module Structure

Keep modules organized:

```
src/
├── main.rs          # Entry point
├── lib.rs           # Library exports
├── cli.rs           # CLI handling
├── module/
│   ├── mod.rs       # Re-exports
│   ├── submodule.rs
│   └── tests.rs     # Optional: separate test file
```

### Binary Size Constraints

This project targets edge hardware (256MB RAM). Binary must stay under 15MB:

```bash
./scripts/check-binary-size.sh
```

Avoid unnecessary dependencies and use `--release` for size checks.

### CI Requirements

All PRs must pass:
- `cargo test --all`
- `cargo fmt -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo check --benches`

## AI Assistant Notes

- No Cursor rules or Copilot instructions exist in this repo
- Follow existing patterns in the codebase
- When in doubt, prefer explicit error handling over panics
- Maintain the async/await style already established
- Preserve the modular architecture with clear trait boundaries
