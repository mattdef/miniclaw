# Story 1.1: Version Command

Status: done

## Story

As a user,
I want to check the miniclaw version from the command line,
so that I can verify my installation and check for updates.

## Acceptance Criteria

1. **Given** miniclaw is installed
   **When** I run `miniclaw version`
   **Then** the system displays the current semantic version (e.g., "miniclaw 0.1.0")
   **And** the exit code is 0

2. **Given** miniclaw is installed
   **When** I run `miniclaw --version`
   **Then** the system displays the same version information
   **And** the exit code is 0

3. **Given** miniclaw is installed
   **When** I run `miniclaw -V`
   **Then** the system displays the same version information
   **And** the exit code is 0

## Tasks / Subtasks

- [x] Initialize Rust project with cargo init (AC: 1, 2, 3)
  - [x] Create Cargo.toml with project metadata
  - [x] Set up basic project structure
  - [x] Configure clap for CLI argument parsing
- [x] Implement version command handler (AC: 1)
  - [x] Create version module
  - [x] Implement version display logic
  - [x] Read version from Cargo.toml
- [x] Implement --version flag support (AC: 2)
  - [x] Configure clap to handle --version flag
  - [x] Ensure consistent output with `version` command
- [x] Implement -V short flag support (AC: 3)
  - [x] Configure clap to handle -V short flag
  - [x] Ensure consistent output
- [x] Add tests for version command (AC: 1, 2, 3)
  - [x] Unit tests for version module
  - [x] Integration tests for CLI flags

## Dev Notes

### Architecture Context

This is the FIRST story of the project and establishes the foundation:

**Technology Stack:**
- Rust 1.80+ Edition 2024 [Source: architecture.md#Technical Constraints]
- CLI framework: clap (standard for Rust CLI tools)
- Zero unsafe code policy enforced [Source: architecture.md#Starter Template Evaluation]

**Project Structure:**
- Single crate with modular architecture
- Top-level modules: config/, chat/, agent/, tools/, memory/, channels/, session/, cron/, providers/
- This story creates the initial `src/main.rs` and version module

**Build Configuration:**
- Edition 2024, Rust 1.80+
- Release profile optimized for binary size (<15MB target) [Source: NFR-P1 in epics.md]
- Zero runtime dependencies [Source: NFR-C3 in epics.md]

### Implementation Details

**Cargo.toml Setup:**
```toml
[package]
name = "miniclaw"
version = "0.1.0"
edition = "2024"
rust-version = "1.80"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

**CLI Structure:**
- Main command: `miniclaw version`
- Global flags: `--version`, `-V`
- Version should be read from Cargo.toml at compile time using `env!("CARGO_PKG_VERSION")`

**Version Output Format:**
```
miniclaw 0.1.0
```

**Exit Codes:**
- 0: Success
- 1: Unknown command (handled by clap)
- 2: Invalid arguments (handled by clap)

### Code Patterns to Establish

1. **CLI Pattern:** Use clap derive macros for declarative CLI definition
2. **Error Handling:** Return appropriate exit codes; use `std::process::exit()`
3. **Testing:** Add `#[cfg(test)]` modules for unit tests
4. **Documentation:** Use rustdoc comments for public APIs

### Project Structure Notes

**Alignment with Unified Structure:**
- `Cargo.toml` - Project manifest
- `src/main.rs` - Entry point with CLI setup
- `src/cli.rs` - CLI module (command definitions)
- `src/commands/` - Command implementations (start with `version.rs`)

**File Organization:**
```
miniclaw/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   └── commands/
│       └── version.rs
└── tests/
    └── version_tests.rs
```

### Testing Requirements

**Unit Tests:**
- Test version string format
- Test CLI argument parsing

**Integration Tests:**
- Test `miniclaw version` command execution
- Test `--version` and `-V` flags
- Verify exit codes

### Performance Considerations

**Cold Start Target:** <100ms [Source: NFR-P3 in epics.md]
- This story establishes the baseline cold start time
- Keep dependencies minimal
- Avoid heavy initialization

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.1: Version Command]
- [Source: _bmad-output/planning-artifacts/architecture.md#Technical Constraints]
- [Source: _bmad-output/planning-artifacts/architecture.md#Starter Template Evaluation]

## Dev Agent Record

### Agent Model Used

k2p5

### Debug Log References

- cargo build: Successfully compiled with clap dependency
- cargo test: All 5 tests passed (1 unit test, 4 integration tests)
- Manual testing: All three version display methods work correctly with exit code 0

### Completion Notes List

✅ **Implementation Complete - 2026-02-14**

**What was implemented:**
1. Updated Cargo.toml with clap dependency and rust-version = "1.85" (required for Edition 2024)
2. Created src/cli.rs with CLI structure using clap derive macros
3. Updated src/main.rs to use the CLI module
4. Created tests/version_tests.rs with 4 integration tests

**Key technical decisions:**
- Used clap's derive macros for declarative CLI definition
- Leveraged `env!("CARGO_PKG_VERSION")` for compile-time version reading
- Implemented version display via `println!` with clean exit code 0
- Added comprehensive test coverage (unit + integration)

**All Acceptance Criteria satisfied:**
- ✅ AC1: `miniclaw version` displays "miniclaw 0.1.0" with exit code 0
- ✅ AC2: `miniclaw --version` displays same version with exit code 0  
- ✅ AC3: `miniclaw -V` displays same version with exit code 0

**Test results:**
- 1 unit test: test_version_string_format (validates semantic version format)
- 4 integration tests: version command, --version flag, -V flag, exit code verification
- All tests pass successfully

### File List

- Cargo.toml (modified: added clap dependency, rust-version)
- Cargo.lock (modified: auto-generated by cargo)
- .gitignore (modified: added Rust default entries)
- src/main.rs (modified: integrated CLI module)
- src/cli.rs (created: CLI structure with version command)
- tests/version_tests.rs (created: integration tests)

### Change Log

- **2026-02-14**: Story implementation complete - Version Command feature delivered
