# Story 2.3: Workspace Structure Creation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want miniclaw to create a complete workspace structure,
So that I can customize my agent's personality and capabilities.

## Acceptance Criteria

1. **Workspace Directory Creation** (AC: 1)
   - **Given** onboarding is running
   - **When** the system creates the workspace
   - **Then** it creates `~/.miniclaw/workspace/` directory

2. **SOUL.md Creation** (AC: 2)
   - **Given** workspace directory creation
   - **When** initializing the workspace
   - **Then** it creates `workspace/SOUL.md` with default agent personality template
   - **And** the file contains sections for agent name, personality traits, and communication style

3. **AGENTS.md Creation** (AC: 3)
   - **Given** workspace initialization
   - **When** creating agent instructions
   - **Then** it creates `workspace/AGENTS.md` with default agent behavior guidelines
   - **And** includes sections for available tools and usage patterns

4. **USER.md Creation** (AC: 4)
   - **Given** workspace initialization
   - **When** setting up user profile
   - **Then** it creates `workspace/USER.md` with placeholder for user information
   - **And** includes sections for preferences and context

5. **TOOLS.md Creation** (AC: 5)
   - **Given** workspace initialization
   - **When** creating tool documentation
   - **Then** it creates `workspace/TOOLS.md` documenting all available tools
   - **And** includes usage examples for each tool
   - **And** describes parameter formats

6. **HEARTBEAT.md Creation** (AC: 6)
   - **Given** workspace initialization
   - **When** setting up periodic tasks
   - **Then** it creates `workspace/HEARTBEAT.md` for scheduled task definitions
   - **And** includes example cron jobs
   - **And** explains the heartbeat system

7. **File Recovery** (AC: 7)
   - **Given** all workspace files are created
   - **When** a file is missing or corrupted
   - **Then** the system can recreate it individually
   - **And** preserves other existing files

8. **Context Loading** (AC: 8)
   - **Given** workspace files exist
   - **When** the agent assembles context
   - **Then** it loads SOUL.md as system personality
   - **And** loads AGENTS.md as behavioral guidelines
   - **And** loads USER.md for user context
   - **And** loads TOOLS.md for tool documentation
   - **And** loads HEARTBEAT.md for scheduled tasks

## Tasks / Subtasks

- [x] Create workspace module structure (AC: 1)
  - [x] Create `src/workspace/mod.rs` with workspace initialization logic
  - [x] Create `src/workspace/templates.rs` with default content templates
  - [x] Export public functions from workspace module
- [x] Implement workspace directory creation (AC: 1)
  - [x] Create `~/.miniclaw/workspace/` directory
  - [x] Handle directory creation errors gracefully
  - [x] Set appropriate permissions (0755 for directories)
- [x] Implement SOUL.md template and creation (AC: 2)
  - [x] Define default SOUL.md content template
  - [x] Create file at `~/.miniclaw/workspace/SOUL.md`
  - [x] Include sections: agent name, personality, communication style
- [x] Implement AGENTS.md template and creation (AC: 3)
  - [x] Define default AGENTS.md content template
  - [x] Create file at `~/.miniclaw/workspace/AGENTS.md`
  - [x] Include sections: behavior guidelines, available tools
- [x] Implement USER.md template and creation (AC: 4)
  - [x] Define default USER.md content template
  - [x] Create file at `~/.miniclaw/workspace/USER.md`
  - [x] Include sections: user preferences, context placeholders
- [x] Implement TOOLS.md template and creation (AC: 5)
  - [x] Define default TOOLS.md content template
  - [x] Create file at `~/.miniclaw/workspace/TOOLS.md`
  - [x] Document all available tools with examples
- [x] Implement HEARTBEAT.md template and creation (AC: 6)
  - [x] Define default HEARTBEAT.md content template
  - [x] Create file at `~/.miniclaw/workspace/HEARTBEAT.md`
  - [x] Include example cron jobs and heartbeat explanations
- [x] Implement file recovery functionality (AC: 7)
  - [x] Create function to check for missing workspace files
  - [x] Implement individual file recreation
  - [x] Preserve existing files during recovery
- [x] Implement context loading interface (AC: 8)
  - [x] Create functions to load each workspace file
  - [x] Return structured content for context builder
  - [x] Handle missing files gracefully
- [x] Integrate with onboarding command (AC: 1-8)
  - [x] Call workspace initialization from `run_onboarding()`
  - [x] Add verbose logging for workspace creation
  - [x] Display completion summary
- [x] Write unit tests
  - [x] Test workspace directory creation
  - [x] Test each file creation (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md)
  - [x] Test file recovery functionality
  - [x] Test context loading functions
  - [x] Test error handling for filesystem issues

## Dev Notes

### Architecture Compliance

**Module Structure:**

- Create `src/workspace/` directory with module structure
- Create `src/workspace/mod.rs` as module interface
- Create `src/workspace/templates.rs` for default content templates
- Follow Rust RFC 430 naming conventions strictly

**Workspace Module Design:**

```rust
// src/workspace/mod.rs
pub mod templates;

use std::path::PathBuf;
use anyhow::Result;

/// Initialize the complete workspace structure
pub fn initialize_workspace(base_path: &PathBuf, verbose: bool) -> Result<()> {
    // Create workspace directory
    // Create all markdown files from templates
    // Log operations if verbose
}

/// Load workspace files for context assembly
pub struct WorkspaceContext {
    pub soul: String,
    pub agents: String,
    pub user: String,
    pub tools: String,
    pub heartbeat: String,
}

pub fn load_workspace_context(workspace_path: &PathBuf) -> Result<WorkspaceContext> {
    // Load all markdown files
    // Return structured context
}

/// Check and recreate missing workspace files
pub fn repair_workspace(base_path: &PathBuf, verbose: bool) -> Result<()> {
    // Check each file exists
    // Recreate missing ones
    // Preserve existing files
}
```

**Template Structure:**

```rust
// src/workspace/templates.rs

pub const DEFAULT_SOUL: &str = r#"# Agent Soul

## Name
miniclaw

## Personality Traits
- Helpful and efficient
- Technical but accessible
- Proactive in suggesting improvements

## Communication Style
- Clear and concise
- Uses appropriate technical terminology
- Maintains professional but friendly tone
"#;

// Similar constants for AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md
```

**File Permissions:**

- Directories: 0755 (rwxr-xr-x)
- Markdown files: 0644 (rw-r--r--)
- Consistent with Unix permissions used in Story 2.1 (config.json uses 0600)

**Error Handling:**

- Use `anyhow` for CLI/main error handling
- Use `thiserror` for workspace-specific errors
- Provide clear error messages for filesystem issues
- Handle missing parent directories (create recursively)

**Integration with Onboarding:**

- Reuse workspace path detection from Story 2.2
- Call `initialize_workspace()` from `run_onboarding()`
- Add verbose logging showing each file created
- Display summary of workspace structure on completion

### Source Tree Components

**Files to Create:**

- `src/workspace/mod.rs` - Workspace module interface
- `src/workspace/templates.rs` - Default content templates

**Files to Modify:**

- `src/config/onboard.rs` - Call workspace initialization
- `src/config/mod.rs` - Add `pub mod workspace;` if workspace is under config, or create as top-level module
- `Cargo.toml` - No new dependencies needed (use existing std::fs, anyhow)

**Workspace Directory Structure Created:**

```
~/.miniclaw/
├── config.json              # Created in Story 2.1/2.2
└── workspace/               # Created in this story
    ├── SOUL.md              # Agent personality
    ├── AGENTS.md            # Agent behavior guidelines
    ├── USER.md              # User profile and preferences
    ├── TOOLS.md             # Tool documentation
    ├── HEARTBEAT.md         # Scheduled tasks
    ├── skills/              # Created in Story 2.4
    ├── sessions/            # Created in Story 2.5
    └── memory/              # Created in Story 8.x
```

### Testing Standards

- Co-located tests in `#[cfg(test)]` module at bottom of each file
- Integration tests in `tests/workspace_tests.rs`
- Test scenarios:
  - Fresh workspace initialization
  - Individual file recreation
  - Context loading with all files present
  - Context loading with missing files
  - File permissions verification
  - Error handling for read-only filesystem

### Project Structure Notes

- Workspace directory is the foundation for Stories 2.4 (skills), 2.5 (sessions), and Epic 8 (memory)
- File content templates should be easily customizable by users
- Context loading interface will be used by Epic 5 (Agent Core) Context Builder
- HEARTBEAT.md provides foundation for Epic 7 (Cron/Scheduling tools)
- Aligns with unified project structure defined in architecture.md

### Previous Story Intelligence

**From Story 2.1 (Configuration File Management):**

- Config directory: `~/.miniclaw/` established
- Pattern for module creation: create `src/config/schema.rs`, `src/config/loader.rs`
- File permission handling: use `std::fs::set_permissions()` with `Permissions::from_mode(0o600)`
- Error handling pattern: `anyhow` for CLI, `thiserror` for library errors

**From Story 2.2 (Interactive Onboarding Command):**

- Workspace base path: `~/.miniclaw/` determined in Story 2.2
- Onboarding flow: check existing → create directory → prompt user → save config
- Integration point: add workspace initialization call after config creation
- Verbose logging pattern: `debug!("Creating directory: {:?}", path);`
- File structure: `src/config/onboard.rs` with interactive prompts
- Review feedback from 2.2:
  - Avoid `std::process::exit()` - return `Result<bool>` instead
  - Add integration tests in `tests/` directory
  - Use `Path::is_absolute()` for path validation
  - All user-facing prompts need confirmation step

**Git Intelligence - Recent Patterns:**

- Recent commits show modular development approach (config module first, then workspace)
- Pattern: Add module structure → implement core functions → add tests → integrate
- Commit 11bfaa4 shows config module implementation (358 lines)
- Commit ebc0bd1 shows CLI integration
- Test files added separately: `tests/help_tests.rs`, `tests/onboard_tests.rs`

**Key Implementation Details from 2.2:**

```rust
// From src/config/onboard.rs - reuse patterns
use std::path::Path;
use anyhow::{Context, Result};

// Directory creation pattern
std::fs::create_dir_all(&workspace_path)
    .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_path))?;

// Permission setting pattern (Unix)
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(&workspace_path, permissions)?;
}
```

### References

- Architecture: [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- Architecture Data Persistence: [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- Epic 2: [Source: _bmad-output/planning-artifacts/epics.md#Epic-2-Configuration-&-Workspace-Management]
- Story 2.2 (Previous): [Source: _bmad-output/implementation-artifacts/2-2-interactive-onboarding-command.md]
- Story 2.3 Requirements: [Source: _bmad-output/planning-artifacts/epics.md#Story-2.3-Workspace-Structure-Creation]
- File Naming Conventions: [Source: _bmad-output/planning-artifacts/architecture.md#Naming-Patterns]
- JSON Session Format: [Source: _bmad-output/planning-artifacts/architecture.md#Format-Patterns]

## Dev Agent Record

### Agent Model Used

kimi-for-coding/k2p5

### Debug Log References

- Implementation completed successfully with all tests passing
- 17 unit tests in workspace module (all passing)
- 7 integration tests in tests/workspace_tests.rs (all passing)

### Completion Notes List

- **2026-02-15:** Implemented complete workspace module structure
  - Created `src/workspace/mod.rs` with WorkspaceContext struct and core functions:
    - `initialize_workspace()`: Creates workspace directory and all markdown files
    - `load_workspace_context()`: Loads all workspace files into structured context
    - `repair_workspace()`: Recreates missing files while preserving existing ones
    - `workspace_file_exists()`, `get_workspace_file_path()`: Utility functions
  
- Created `src/workspace/templates.rs` with default content for all 5 files:
  - SOUL.md: Agent personality with traits, identity, and communication style
  - AGENTS.md: Behavior guidelines and available tools documentation
  - USER.md: User profile template with preferences and custom instructions
  - TOOLS.md: Comprehensive tool documentation with examples and parameters
  - HEARTBEAT.md: Scheduled task system with cron expression documentation
  
- **Integration with onboarding:**
  - Modified `src/config/onboard.rs` to call `workspace::initialize_workspace()`
  - Added workspace module import and updated `create_workspace_structure()` function
  - Verbose logging shows each file and directory created
  - Updated onboard test to verify all workspace files are created
  
- **Testing:**
  - 17 unit tests covering all workspace functions
  - 7 integration tests verifying end-to-end functionality
  - Tests cover: directory creation, file creation, preservation of existing files,
    file recovery, context loading, error handling, and permissions (Unix)
  
- **Architecture compliance:**
  - Follows Rust RFC 430 naming conventions
  - Uses anyhow for error handling with context
  - Proper file permissions on Unix (0755 for directories, 0644 for files)
  - All workspace files preserved during re-onboarding (no overwrites)
  
- **All Acceptance Criteria satisfied:**
  - AC1: Workspace directory creation ✓
  - AC2-6: All markdown files created with proper content ✓
  - AC7: File recovery functionality (repair_workspace) ✓
  - AC8: Context loading interface (WorkspaceContext) ✓

### File List

**New Files:**

- `src/workspace/mod.rs` - Workspace module interface
- `src/workspace/templates.rs` - Default content templates
- `src/config/onboard.rs` - Interactive onboarding logic
- `tests/workspace_tests.rs` - Integration tests for workspace functionality
- `tests/onboard_tests.rs` - Integration tests for onboarding command

**Modified Files:**

- `src/cli.rs` - Remove `std::process::exit()`, return `Result`, handle onboarding
- `src/main.rs` - Add workspace module declaration, handle `cli::run()` result
- `src/config/mod.rs` - Export workspace module and onboarding
- `Cargo.toml` - Dependencies for interactive UI and error handling
- `Cargo.lock` - Updated dependency tree

## Senior Developer Review (AI)

**Reviewer:** AI Code Review Agent  
**Date:** 2026-02-15  
**Status:** Complete

### Review Findings

- **HIGH**: Git State - Staged all untracked files and uncommitted changes.
- **HIGH**: Exit Calls - Refactored `src/cli.rs` to return `Result` instead of using `std::process::exit()`.
- **MEDIUM**: Documentation - Updated File List to include all modified files (`main.rs`, `cli.rs`, `Cargo.toml`).
- **MEDIUM**: Logging - Switched `src/workspace/mod.rs` from `println!` to `tracing`.
- **LOW**: Error Handling - Aligned workspace module with `thiserror` for library-level errors.

### Review Conclusion

Implementation is now fully compliant with project standards and architecture guidelines. All untracked files have been staged, and previous review feedback regarding `std::process::exit()` has been addressed.

---

## Change Log

**2026-02-15:** Story 2.3 - Workspace Structure Creation

- Created comprehensive story file with 8 acceptance criteria
- Defined 10 task groups with subtasks
- Included architecture compliance notes from architecture.md
- Referenced previous story 2.2 implementation patterns and review feedback
- Documented workspace structure and context loading interface
- Ultimate context engine analysis completed - comprehensive developer guide created

**2026-02-15:** Story 2.3 Implementation Complete

- Implemented complete workspace module with all functionality
- Created src/workspace/mod.rs with WorkspaceContext and core functions
- Created src/workspace/templates.rs with default content for all 5 files
- Integrated with onboarding command in src/config/onboard.rs
- Added workspace module declaration in src/main.rs
- 17 unit tests in workspace module (all passing)
- 7 integration tests in tests/workspace_tests.rs (all passing)
- All 8 acceptance criteria satisfied
- Files created during onboarding:
  - ~/.miniclaw/workspace/SOUL.md (agent personality)
  - ~/.miniclaw/workspace/AGENTS.md (behavior guidelines)
  - ~/.miniclaw/workspace/USER.md (user profile)
  - ~/.miniclaw/workspace/TOOLS.md (tool documentation)
  - ~/.miniclaw/workspace/HEARTBEAT.md (scheduled tasks)
- Architecture compliance: Rust RFC 430, anyhow errors, proper permissions

**2026-02-15:** Story 2.3 Code Review Fixes

- Refactored CLI to avoid `std::process::exit()`
- Aligned workspace error handling with `thiserror`
- Updated logging to use `tracing` instead of `println!`
- Fixed incomplete File List and documented integration tests

Status: done

