# Story 1.2: CLI Framework Setup

Status: done

## Story

As a user,
I want a robust CLI interface with proper argument parsing,
so that I can interact with miniclaw intuitively and get helpful error messages.

## Acceptance Criteria

1. **Given** miniclaw is installed
   **When** I run `miniclaw` without any arguments
   **Then** the system displays the help message with available commands
   **And** the exit code is 0

2. **Given** miniclaw is installed
   **When** I run `miniclaw invalid_command`
   **Then** the system displays an error message "unknown command: invalid_command"
   **And** suggests valid commands
   **And** the exit code is 1

3. **Given** miniclaw is installed
   **When** I run a command with invalid flags (e.g., `miniclaw version --invalid`)
   **Then** the system displays an error about the unrecognized flag
   **And** shows correct usage
   **And** the exit code is 2

4. **Given** any CLI operation
   **When** an error occurs
   **Then** the error message is clear and actionable
   **And** error output goes to stderr
   **And** normal output goes to stdout

## Tasks / Subtasks

- [x] Configure default help behavior (AC: 1)
  - [x] Set up clap to show help when no subcommand is provided
  - [x] Ensure help displays all available commands
  - [x] Verify exit code is 0 for help display

- [x] Implement unknown command error handling (AC: 2)
  - [x] Configure clap to catch unknown subcommands
  - [x] Display "unknown command: {command}" error message
  - [x] Show suggestions for valid commands
  - [x] Ensure exit code is 1

- [x] Implement invalid flag error handling (AC: 3)
  - [x] Configure clap to validate flags per subcommand
  - [x] Display error for unrecognized flags
  - [x] Show correct usage information
  - [x] Ensure exit code is 2

- [x] Implement stdout/stderr separation (AC: 4)
  - [x] Configure all normal output to go to stdout
  - [x] Configure all error messages to go to stderr
  - [x] Add integration tests to verify stream separation

- [x] Add comprehensive tests (AC: 1, 2, 3, 4)
  - [x] Unit tests for CLI argument parsing
  - [x] Integration tests for help display
  - [x] Integration tests for error handling
  - [x] Integration tests for exit codes

## Dev Notes

### Architecture Context

**Technology Stack:**
- Rust 1.85+ Edition 2024 [Source: architecture.md#Technical Constraints]
- CLI framework: clap with derive macros (already in use from Story 1.1)
- Zero unsafe code policy enforced [Source: architecture.md#Starter Template Evaluation]

**Current State (from Story 1.1):**
- `Cargo.toml` already has clap dependency configured
- `src/cli.rs` exists with basic CLI structure
- `src/main.rs` has entry point with CLI module integration
- Version command already implemented and working

**Exit Code Standards:**
- `0` : Success (help displayed, command executed successfully)
- `1` : Unknown command or execution error
- `2` : Invalid arguments or unrecognized flags
[Source: architecture.md#Process Patterns]

**Error Handling Pattern:**
```rust
// Use ? for propagation
let config = load_config().await?;

// Convert errors at boundaries
impl From<clap::Error> for CliError {
    fn from(e: clap::Error) -> Self {
        CliError::InvalidArguments(e.to_string())
    }
}
```
[Source: architecture.md#Process Patterns]

### Implementation Details

**CLI Structure Enhancement:**
The current `src/cli.rs` has basic structure. Story 1.2 extends it to handle:
1. Default help when no subcommand provided
2. Custom error messages for unknown commands
3. Flag validation with proper exit codes
4. Stdout/stderr separation

**Clap Configuration:**
```rust
#[derive(Parser)]
#[command(name = "miniclaw")]
#[command(about = "Your AI agent for edge hardware")]
#[command(version)]
#[command(arg_required_else_help = true)] // Show help if no args provided
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,
    // Future commands will be added here
}
```

**Error Message Requirements:**
- Unknown command: "unknown command: {command}"
- Invalid flag: clap's built-in error formatting (already includes usage)
- Clear and actionable messages (no technical jargon for users)
- All errors go to stderr

**Stream Separation:**
```rust
// Normal output → stdout
println!("Help message");

// Error output → stderr  
eprintln!("Error: unknown command");
std::process::exit(1);
```

### Project Structure Notes

**Files to Modify:**
- `src/cli.rs` - Enhance CLI structure with error handling configuration
- `src/main.rs` - Update to handle exit codes and stream separation

**Files to Create:**
- `tests/cli_tests.rs` - Integration tests for CLI behavior (or extend existing tests/version_tests.rs)

**Alignment with Unified Structure:**
- ✅ Follows modular architecture (src/cli.rs separate from main.rs)
- ✅ Uses clap derive macros as established in Story 1.1
- ✅ Maintains snake_case naming (cli.rs, main.rs)
- ✅ Tests co-located in tests/ directory

### Testing Requirements

**Unit Tests (in src/cli.rs):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_structure() {
        // Verify CLI struct can be instantiated
    }
}
```

**Integration Tests:**
1. **Help Display Test:**
   - Run `miniclaw` without args
   - Verify exit code 0
   - Verify stdout contains help text
   - Verify stderr is empty

2. **Unknown Command Test:**
   - Run `miniclaw invalid_command`
   - Verify exit code 1
   - Verify stderr contains "unknown command"
   - Verify stdout is empty or contains suggestions

3. **Invalid Flag Test:**
   - Run `miniclaw version --invalid`
   - Verify exit code 2
   - Verify stderr contains error about unrecognized flag
   - Verify stderr contains usage information

4. **Stream Separation Test:**
   - Test that errors go to stderr
   - Test that normal output goes to stdout

**Test Commands:**
```bash
cargo test --test cli_tests
cargo test --lib  # For unit tests
```

### Performance Considerations

**Cold Start Target:** <100ms [Source: NFR-P3 in epics.md]
- clap derive macros are compile-time, no runtime overhead
- Minimal initialization for CLI parsing
- Fast error path (fail fast pattern)

### Previous Story Intelligence (from Story 1.1)

**Learnings from Version Command Implementation:**
1. **Clap Setup Pattern:** Using derive macros with `#[derive(Parser)]` works well
2. **Version Reading:** `env!("CARGO_PKG_VERSION")` for compile-time version
3. **Testing Pattern:** Separate integration tests in tests/ directory
4. **Exit Codes:** Already established pattern with version command (exit 0 on success)
5. **File Organization:** src/cli.rs for CLI structure, src/main.rs for entry point

**Established Patterns to Continue:**
- Derive macro approach for CLI definition
- Integration tests for CLI behavior verification
- Clear separation between CLI structure (cli.rs) and execution (main.rs)
- Documentation comments that become help text

**Potential Issues to Avoid:**
- Don't use `std::process::exit()` in library code (only in main.rs)
- Don't mix print! and eprintln! for the same logical output
- Don't suppress clap's built-in error handling (it's quite good)

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.2: CLI Framework Setup]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns]
- [Source: _bmad-output/implementation-artifacts/1-1-version-command.md] - Previous story learnings
- [Previous Story Files: Cargo.toml, src/cli.rs, src/main.rs, tests/version_tests.rs]

## Dev Agent Record

### Agent Model Used

k2p5

### Debug Log References

- cargo test --test cli_tests: All 12 tests passed
- cargo test: All 17 tests passed (1 unit + 12 cli + 4 version tests)

### Completion Notes List

✅ **Implementation Complete - 2026-02-14**

**What was implemented:**

1. **Default help behavior (AC 1):**
   - Modified `src/cli.rs` to detect when no arguments are provided
   - Shows custom help message with available commands
   - Exits with code 0 as required

2. **Unknown command error handling (AC 2):**
   - Implemented error parsing in `Cli::try_parse()` error handler
   - Detects "unrecognized subcommand" errors from clap
   - Displays "error: unknown command: {command}" to stderr
   - Exits with code 1

3. **Invalid flag error handling (AC 3):**
   - Detects "unexpected argument" errors from clap
   - Displays clap's built-in error message with usage information
   - Exits with code 2

4. **Stdout/stderr separation (AC 4):**
   - All normal output (help, version) goes to stdout via `println!`
   - All errors go to stderr via `eprintln!` and `e.print()`
   - Integration tests verify stream separation

5. **Version flag handling:**
   - Manual check for `--version` and `-V` flags before clap parsing
   - Ensures version display works correctly with exit code 0

**Key technical decisions:**
- Used `Cli::try_parse()` instead of `Cli::parse()` to catch and handle errors
- Manually check for version flags before parsing to maintain compatibility with Story 1.1
- Custom help output for no-args case to ensure exit code 0 (clap's default behavior exits with code 2)
- Error message parsing to differentiate between unknown commands (exit 1) and invalid flags (exit 2)

**All Acceptance Criteria satisfied:**
- ✅ AC1: `miniclaw` without args shows help and exits with code 0
- ✅ AC2: `miniclaw invalid_command` shows "unknown command" error and exits with code 1
- ✅ AC3: `miniclaw version --invalid` shows flag error and exits with code 2
- ✅ AC4: Normal output to stdout, errors to stderr

**Test results:**
- 12 new integration tests in tests/cli_tests.rs
- 1 unit test in src/cli.rs (existing)
- 4 version tests in tests/version_tests.rs (existing)
- All 17 tests pass successfully

### File List

- src/cli.rs (modified: enhanced CLI structure with error handling and exit codes)
- tests/cli_tests.rs (created: 12 integration tests for CLI behavior)

### Change Log

- **2026-02-14**: Story 1.2 implementation complete - CLI Framework Setup feature delivered
- **2026-02-14**: Code review fixes applied:
  - Fixed duplicate error output for unknown commands
  - Added suggestions for valid commands in error message
  - Added unit tests for extract_command_from_error function

