# Story 2.1: Configuration File Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want miniclaw to manage configuration through a flexible hierarchy,
so that I can customize behavior through files, environment variables, or command flags.

## Acceptance Criteria

**Given** a fresh miniclaw installation
**When** the system loads configuration
**Then** it applies default values first
**And** overrides with `~/.miniclaw/config.json` if it exists
**And** overrides with environment variables (e.g., `MINICLAW_API_KEY`)
**And** finally overrides with CLI flags

**Given** a config file at `~/.miniclaw/config.json`
**When** I define `api_key`, `model`, and `telegram_token` fields
**Then** the system loads and validates these values
**And** applies them to the application

**Given** environment variables are set
**When** I set `OPENROUTER_API_KEY=sk-xxx` or `TELEGRAM_BOT_TOKEN=xxx`
**Then** these values override config file settings
**And** secrets are prioritized from environment variables

**Given** I run a command with CLI flags
**When** I use `--model google/gemini-2.5-flash` or `--config /custom/path.json`
**Then** these values take highest precedence
**And** override both file and environment settings

**Given** the config file has invalid JSON
**When** miniclaw tries to load it
**Then** the system displays a clear error message
**And** suggests running `miniclaw onboard` to recreate it
**And** the exit code is 1

**Given** sensitive configuration data
**When** the config file is created
**Then** file permissions are set to 0600 (owner read/write only)
**And** no other users can read the file

## Tasks / Subtasks

- [x] Create config module structure (AC: 1, 2)
  - [x] Create `src/config/mod.rs` with public Config API
  - [x] Create `src/config/schema.rs` with Config struct definition
  - [x] Create `src/config/loader.rs` with config loading logic
- [x] Implement configuration hierarchy (AC: 1)
  - [x] Define default configuration values
  - [x] Implement file-based config loading from ~/.miniclaw/config.json
  - [x] Implement environment variable override
  - [x] Implement CLI flag override
- [x] Implement config validation (AC: 2, 5)
  - [x] Add JSON parsing with serde_json
  - [x] Add validation for required fields
  - [x] Add error handling for invalid JSON
- [x] Implement file permissions (AC: 6)
  - [x] Set 0600 permissions on config.json creation
  - [x] Verify permissions on existing files
- [x] Write unit tests
  - [x] Test config loading hierarchy
  - [x] Test environment variable override
  - [x] Test CLI flag override
  - [x] Test invalid JSON handling
  - [x] Test file permissions

### Review Follow-ups (AI)

- [x] [AI-Review][HIGH] Fix hierarchy precedence bug in merge_env_variables - reversed .or() logic (FIXED)
- [x] [AI-Review][HIGH] Add CLI flags --model and --config to integrate config module (FIXED)
- [x] [AI-Review][MEDIUM] Fix InvalidPermissions handling - warn and skip instead of error (FIXED)
- [x] [AI-Review][MEDIUM] Fix test pollution by adding proper cleanup in tests (FIXED)

## Dev Notes

### Architecture Compliance

**Module Structure:**

- Create `src/config/` directory with:
  - `mod.rs` - Public Config API (Config struct, load_config() function)
  - `schema.rs` - Configuration structs with serde derives
  - `loader.rs` - Loading logic with hierarchy implementation
- Follow Rust RFC 430 naming conventions strictly
- Use snake_case for modules and files

**Configuration Hierarchy (Highest to Lowest Precedence):**

1. CLI flags (--model, --config)
2. Environment variables (MINICLAW\_\*, OPENROUTER_API_KEY, TELEGRAM_BOT_TOKEN)
3. Config file (~/.miniclaw/config.json)
4. Default values

**Configuration Schema:**

```rust
// src/config/schema.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_whitelist: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            model: Some("google/gemini-2.5-flash".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        }
    }
}
```

**Error Handling:**

- Use `anyhow` for CLI/main error handling
- Use `thiserror` for library ConfigError enum
- Invalid JSON: Return clear error suggesting `miniclaw onboard`
- Exit code 1 for config errors

**Security Requirements:**

- File permissions must be 0600 (owner read/write only)
- Use `std::fs::set_permissions()` with `Permissions::from_mode(0o600)`
- Never log API keys or tokens (even in debug mode)

### Source Tree Components

**Files to Create:**

- `src/config/mod.rs` - Module entry point, exports Config API
- `src/config/schema.rs` - Config struct with serde derives
- `src/config/loader.rs` - Loading logic (file, env, CLI merge)
- `src/config/loader_test.rs` - Unit tests (or use #[cfg(test)] module)

**Dependencies to Add (Cargo.toml):**

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
dirs = "5.0"  # For home directory detection
```

### Testing Standards

- Co-located tests in `#[cfg(test)]` modules at bottom of each file
- Test each layer of configuration hierarchy independently
- Test precedence ordering (CLI > env > file > defaults)
- Test error cases: missing file, invalid JSON, permission errors
- Use temp directories for test config files

### Project Structure Notes

- Aligns with unified project structure defined in architecture.md
- Config module is foundational - all other modules depend on it
- Creates base directory `~/.miniclaw/` (parent of workspace/)
- Config file location: `~/.miniclaw/config.json` (not in workspace/)

### References

- Architecture: [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- Epic 2: [Source: _bmad-output/planning-artifacts/epics.md#Epic-2-Configuration-&-Workspace-Management]
- Story 2.1: [Source: _bmad-output/planning-artifacts/epics.md#Story-2.1-Configuration-File-Management]
- Rust RFC 430: [Source: _bmad-output/planning-artifacts/architecture.md#Naming-Patterns]

## Dev Agent Record

### Agent Model Used

k2p5

### Debug Log References

No critical issues encountered during implementation.

### Completion Notes List

- Implemented full configuration hierarchy with 4 layers: CLI flags > Environment variables > Config file > Defaults
- Created comprehensive Config struct with serde support for serialization/deserialization
- Implemented ConfigError enum with thiserror for proper error handling
- Added save_config function with 0600 file permissions for security
- All 20 unit tests passing including hierarchy precedence, env override, CLI override, and file permission tests
- Added tempfile as dev dependency for testing
- Used unsafe blocks for environment variable manipulation in tests (Rust 2024 Edition requirement)

### File List

**New Files:**

- `src/config/mod.rs` - Module entry point, exports Config API
- `src/config/schema.rs` - Config struct with serde derives
- `src/config/loader.rs` - Loading logic with hierarchy implementation and unit tests

**Modified Files:**

- `Cargo.toml` - Added dependencies: serde, serde_json, anyhow, thiserror, dirs, tempfile (dev)
- `src/main.rs` - Added `mod config;` to include config module
- `src/cli.rs` - Added `--model` and `--config` CLI flags for override

**Dependencies Added:**

- serde = { version = "1.0", features = ["derive"] }
- serde_json = "1.0"
- anyhow = "1.0"
- thiserror = "2.0"
- dirs = "6.0"
- tempfile = "3" (dev-dependencies)

## Change Log

**2026-02-15:** Story 2.1 - Configuration File Management

- Implemented configuration module with full hierarchy support (CLI > Env > File > Defaults)
- Created Config struct with serde serialization support
- Implemented ConfigError enum with comprehensive error handling
- Added save_config with 0600 file permissions for security
- Created 20 comprehensive unit tests covering all acceptance criteria
- All tests passing successfully

## Status

Status: done
