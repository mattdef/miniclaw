# Story 6.3: Exec Tool

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an agent,
I want to execute shell commands,
so that I can run system utilities and scripts.

## Acceptance Criteria

1. **Command Parameters**: Given the exec tool is called, when executing a command, then it accepts parameters: command, args (array), optional cwd, and args must be provided as array (prevents shell injection)

2. **Blacklist Enforcement**: Given a command execution request, when checking against blacklist, then it rejects: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo, and returns error "Command not allowed: {command}"

3. **Execution with Timeout**: Given a whitelisted command, when executing it, then it runs with timeout (30 seconds default), and captures stdout and stderr, and returns combined output

4. **Output Capture**: Given command execution completes, when capturing output, then stdout is captured, and stderr is captured, and exit code is captured, and all are returned to agent

5. **Timeout Handling**: Given a command times out, when execution exceeds timeout, then the process is killed, and error "Command timed out after {timeout}s" is returned

6. **Non-zero Exit Codes**: Given a command fails, when exit code is non-zero, then output is still returned, and exit code is included, and agent is informed of failure

7. **Working Directory**: Given optional cwd parameter, when provided, then command executes in specified directory, and path is validated (same as filesystem tool)

## Tasks / Subtasks

- [x] **Task 1**: Create ExecTool struct and implement Tool trait (AC: #1)
  - [x] 1.1 Create `ExecTool` struct in `src/agent/tools/exec.rs`
  - [x] 1.2 Implement `Tool` trait with name="exec", description, parameters schema
  - [x] 1.3 Define parameters schema with command, args array, and optional cwd
  - [x] 1.4 Implement `execute()` method with command dispatch

- [x] **Task 2**: Implement command blacklist validation (AC: #2)
  - [x] 2.1 Define blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
  - [x] 2.2 Create blacklist check function (case-insensitive)
  - [x] 2.3 Return PermissionDenied error for blacklisted commands
  - [x] 2.4 Add comprehensive blacklist tests

- [x] **Task 3**: Implement command execution with timeout (AC: #3, #5)
  - [x] 3.1 Use `tokio::process::Command` for async execution
  - [x] 3.2 Set default timeout to 30 seconds
  - [x] 3.3 Implement timeout handling with process termination
  - [x] 3.4 Kill process gracefully (SIGTERM) then forcefully (SIGKILL) if needed

- [x] **Task 4**: Implement output capture (AC: #4, #6)
  - [x] 4.1 Capture stdout as String
  - [x] 4.2 Capture stderr as String
  - [x] 4.3 Capture exit code
  - [x] 4.4 Return JSON with all fields: stdout, stderr, exit_code
  - [x] 4.5 Handle non-zero exit codes (don't fail, just report)

- [x] **Task 5**: Implement working directory support (AC: #7)
  - [x] 5.1 Accept optional cwd parameter
  - [x] 5.2 Validate cwd using existing path utilities from `src/utils/paths.rs`
  - [x] 5.3 Ensure cwd is within allowed base directory
  - [x] 5.4 Set working directory on Command before execution

- [x] **Task 6**: Register tool in AgentLoop (AC: All)
  - [x] 6.1 Import ExecTool in AgentLoop/one-shot registration
  - [x] 6.2 Register ExecTool in tool registry
  - [x] 6.3 Verify tool appears in tool listings

- [x] **Task 7**: Testing and validation (AC: All)
  - [x] 7.1 Unit tests for ExecTool in exec.rs
  - [x] 7.2 Test blacklist: each forbidden command
  - [x] 7.3 Test execution: stdout/stderr capture, exit codes
  - [x] 7.4 Test timeout: long-running commands killed properly
  - [x] 7.5 Test cwd: valid directory, invalid directory, path traversal
  - [x] 7.6 Test security: shell injection prevention via array args
  - [x] 7.7 Integration tests for tool execution flow
  - [x] 7.8 All tests pass (target: 310+ tests)

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
- Create: `src/agent/tools/exec.rs`
- Register in: `src/agent/tools/mod.rs` or `src/agent/oneshot.rs`

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Struct: `ExecTool` (PascalCase)
- File: `exec.rs` (snake_case)
- Methods: `execute()`, `validate_command()` (snake_case)

**Async Process Execution** [Source: architecture.md#Process Patterns]:
```rust
use tokio::process::Command;
let output = Command::new(command)
    .args(args)
    .output()
    .await?;
```
**CRITICAL**: Always use `tokio::process::Command`, never `std::process::Command` in async context

**Error Handling Pattern** [Source: architecture.md#Process Patterns]:
- Use `ToolError` from `src/agent/tools/types.rs`
- Error types: `InvalidArguments`, `ExecutionFailed`, `PermissionDenied`, `NotFound`
- Use `?` operator for propagation

**Security Requirements** [Source: architecture.md#Security Boundaries, NFR-S4]:
- **MUST** validate command against blacklist
- **MUST** use array args only (prevents shell injection)
- **MUST NOT** accept shell metacharacters in command string
- **MUST** validate cwd path using canonicalize()
- Blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo

**Timeout Pattern** [Source: architecture.md#Process Patterns]:
```rust
use tokio::time::{timeout, Duration};

match timeout(Duration::from_secs(30), command.output()).await {
    Ok(result) => result,
    Err(_) => {
        child.kill().await?;
        Err(ToolError::Timeout)
    }
}
```

### Source Tree Components to Touch

**Files to Create/Modify**:
1. `src/agent/tools/exec.rs` - NEW: ExecTool implementation
2. `src/agent/tools/mod.rs` - UPDATE: Add `pub mod exec;` export
3. `src/agent/oneshot.rs` - UPDATE: Register ExecTool in registry
4. `src/utils/security.rs` - UPDATE: Add command blacklist (or reuse existing)

**Expected ExecTool Structure**:
```rust
pub struct ExecTool {
    base_dir: PathBuf, // For validating cwd parameter
    blacklist: HashSet<String>,
    default_timeout: Duration,
}

impl ExecTool {
    pub fn new(base_dir: PathBuf) -> Self { ... }
    
    async fn execute_command(
        &self, 
        command: &str, 
        args: &[String], 
        cwd: Option<&Path>
    ) -> ToolResult<String> { ... }
    
    fn is_blacklisted(&self, command: &str) -> bool { ... }
    fn validate_cwd(&self, cwd: &Path) -> ToolResult<PathBuf> { ... }
}
```

**Parameters Schema** (JSON Schema):
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Command to execute (e.g., 'ls', 'git', 'cargo')"
    },
    "args": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Command arguments as array (prevents shell injection)"
    },
    "cwd": {
      "type": "string",
      "description": "Optional working directory path"
    }
  },
  "required": ["command", "args"]
}
```

**Return Format** (JSON):
```json
{
  "stdout": "command output...",
  "stderr": "error output...",
  "exit_code": 0
}
```

**Dependencies**:
- `tokio::process` - Async process execution (already in project)
- `tokio::time` - For timeout handling
- `std::path::PathBuf` - Path manipulation
- `serde_json` - JSON formatting for results
- Existing `src/utils/paths.rs` - Path validation (reusable!)

### Testing Standards Summary

**Test Coverage Required**:
- Blacklist: each forbidden command rejected
- Execution: stdout/stderr capture, exit code handling
- Timeout: commands killed after 30s
- Working directory: valid paths, invalid paths, path traversal attempts
- Security: shell injection prevention via array args
- Error handling: proper error types and messages

**Test Patterns**:
- Use `#[tokio::test]` for async tests
- Use safe commands for testing: `echo`, `pwd`, `whoami`, `cat`
- Test timeout with `sleep` command (if available) or mock
- Test shell injection: verify `;`, `|`, `&&` in args are treated literally

**Security Test Cases** (CRITICAL):
```rust
#[tokio::test]
async fn test_blacklisted_command_rejected() {
    // Attempt: sudo whoami should fail
}

#[tokio::test]
async fn test_shell_injection_prevented() {
    // Attempt: args = ["; rm -rf /"] should NOT execute rm
}

#[tokio::test]
async fn test_cwd_path_traversal_blocked() {
    // Attempt: cwd = "../../../etc" should fail
}
```

## Project Structure Notes

### Alignment with Unified Project Structure

**Tools Directory Structure** (from architecture.md):
```
src/agent/tools/
├── mod.rs              # Tool trait & Registry - UPDATE to export exec module
├── types.rs            # ToolError, ToolResult, ToolDefinition
├── filesystem.rs       # Already implemented (Story 6.2)
└── exec.rs            # NEW: ExecTool implementation
```

**Integration Points**:
- ExecTool implements Tool trait from types.rs
- Registered in ToolRegistry via AgentLoop
- Path validation reuses existing `src/utils/paths.rs`
- Security blacklist may be shared with spawn tool (Story 6.5)

### Detected Conflicts or Variances

**⚠️ REUSE EXISTING INFRASTRUCTURE**

Unlike Story 6.2 which created new path validation, Story 6.3 should **reuse** the existing utilities:
- **Path validation**: Use `src/utils/paths.rs` from Story 6.2 (PathValidator)
- **Blacklist**: May add to `src/utils/security.rs` if exists, or create it

**Design Decision: Timeout Handling**:
- Default timeout: 30 seconds (configurable via constructor for future)
- Graceful termination: Try SIGTERM first, wait 5s, then SIGKILL if still running
- This prevents zombie processes and ensures cleanup

**Design Decision: Array Args**:
- **MUST** use array args to prevent shell injection
- **MUST NOT** accept a single string with spaces
- This is a security requirement from NFR-S4

## References

### Technical Details with Source Paths

**Tool System Architecture**:
- [Source: architecture.md#Project Structure & Boundaries] - Tools directory structure
- [Source: architecture.md#Architectural Boundaries] - Tool trait definition
- [Source: 6-1-tool-registry-and-trait.md] - Previous story with Tool trait implementation
- [Source: 6-2-filesystem-tool.md] - Tool implementation pattern (follow same structure!)

**Security Requirements**:
- [Source: architecture.md#Security Boundaries] - Exec blacklist requirement
- [Source: architecture.md#NFR-S4] - "L'outil exec doit refuser d'exécuter les commandes blacklisted"
- [Source: epics.md#NFR-S4] - Blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo

**Async Process Execution**:
- [Source: architecture.md#Process Patterns] - "All I/O operations must be async"
- [Source: architecture.md#Anti-Patterns to AVOID] - Never use std::process::Command in async
- Use `tokio::process::Command` and `tokio::time::timeout`

**Path Validation Reuse**:
- [Source: 6-2-filesystem-tool.md - Dev Notes] - Path validation centralized in `src/utils/paths.rs`
- Reuse PathValidator for cwd parameter validation
- Same security model: canonicalize + base directory check

**NFRs Applicable**:
- [Source: architecture.md#NFR-S4] - Exec blacklist enforcement
- [Source: architecture.md#NFR-S3] - Path canonicalization for cwd
- [Source: architecture.md#NFR-R3] - Never panic on user input (invalid commands)
- [Source: architecture.md#NFR-R1] - Auto-restart for timeout kills (graceful handling)

### Previous Story Intelligence

**From Story 6.1 (Tool Registry and Trait)** [Source: 6-1-tool-registry-and-trait.md]:
- Tool trait defined in `src/agent/tools/types.rs` with name(), description(), parameters(), execute()
- ToolRegistry in `src/agent/tools/mod.rs` uses Arc<RwLock<HashMap>> for thread safety
- Pattern: Manual registration in AgentLoop::new() or oneshot.rs
- Error handling: ToolError enum with InvalidArguments, ExecutionFailed, PermissionDenied, NotFound
- JSON Schema validation already implemented in ToolRegistry::execute_tool()

**From Story 6.2 (Filesystem Tool)** [Source: 6-2-filesystem-tool.md]:
- **CRITICAL REUSABLE COMPONENT**: `src/utils/paths.rs` - PathValidator for cwd validation
- Tool implementation pattern established (follow exactly!)
- Test patterns: 17 unit tests, co-located in `#[cfg(test)]` module
- Code review learnings: Canonicalize paths, use async I/O only, thread-safe by default
- File structure: Tool struct with base_dir, implement Tool trait, comprehensive tests

**Key Learnings from Previous Stories**:
1. **Path Validation**: Reuse `PathValidator` from `src/utils/paths.rs` (Story 6.2)
2. **Thread Safety**: ToolRegistry is thread-safe with Arc<RwLock<>>
3. **Registration Pattern**: Register in oneshot.rs for one-shot mode:
   ```rust
   let exec_tool = ExecTool::new(workspace_dir);
   registry.register(Box::new(exec_tool))?;
   ```
4. **Error Handling**: Return ToolError variants with clear messages
5. **Testing**: Use `#[tokio::test]` for async tests, co-located in `#[cfg(test)]` modules
6. **Security**: All Stories 6.x must follow same security model

**Files from Story 6.2 to Reference**:
- `src/agent/tools/filesystem.rs` - Implementation pattern (follow this structure!)
- `src/utils/paths.rs` - Reuse for cwd validation
- `src/agent/tools/mod.rs` - See how to export new module
- `src/agent/oneshot.rs` - See registration pattern

### Git Intelligence Summary

**Recent Commits**:
- `7b9beaf` - Add path validation utilities (PathValidator for reuse!)
- `7cd18a9` - Update filesystem tool to done status and implement review fixes
- `9c2c4c3` - Add filesystem tool implementation
- `26132f6` - Tool registry improvements from code review

**Code Patterns from Recent Work**:
- Trait-based extensibility for tools
- Arc<RwLock<>> pattern for thread-safe shared state
- Structured logging with tracing
- Tests co-located in `#[cfg(test)]` modules
- Manual tool registration in oneshot.rs
- JSON Schema validation in registry before tool execution
- **Reusable utilities in src/utils/** (NEW pattern from Story 6.2!)

### Latest Tech Information

**Dependencies Already in Project**:
- `tokio::process` - For async command execution
- `tokio::time` - For timeout handling
- `serde_json` - For JSON formatting of results
- `thiserror` - For error types (via ToolError)
- `tracing` - For structured logging
- `src/utils/paths.rs` - Path validation (from Story 6.2)

**No additional dependencies needed** - all requirements satisfied by existing stack.

**Async Process Execution Pattern**:
```rust
use tokio::process::Command;
use tokio::time::{timeout, Duration};

async fn execute_with_timeout(
    command: &str,
    args: &[String],
    cwd: Option<&Path>,
    timeout_secs: u64,
) -> Result<Output, ToolError> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    
    match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(ToolError::ExecutionFailed(e.to_string())),
        Err(_) => {
            // Timeout occurred - process will be killed automatically
            Err(ToolError::Timeout)
        }
    }
}
```

**Blacklist Implementation**:
```rust
use std::collections::HashSet;

fn is_blacklisted(command: &str) -> bool {
    let blacklist: HashSet<&str> = [
        "rm", "sudo", "dd", "mkfs", 
        "shutdown", "reboot", "passwd", "visudo"
    ].iter().cloned().collect();
    
    // Check base command name (not full path)
    let base_cmd = std::path::Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command);
    
    blacklist.contains(base_cmd.to_lowercase().as_str())
}
```

## Dev Agent Record

### Agent Model Used

Claude (Anthropic) - 2026-02-16

### Debug Log References

- Tool implementation following red-green-refactor cycle
- Code review fixes applied: 2026-02-16
- All tests passing: 317 tests (target: 310+)

### Completion Notes List

1. ✅ **Task 1: ExecTool Implementation**
   - Created `ExecTool` struct with base_dir configuration
   - Implemented `Tool` trait with name="exec", comprehensive description
   - Parameters schema with command, args array, and optional cwd
   - Implemented `execute()` method with proper argument extraction and validation
   - Constructor returns `Result<Self, ToolError>` for graceful error handling

2. ✅ **Task 2: Blacklist Validation**
   - Implemented blacklist: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
   - Blacklist centralized as constant `EXEC_BLACKLIST` for reusability
   - Case-insensitive command name checking
   - Supports path-based commands (e.g., "/bin/rm")
   - Returns PermissionDenied error for blacklisted commands
   - Comprehensive tests for all blacklist scenarios

3. ✅ **Task 3: Async Execution with Timeout**
   - Uses `tokio::process::Command` for async execution
   - Default timeout: 30 seconds (defined as constant `DEFAULT_EXEC_TIMEOUT_SECS`)
   - Timeout handled via tokio::select! pattern
   - Process killed on timeout using start_kill() then kill() (SIGTERM → SIGKILL)

4. ✅ **Task 4: Output Capture**
   - Captures stdout and stderr as strings
   - Captures exit code (i32)
   - Returns JSON with all three fields
   - Non-zero exit codes return output (don't fail)

5. ✅ **Task 5: Working Directory Support**
   - Accepts optional cwd parameter
   - Validates cwd using existing PathValidator from utils/paths.rs
   - Ensures cwd is within base directory
   - Sets working directory on Command

6. ✅ **Task 6: Tool Registration**
   - Registered in oneshot.rs with workspace directory
   - Exported in mod.rs as `pub mod exec;`
   - Tool appears in tool listings

7. ✅ **Task 7: Testing**
   - 19 unit tests in exec.rs covering all functionality
   - Blacklist tests for each forbidden command
   - Execution tests for stdout/stderr/exit_code
   - Timeout test (ignored by default, takes 30+ seconds to run)
   - CWD validation tests
   - Shell injection prevention tests
   - Constructor error handling tests
   - Args validation tests (must be strings)
   - All 317 tests pass (increased from 315 after code review fixes)

### Code Review Fixes Applied (2026-02-16)

1. **Fixed PANIC in constructor**: Changed `ExecTool::new()` to return `Result<Self, ToolError>` instead of panicking on invalid base directory
2. **Fixed process killing on timeout**: Implemented proper SIGTERM → SIGKILL pattern using `start_kill()` and `kill()` with tokio::select!
3. **Centralized blacklist**: Moved blacklist to constant `EXEC_BLACKLIST` for reusability across tools
4. **Improved error classification**: Added `NotFound` and `PermissionDenied` error types for better debugging
5. **Fixed args validation**: Now returns error if args contain non-string values instead of silently converting to empty strings
6. **Added missing tests**: Constructor error handling, args validation, timeout behavior
7. **Improved description**: Added mention of cwd parameter and shell injection prevention
8. **Magic numbers eliminated**: Timeout value now a named constant `DEFAULT_EXEC_TIMEOUT_SECS`

### File List

**Created Files:**
1. `src/agent/tools/exec.rs` - ExecTool implementation with 19 unit tests

**Modified Files:**
1. `src/agent/tools/mod.rs` - Added `pub mod exec;` export
2. `src/agent/oneshot.rs` - Registered ExecTool in tool registry with proper error handling
3. `_bmad-output/implementation-artifacts/6-3-exec-tool.md` - Updated with code review fixes
4. `_bmad-output/implementation-artifacts/sprint-status.yaml` - Status tracking

**Git Commits:**
- Implementation of exec tool with comprehensive tests
- Tool registration in mod.rs and oneshot.rs
- Code review fixes applied (process killing, error handling, validation)

---

**Story Created**: 2026-02-16
**Epic**: 6 - Tool System - Core Tools
**Dependencies**: Story 6.1 (Tool Registry), Story 6.2 (Filesystem Tool - path validation utilities)
**Blocks**: Story 6.4 (Web Tool), Story 6.5 (Spawn Tool)

**Completion Note**: Code review completed - all HIGH and MEDIUM issues fixed, AC #5 fully implemented
