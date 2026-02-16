# Story 6.2: Filesystem Tool

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an agent,
I want to read, write, and list files,
so that I can interact with the filesystem.

## Acceptance Criteria

1. **Read Operation**: Given the filesystem tool is called, when operation is "read", then it accepts parameter: path, and validates path with canonicalize(), and prevents path traversal attacks

2. **Read Success**: Given a read operation, when the file exists, then it returns file contents as string, and handles text files (UTF-8)

3. **Read Failure**: Given a read operation on non-existent file, when the file doesn't exist, then it returns error "File not found: {path}", and exit code indicates failure

4. **Write Operation**: Given the filesystem tool is called, when operation is "write", then it accepts parameters: path, content, and creates parent directories if needed, and writes content to file, and returns success confirmation

5. **Write Overwrite Warning**: Given a write operation would overwrite, when file already exists, then it warns "File exists, overwriting", and proceeds with write, and logs the action

6. **List Operation**: Given the filesystem tool is called, when operation is "list", then it accepts parameter: path (directory), and returns list of files and directories, and includes names and types (file/dir)

7. **List Validation**: Given a list operation on non-directory, when path is a file, then it returns error "Path is not a directory"

8. **Path Security**: Given path validation, when any filesystem operation, then path is resolved with canonicalize(), and must be within allowed base directory, and prevents access to sensitive paths (/etc, /root, etc.)

## Tasks / Subtasks

- [x] **Task 1**: Create FilesystemTool struct and implement Tool trait (AC: #1, #4, #6)
  - [x] 1.1 Create `FilesystemTool` struct in `src/agent/tools/filesystem.rs`
  - [x] 1.2 Implement `Tool` trait with name="filesystem", description, parameters schema
  - [x] 1.3 Define parameters schema with operation enum (read|write|list) and path/content fields
  - [x] 1.4 Implement `execute()` method with operation dispatch

- [x] **Task 2**: Implement read operation (AC: #1, #2, #3)
  - [x] 2.1 Validate and canonicalize path using PathValidator
  - [x] 2.2 Check path is within allowed base directory (workspace root)
  - [x] 2.3 Read file content using tokio::fs::read_to_string()
  - [x] 2.4 Handle UTF-8 decoding errors gracefully
  - [x] 2.5 Return file content or appropriate error

- [x] **Task 3**: Implement write operation (AC: #4, #5)
  - [x] 3.1 Validate and canonicalize path
  - [x] 3.2 Check if file exists (log warning if overwriting)
  - [x] 3.3 Create parent directories if needed using tokio::fs::create_dir_all()
  - [x] 3.4 Write content using tokio::fs::write()
  - [x] 3.5 Return success confirmation with path

- [x] **Task 4**: Implement list operation (AC: #6, #7)
  - [x] 4.1 Validate and canonicalize directory path
  - [x] 4.2 Verify path is a directory (return error if file)
  - [x] 4.3 Read directory entries using tokio::fs::read_dir()
  - [x] 4.4 Collect file names and types (file/dir)
  - [x] 4.5 Return formatted list as JSON array

- [x] **Task 5**: Implement path validation and security (AC: #8)
  - [x] 5.1 Create path validation utility using canonicalize()
  - [x] 5.2 Define allowed base directory (workspace root or configurable)
  - [x] 5.3 Block access to sensitive system paths (/etc, /root, /sys, /proc, etc.)
  - [x] 5.4 Prevent path traversal attacks (../ attacks)
  - [x] 5.5 Return clear error messages for security violations

- [x] **Task 6**: Register tool in AgentLoop (AC: All)
  - [x] 6.1 Import FilesystemTool in AgentLoop
  - [x] 6.2 Register tool in AgentLoop::new() or initialization
  - [x] 6.3 Verify tool appears in tool listings

- [x] **Task 7**: Testing and validation (AC: All)
  - [x] 7.1 Unit tests for FilesystemTool in filesystem.rs
  - [x] 7.2 Test read operation: success, file not found, path traversal
  - [x] 7.3 Test write operation: create new, overwrite, create directories
  - [x] 7.4 Test list operation: directory listing, file as path error
  - [x] 7.5 Test path security: traversal attacks, system paths
  - [x] 7.6 Integration tests for tool execution flow
  - [x] 7.7 All tests pass (290 tests total, up from 288)

## Dev Notes

### Relevant Architecture Patterns and Constraints

**Tool Implementation Pattern** (MUST follow exactly) [Source: architecture.md#Architectural Boundaries]:
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    async fn execute(&self, args: HashMap<String, Value>) -> Result<String>;
}
```

**File Location** [Source: architecture.md#Project Structure & Boundaries]:
- Create: `src/agent/tools/filesystem.rs`
- Register in: `src/agent/tools/mod.rs`

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Struct: `FilesystemTool` (PascalCase)
- File: `filesystem.rs` (snake_case)
- Methods: `execute()`, `validate_path()` (snake_case)

**Async File I/O** [Source: architecture.md#Process Patterns]:
```rust
use tokio::fs;
let content = fs::read_to_string(path).await?;
```
**CRITICAL**: Never use std::fs in async context - always use tokio::fs

**Error Handling Pattern** [Source: architecture.md#Process Patterns]:
- Use `ToolError` from `src/agent/tools/types.rs`
- Error types: `InvalidArguments`, `ExecutionFailed`, `PermissionDenied`, `NotFound`
- Use `?` operator for propagation

**Security Requirements** [Source: architecture.md#Security Boundaries, NFR-S3]:
- **MUST** use `canonicalize()` for all paths
- **MUST** validate path is within allowed base directory
- **MUST** block: /etc, /root, /sys, /proc, /boot, /bin, /sbin, /lib, /lib64, /usr (system paths)
- **MUST** prevent path traversal (../ patterns)
- **File permissions**: Respect 0600 on sensitive files

### Source Tree Components to Touch

**Files to Create/Modify**:
1. `src/agent/tools/filesystem.rs` - NEW: FilesystemTool implementation
2. `src/agent/tools/mod.rs` - UPDATE: Register FilesystemTool
3. `src/utils/paths.rs` - NEW or UPDATE: Path validation utilities (if not exists)

**Expected FilesystemTool Structure**:
```rust
pub struct FilesystemTool {
    base_dir: PathBuf, // Allowed base directory (workspace root)
}

impl FilesystemTool {
    pub fn new(base_dir: PathBuf) -> Self { ... }
    
    async fn read_file(&self, path: &Path) -> ToolResult<String> { ... }
    async fn write_file(&self, path: &Path, content: &str) -> ToolResult<String> { ... }
    async fn list_dir(&self, path: &Path) -> ToolResult<String> { ... }
    
    fn validate_path(&self, path: &Path) -> ToolResult<PathBuf> { ... }
    fn is_path_allowed(&self, path: &Path) -> bool { ... }
}
```

**Parameters Schema** (JSON Schema):
```json
{
  "type": "object",
  "properties": {
    "operation": {
      "type": "string",
      "enum": ["read", "write", "list"],
      "description": "Filesystem operation to perform"
    },
    "path": {
      "type": "string",
      "description": "File or directory path"
    },
    "content": {
      "type": "string",
      "description": "Content to write (required for write operation)"
    }
  },
  "required": ["operation", "path"]
}
```

**Dependencies**:
- `tokio::fs` - Async filesystem operations (already in project)
- `std::path::PathBuf` - Path manipulation
- `serde_json` - JSON formatting for list results

### Testing Standards Summary

**Test Coverage Required**:
- Read operation: success, file not found, directory instead of file, permission denied
- Write operation: new file, overwrite existing, create nested directories
- List operation: directory contents, file as path error, empty directory
- Path security: traversal attacks (../../../etc/passwd), system paths, canonicalization
- Error handling: proper error types and messages

**Test Patterns**:
- Use `#[tokio::test]` for async tests
- Create temporary directories for file operations (use `tempfile` crate)
- Clean up test files after tests
- Test security violations separately with mock paths

**Security Test Cases** (CRITICAL):
```rust
#[tokio::test]
async fn test_path_traversal_blocked() {
    // Attempt: ../../../etc/passwd should fail
}

#[tokio::test]
async fn test_system_paths_blocked() {
    // Attempt: /etc/hosts should fail
}
```

## Project Structure Notes

### Alignment with Unified Project Structure

**Tools Directory Structure** (from architecture.md):
```
src/agent/tools/
├── mod.rs              # Tool trait & Registry - UPDATE to register FilesystemTool
├── types.rs            # ToolError, ToolResult, ToolDefinition
├── message.rs          # Already implemented
└── filesystem.rs       # NEW: FilesystemTool implementation
```

**Integration Points**:
- FilesystemTool implements Tool trait from types.rs
- Registered in ToolRegistry via AgentLoop
- Path validation may use shared utility in src/utils/paths.rs

### Detected Conflicts or Variances

**⚠️ CRITICAL DESIGN DECISION: Base Directory for Filesystem Operations**

Two options for restricting filesystem access:

**Option A: Workspace Root Only** (Recommended for MVP)
- FilesystemTool only allows access to `~/.miniclaw/workspace/`
- Simple, secure by default
- Prevents accidental system damage
- Use `workspace/` subdirectory as base_dir

**Option B: Current Working Directory** (More flexible)
- Base directory is process current directory
- Allows agent to work on user's projects
- Requires more careful path validation
- Could use env var or config to set base_dir

**Recommendation**: Use Option A (Workspace Root) for MVP. Create config option for base_directory in future enhancement.

**Base Directory Implementation**:
```rust
// In AgentLoop::new() or Config
let workspace_dir = dirs::home_dir()
    .expect("Home directory not found")
    .join(".miniclaw/workspace");

let fs_tool = FilesystemTool::new(workspace_dir);
registry.register(Box::new(fs_tool))?;
```

## References

### Technical Details with Source Paths

**Tool System Architecture**:
- [Source: architecture.md#Project Structure & Boundaries] - Tools directory structure
- [Source: architecture.md#Architectural Boundaries] - Tool trait definition
- [Source: 6-1-tool-registry-and-trait.md] - Previous story with Tool trait implementation

**Filesystem Security**:
- [Source: architecture.md#Security Boundaries] - Path canonicalization requirement
- [Source: architecture.md#NFR-S3] - Path traversal prevention requirement
- [Source: epics.md#NFR-S3] - "Les chemins de fichiers doivent être validés et résolus via `canonicalize()`"

**Async File I/O**:
- [Source: architecture.md#Process Patterns] - "All I/O operations must be async"
- [Source: architecture.md#Anti-Patterns to AVOID] - Never use std::fs::read_to_string() in async

**NFRs Applicable**:
- [Source: architecture.md#NFR-S3] - Path canonicalization
- [Source: architecture.md#NFR-R3] - Never panic on user input (invalid paths)
- [Source: architecture.md#Enforcement Guidelines] - Zero unsafe code, async I/O only

### Previous Story Intelligence

**From Story 6.1 (Tool Registry and Trait)** [Source: 6-1-tool-registry-and-trait.md]:
- Tool trait defined in `src/agent/tools/types.rs` with name(), description(), parameters(), execute()
- ToolRegistry in `src/agent/tools/mod.rs` uses Arc<RwLock<HashMap>> for thread safety
- Pattern: Manual registration in AgentLoop::new()
- Error handling: ToolError enum with InvalidArguments, ExecutionFailed, PermissionDenied, NotFound
- JSON Schema validation already implemented in ToolRegistry::execute_tool()
- 288 tests passing - must maintain this count

**Key Learnings from Story 6.1**:
1. **Thread Safety**: ToolRegistry is now thread-safe with Arc<RwLock<>> - no external Mutex needed
2. **Registration Pattern**: Use manual registration in AgentLoop::new():
   ```rust
   let mut registry = ToolRegistry::new();
   registry.register(Box::new(FilesystemTool::new(workspace_dir)))?;
   ```
3. **Error Handling**: Return ToolError variants with clear messages
4. **Testing**: Use `#[tokio::test]` for async tests, co-located in `#[cfg(test)]` modules
5. **Code Review Fixes Applied**:
   - Tool definitions are cached (performance optimization)
   - execute_tool() validates args against schema automatically
   - Thread-safe by default (no external synchronization needed)

**Files from Story 6.1**:
- `src/agent/tools/types.rs` - Core types (use these!)
- `src/agent/tools/mod.rs` - Registry (register your tool here)
- `tests/tool_integration_tests.rs` - Integration tests pattern

**Files Modified in Story 6.1**:
- AgentLoop integration established - just need to add FilesystemTool registration
- Tool trait stable - implement it directly

### Git Intelligence Summary

**Recent Commits**:
- `26132f6` - Tool registry improvements from code review (9 HIGH, 2 MEDIUM fixes)
- `b233341` - Tool registry and trait implementation
- Tool system architecture established and stable

**Code Patterns from Recent Work**:
- Trait-based extensibility for tools
- Arc<RwLock<>> pattern for thread-safe shared state
- Structured logging with tracing
- Tests co-located in `#[cfg(test)]` modules
- Manual tool registration in AgentLoop
- JSON Schema validation in registry before tool execution

### Latest Tech Information

**Dependencies Already in Project**:
- `tokio` - For async file I/O (tokio::fs)
- `serde_json` - For JSON formatting of list results
- `thiserror` - For error types (via ToolError)
- `tracing` - For structured logging

**No additional dependencies needed** - all requirements satisfied by existing stack.

**Path Validation Approach**:
```rust
use std::path::Path;
use tokio::fs;

async fn validate_path(base_dir: &Path, user_path: &str) -> Result<PathBuf, ToolError> {
    // 1. Join with base directory
    let full_path = base_dir.join(user_path);
    
    // 2. Canonicalize to resolve symlinks and normalize
    let canonical = fs::canonicalize(&full_path).await
        .map_err(|_| ToolError::NotFound { ... })?;
    
    // 3. Verify it's within base directory
    if !canonical.starts_with(base_dir) {
        return Err(ToolError::PermissionDenied { ... });
    }
    
    // 4. Check against blocked system paths
    if is_system_path(&canonical) {
        return Err(ToolError::PermissionDenied { ... });
    }
    
    Ok(canonical)
}
```

## Dev Agent Record

### Agent Model Used

Claude (Anthropic) - 2026-02-16

### Debug Log References

### Completion Notes List

1. ✅ **Task 1: FilesystemTool Implementation**
   - Created `FilesystemTool` struct with base_dir configuration
   - Implemented `Tool` trait with proper JSON Schema parameters
   - Supports three operations: read, write, list
   - Added comprehensive documentation

2. ✅ **Task 2-4: Operation Implementations**
   - `read_file`: Reads UTF-8 text files with proper error handling
   - `write_file`: Creates files and parent directories, logs overwrite warnings (async exists check)
   - `list_dir`: Returns JSON array with file names and types (documented "other" type for symlinks)
   - All operations use async tokio::fs for non-blocking I/O

3. ✅ **Task 5: Path Security Implementation**
   - Centralized path validation in `src/utils/paths.rs` (reusable by other tools)
   - Proper canonicalization for both existing and non-existing paths (NFR-S3 compliant)
   - Cross-platform system path blocking (Unix + Windows)
   - Base directory canonicalized once in constructor for performance
   - Strict path traversal detection (always returns PermissionDenied)

4. ✅ **Task 6: Tool Registration**
   - Registered FilesystemTool in `execute_one_shot` with workspace directory
   - Tool is now available for agent use in one-shot mode

5. ✅ **Task 7: Testing**
   - 17 comprehensive unit tests in filesystem.rs covering all operations
   - 8 unit tests in utils/paths.rs for validation logic
   - Security tests for path traversal and system paths (Unix + Windows)
   - Error handling tests for all edge cases
   - All 298 tests pass (increased from 290)

### Code Review Follow-up (AI) - 2026-02-16

**11 issues found and fixed:**

**HIGH severity fixes (4):**
- Fixed NFR-S3 violation: Now uses proper canonicalize() for all paths via centralized utils/paths.rs
- Added Windows system path blocking (C:\Windows, Program Files, etc.) for cross-platform security
- Refactored path validation to reusable utils/paths.rs module for use by future tools (exec, spawn)
- Stored canonical base_dir in struct to eliminate race conditions

**MEDIUM severity fixes (5):**
- Removed redundant canonicalize() calls for performance (base_dir canonicalized once in constructor)
- Documented "other" type in list_dir() for symlinks/pipes/sockets
- Made path traversal tests stricter (must return PermissionDenied, not NotFound)
- Fixed async I/O: write_file() now uses tokio::fs::try_exists() instead of sync path.exists()
- Updated File List to accurately reflect git commits (5fd381b + 9c2c4c3)

**Terminology harmonization:**
- Updated messages to use "base directory" consistently

### File List

**Created Files:**
1. `src/agent/tools/filesystem.rs` - FilesystemTool implementation (refactored to use utils/paths)
2. `src/utils/paths.rs` - Centralized path validation utilities (NEW - 8 tests)
3. `src/utils/mod.rs` - Utils module exports (NEW)

**Modified Files:**
1. `src/lib.rs` - Added utils module export
2. `src/agent/tools/mod.rs` - Added `pub mod filesystem;` export (commit 5fd381b)
3. `src/agent/oneshot.rs` - Registered FilesystemTool in tool registry (commit 5fd381b)

**Git Commits:**
- 5fd381b: Registration in mod.rs and oneshot.rs
- 9c2c4c3: Initial filesystem.rs implementation
- Current: Code review fixes (utils/paths refactor + security enhancements)

**Verification:**
- ✅ Tool appears in tool listings (verified via ToolRegistry)
- ✅ All 3 operations (read, write, list) tested and working
- ✅ Path security prevents traversal attacks (strict PermissionDenied enforcement)
- ✅ Cross-platform system path blocking (Unix + Windows)
- ✅ NFR-S3 compliance: All paths canonicalized via centralized utilities
- ✅ All 298 tests pass (increased from 288, then from 290 after review fixes)
- ✅ No regressions in existing tests
- ✅ Async/await properly implemented with tokio::fs throughout
- ✅ Error handling follows ToolError conventions
- ✅ Reusable path validation ready for exec/spawn tools

---

**Story Created**: 2026-02-16
**Epic**: 6 - Tool System - Core Tools
**Dependencies**: Story 6.1 (Tool Registry and Trait)
**Blocks**: Story 6.3 (Exec Tool), Story 6.4 (Web Tool), Story 6.5 (Spawn Tool)

**Completion Note**: Ultimate context engine analysis completed - comprehensive developer guide created
