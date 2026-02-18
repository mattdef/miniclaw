---
title: "Codebase Optimization - Phases 1–4"
slug: "codebase-optimization-phases-1-4"
created: "2026-02-18"
status: "completed"
stepsCompleted: [1, 2, 3, 4]
tech_stack:
  - "Rust (Edition 2024, MSRV 1.85)"
  - "Tokio (async runtime)"
  - "tracing (structured logging)"
  - "serde_json (serialization)"
  - "thiserror / anyhow (error handling)"
  - "clap (CLI)"
files_to_modify:
  - "src/agent/tools/exec.rs"
  - "src/agent/tools/spawn.rs"
  - "src/agent/tools/mod.rs"
  - "src/agent/tools/security.rs (new)"
  - "src/memory/ranker.rs"
  - "src/memory/long_term.rs"
  - "src/memory/daily_notes.rs"
  - "src/memory/short_term.rs"
  - "src/memory/types.rs"
  - "src/chat/hub.rs"
  - "src/session/manager.rs"
  - "src/workspace/mod.rs"
  - "src/workspace/templates.rs"
  - "src/agent/agent_loop.rs"
  - "src/config/loader.rs"
  - "src/config/onboard.rs"
  - "src/providers/factory.rs"
  - "src/skills/manager.rs"
  - "src/gateway.rs"
  - "src/utils/mod.rs"
  - "src/utils/fs.rs (new)"
  - "tests/version_tests.rs (delete)"
  - "tests/cli_tests.rs"
  - "tests/gateway_tests.rs"
  - "tests/session_integration_tests.rs"
code_patterns:
  - "thiserror for domain errors, anyhow for application-level errors"
  - "Arc<RwLock<T>> for shared mutable state (do NOT wrap mpsc::Receiver)"
  - "async-trait for async traits"
  - "tokio::fs for all async file I/O (not std::fs in async context)"
  - "tracing macros (info!, debug!, error!, warn!) with structured fields"
  - "cargo fmt + clippy -D warnings enforced in CI"
test_patterns:
  - "Unit tests inline in #[cfg(test)] modules"
  - "Integration tests in /tests/ directory using assert_cmd + predicates"
  - "tempfile crate for temporary file/dir creation in tests"
  - "tokio::test for async test functions"
  - 'env!("CARGO_PKG_VERSION") for version strings in tests (not hardcoded)'
---

# Tech-Spec: Codebase Optimization - Phases 1–4

**Created:** 2026-02-18

## Overview

### Problem Statement

The miniclaw codebase (~23,500 lines, 73 Rust source files) contains duplicated code, dead code, async/performance anti-patterns, and redundant tests identified during a full audit in February 2026. These issues increase maintenance burden, introduce risk of drift between copies, and degrade runtime performance (unnecessary allocations, sequential saves, overly broad locks).

### Solution

A 4-phase refactoring ordered by increasing risk: quick wins (direct removals) → code deduplication (shared helpers) → architecture improvements (abstractions, builder patterns) → test consolidation and coverage. Each phase is independently verifiable via `cargo test --all` and `cargo clippy`. No observable user-facing behavior changes.

### Scope

**In Scope:**

- Phase 1 — Quick wins (~2h): dead code, suppressions, blacklist deduplication, memory score optimization, duplicate constants, hardcoded version in tests
- Phase 2 — Refactoring (~4h): shared helpers, fix `RwLock<Receiver>` anti-pattern (Option B: single-owner), memory cache with `Arc<Vec<...>>`, `utils/fs.rs`, `MemoryError::storage_failed()`, session helpers
- Phase 3 — Architecture (~6h): unified `MemoryEntry`, `ApiKeyProviderConfig` trait, parallel session saves, dirty-tracking, `AgentLoop` builder, `ProviderConfig::set_default_model()`, config display helpers
- Phase 4 — Tests (~3h): remove duplicate test files, consolidate session tests, add missing tests (CLI errors, gateway, signal handlers)
- Public module interfaces may be freely modified (internal CLI binary, no external consumers)

**Out of Scope:**

- New features or runtime behavior changes
- Modifications to PRD, epics, or product roadmap
- Third-party dependency updates (audit doc section 8)
- User documentation / CHANGELOG

## Context for Development

### Codebase Patterns

- **Error handling**: `thiserror` for typed domain errors (e.g. `MemoryError`, `ProviderError`), `anyhow` + `.context()` for application-level propagation. Custom `Result<T>` type aliases per module.
- **Async**: Tokio runtime. All file I/O must use `tokio::fs` (not `std::fs`). `Arc<RwLock<T>>` for shared state — **never wrap `mpsc::Receiver`** (receivers are single-consumer by design).
- **Logging**: `tracing` macros with structured fields (`info!(field = value, "message")`). Log to stderr only.
- **Testing**: Unit tests inline (`#[cfg(test)] mod tests`), integration tests in `/tests/`. Uses `assert_cmd`, `predicates`, `tempfile`. Async tests use `#[tokio::test]`. Version strings must use `env!("CARGO_PKG_VERSION")`.
- **Modules**: `mod.rs` re-exports sub-modules. New modules must be declared in parent `mod.rs`.
- **Traits**: `async-trait` for async trait methods. `Arc<dyn Trait>` for dependency injection.

### Files to Reference

| File                                 | Purpose                                                                                           |
| ------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `docs/OPTIMIZE_PROJECT.md`           | Full audit — source of truth for all tasks (1247 lines)                                           |
| `src/agent/tools/exec.rs`            | EXEC_BLACKLIST, validate_cwd, is_blacklisted — duplicates to consolidate                          |
| `src/agent/tools/spawn.rs`           | SPAWN_BLACKLIST, validate_cwd, is_blacklisted — duplicates to consolidate                         |
| `src/agent/tools/mod.rs`             | ToolRegistry — where new security.rs module is declared                                           |
| `src/memory/ranker.rs`               | calculate_score() — unnecessary allocation on every call                                          |
| `src/memory/long_term.rs`            | blocking exists(), StorageFailed pattern, MemoryEntry                                             |
| `src/memory/daily_notes.rs`          | DailyNoteEntry struct, uses types::MemoryEntry already                                            |
| `src/memory/short_term.rs`           | Separate MemoryEntry struct (content + timestamp only)                                            |
| `src/memory/types.rs`                | Canonical MemoryEntry (with memory_type + file_path), MemoryError                                 |
| `src/chat/hub.rs`                    | RwLock<Receiver> anti-pattern — inbound_rx and outbound_rx fields                                 |
| `src/session/manager.rs`             | Sequential saves, no dirty tracking — two SessionManager instances                                |
| `src/workspace/mod.rs`               | Local WORKSPACE_FILES const — duplicate of templates::WORKSPACE_FILES                             |
| `src/workspace/templates.rs`         | Canonical WORKSPACE*FILES + all DEFAULT*\* consts                                                 |
| `src/agent/agent_loop.rs`            | 4 nearly-identical constructors (new, with_inbound_receiver, with_model, with_model_and_receiver) |
| `src/config/loader.rs`               | apply_cli_model_override match-per-provider, 3 deprecated-field warn blocks                       |
| `src/config/onboard.rs`              | Dead prompt_api_key(), display duplication in confirm/display/save                                |
| `src/providers/factory.rs`           | ProviderConfig enum, 4 config structs, default_model() getter exists                              |
| `src/skills/manager.rs`              | Deprecated \_built_in_tools param (line 355)                                                      |
| `src/gateway.rs`                     | Two SessionManager instances (comment: "future refactor to unify")                                |
| `src/utils/mod.rs`                   | Exports: circuit_breaker, error, paths, security — add fs module here                             |
| `tests/version_tests.rs`             | 4 tests, 100% duplicate of first 4 in cli_tests.rs — DELETE                                       |
| `tests/cli_tests.rs`                 | 11 tests, hardcoded "0.1.0" in 3 tests — fix to env!()                                            |
| `tests/session_integration_tests.rs` | Session persistence integration tests                                                             |

### Technical Decisions

- **RwLock fix**: Option B (single-owner) — `ChatHub` will own `inbound_rx: mpsc::Receiver<InboundMessage>` and `outbound_rx: mpsc::Receiver<OutboundMessage>` directly. The `recv_inbound()` and `recv_outbound()` methods take `&mut self`. Tests referencing `hub.inbound_rx.write().await` must be updated accordingly.
- **Public interfaces**: Freely modifiable (internal CLI binary, no external consumers).
- **Execution order**: Phases 1→2→3→4, `cargo test --all` after each phase.
- **New files**: `src/agent/tools/security.rs` (shared blacklist), `src/utils/fs.rs` (async file helpers).
- **Ollama exclusion from ApiKeyProviderConfig**: Ollama has no `api_key` field — the trait applies only to API-key-based providers (OpenRouter, OpenAI, Kimi).

## Implementation Plan

### Tasks

#### Phase 1 — Quick Wins

- [ ] Task 1: Remove dead `prompt_api_key()` function from `onboard.rs`
  - File: `src/config/onboard.rs`
  - Action: Delete the entire `prompt_api_key()` function and its `#[allow(dead_code)]` attribute. Verify no callers exist with `cargo check`.
  - Notes: The function already has a comment "now replaced by prompt_provider_selection". Safe to delete.

- [ ] Task 2: Remove `_built_in_tools` deprecated parameter from `SkillManager`
  - File: `src/skills/manager.rs`
  - Action: Remove the `_built_in_tools: &[String]` parameter from the function at line 355. Update all call sites. Run `cargo check` to find all callers.
  - Notes: Parameter is prefixed with `_`, indicating intentional non-use. All callers must be updated.

- [ ] Task 3: Create `src/agent/tools/security.rs` with shared blacklist and helpers
  - File: `src/agent/tools/security.rs` (new)
  - Action: Create a new module containing:
    1. `pub const COMMAND_BLACKLIST: &[&str]` — merged list from both `EXEC_BLACKLIST` and `SPAWN_BLACKLIST` (they are identical)
    2. `pub fn is_blacklisted(command: &str) -> bool` — shared implementation
    3. `pub fn validate_cwd(cwd: &Option<String>) -> Result<Option<PathBuf>, ToolError>` — shared implementation extracted from both `exec.rs` and `spawn.rs`
  - Notes: Import `ToolError` (or the appropriate error type) from the tools module. Both `validate_cwd` implementations use the same match block.

- [ ] Task 4: Declare `security` module in `src/agent/tools/mod.rs`
  - File: `src/agent/tools/mod.rs`
  - Action: Add `pub mod security;` to the module declarations. Export `COMMAND_BLACKLIST`, `is_blacklisted`, and `validate_cwd` via `pub use security::...` if needed by other modules.

- [ ] Task 5: Replace duplicated blacklist/validate_cwd in `exec.rs` and `spawn.rs`
  - Files: `src/agent/tools/exec.rs`, `src/agent/tools/spawn.rs`
  - Action: Delete `EXEC_BLACKLIST`/`SPAWN_BLACKLIST` consts, delete local `is_blacklisted()` and `validate_cwd()` functions. Replace all usages with calls to `super::security::is_blacklisted()` and `super::security::validate_cwd()`.
  - Notes: Run `cargo test --all` after this task.

- [ ] Task 6: Remove duplicate `WORKSPACE_FILES` const from `workspace/mod.rs`
  - File: `src/workspace/mod.rs`
  - Action: Delete the local `WORKSPACE_FILES` const (lines 32–38). Update all references within `mod.rs` to use `templates::WORKSPACE_FILES` directly.
  - Notes: `templates::WORKSPACE_FILES` at `src/workspace/templates.rs:581-587` is the canonical source.

- [ ] Task 7: Fix `calculate_score()` allocation in `ranker.rs`
  - File: `src/memory/ranker.rs`
  - Action: Change the `calculate_score()` function to accept the pre-lowercased content as a `&str` parameter, OR cache the lowercased string outside the scoring loop. Specifically: move `content.to_lowercase()` out of the per-call path — compute it once per entry before passing to `calculate_score()`, or change the function signature to take `content_lower: &str`.
  - Notes: This is a hot path — called for every memory entry during ranking. The fix eliminates one `String` allocation per call.

- [ ] Task 8: Fix hardcoded version strings in `tests/cli_tests.rs`
  - File: `tests/cli_tests.rs`
  - Action: Replace all 3 occurrences of the hardcoded string `"0.1.0"` with `env!("CARGO_PKG_VERSION")`. Use `contains()` or `assert_cmd` predicates with the env macro value.
  - Notes: These tests will break on every version bump otherwise.

- [ ] Task 9: Delete `tests/version_tests.rs`
  - File: `tests/version_tests.rs`
  - Action: Delete the file entirely. All 4 tests it contains (test_version_command, test_version_flag_long, test_version_flag_short, test_version_exit_code) are already present in `tests/cli_tests.rs`.
  - Notes: Run `cargo test --all` to confirm no tests are lost.

- [ ] Task 10: Run full validation after Phase 1
  - Action: Run `cargo test --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check`. Fix any issues before proceeding to Phase 2.

#### Phase 2 — Refactoring

- [ ] Task 11: Fix `RwLock<Receiver>` anti-pattern in `ChatHub`
  - File: `src/chat/hub.rs`
  - Action: Change `inbound_rx` field from `Arc<RwLock<mpsc::Receiver<InboundMessage>>>` to `mpsc::Receiver<InboundMessage>`. Change `outbound_rx` field from `Arc<RwLock<mpsc::Receiver<OutboundMessage>>>` to `mpsc::Receiver<OutboundMessage>`. Update `recv_inbound()` to take `&mut self` and call `self.inbound_rx.recv().await` directly. Update `recv_outbound()` similarly. Update constructors and any test code that accessed `hub.inbound_rx.write().await`.
  - Notes: `mpsc::Receiver` is single-consumer — wrapping in `Arc<RwLock<>>` is an anti-pattern. The `write()` lock was being used to receive, which defeats the purpose of `RwLock`. After this change, the caller that owns `ChatHub` must hold a `&mut` ref to call recv methods.

- [ ] Task 12: Add `MemoryError::storage_failed()` constructor helper
  - File: `src/memory/types.rs`
  - Action: Add a `pub fn storage_failed(operation: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> MemoryError` constructor function (or an `impl MemoryError` method) that constructs `MemoryError::StorageFailed { operation: operation.into(), source: Box::new(source) }` (or whatever the actual variant shape is). Then update `long_term.rs` and `daily_notes.rs` to use this helper instead of constructing the variant inline.
  - Notes: Check the exact `StorageFailed` variant shape in `memory/types.rs` before implementing.

- [ ] Task 13: Fix blocking `file_path.exists()` in `long_term.rs`
  - File: `src/memory/long_term.rs`
  - Action: Replace the synchronous `file_path.exists()` call (line 29) with `tokio::fs::try_exists(&file_path).await.unwrap_or(false)` or `tokio::fs::metadata(&file_path).await.is_ok()`. Add `use tokio::fs;` if not already imported.
  - Notes: Blocking calls in async context block the Tokio executor thread. `tokio::fs::try_exists` is the idiomatic replacement.

- [ ] Task 14: Create `src/utils/fs.rs` with shared async file helpers
  - File: `src/utils/fs.rs` (new)
  - Action: Create a new utility module with at minimum:
    1. `pub async fn file_exists(path: &Path) -> bool` — wraps `tokio::fs::try_exists` with `unwrap_or(false)`
    2. `pub async fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()>` — if the pattern is used in multiple places (check `session/persistence.rs`)
  - Notes: Keep it minimal — only extract patterns that are actually duplicated across 2+ files.

- [ ] Task 15: Declare `fs` module in `src/utils/mod.rs`
  - File: `src/utils/mod.rs`
  - Action: Add `pub mod fs;` alongside the existing `circuit_breaker`, `error`, `paths`, `security` module declarations.

- [ ] Task 16: Update `long_term.rs` to use `utils::fs::file_exists`
  - File: `src/memory/long_term.rs`
  - Action: Replace the `tokio::fs::try_exists` call introduced in Task 13 with `crate::utils::fs::file_exists(&file_path).await`. Also update any other `StorageFailed` constructions to use the helper from Task 12.

- [ ] Task 17: Run full validation after Phase 2
  - Action: Run `cargo test --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check`. Fix any issues before proceeding to Phase 3.

#### Phase 3 — Architecture

- [ ] Task 18: Unify duplicate `SessionManager` in `gateway.rs`
  - File: `src/gateway.rs`
  - Action: Remove the second `SessionManager` instance created for `AgentLoop` (the one wrapped in `RwLock`). Pass the existing `session_manager` (already an `Arc<SessionManager>` or similar) to `AgentLoop` instead. Update `AgentLoop` constructor to accept the shared `SessionManager`. The existing comment "In a future refactor, we should unify these" marks the location.
  - Notes: Read `gateway.rs` carefully around line 154 and the `AgentLoop` constructor to understand the current wiring before changing.

- [ ] Task 19: Add `ProviderConfig::set_default_model()` method
  - File: `src/providers/factory.rs`
  - Action: Add a `pub fn set_default_model(&mut self, model: String)` method to `ProviderConfig` (the enum). Implement it as a match arm per variant that sets the `default_model` field on each inner config struct. This mirrors the existing `default_model()` getter.
  - Notes: Used by `apply_cli_model_override()` in loader.rs (Task 20).

- [ ] Task 20: Refactor `apply_cli_model_override()` in `loader.rs`
  - File: `src/config/loader.rs`
  - Action: Replace the per-provider match arm in `apply_cli_model_override()` (lines 78–104) with a call to `provider_config.set_default_model(model)` using the method added in Task 19. The function body should collapse to: check if model override is set, call `set_default_model()`, return.
  - Notes: This eliminates one match arm per provider (currently 4+) that will need updating every time a new provider is added.

- [ ] Task 21: Extract deprecated-field warn macro in `loader.rs`
  - File: `src/config/loader.rs`
  - Action: Replace the 3 separate `if field.is_some() { warn!(...) }` blocks for deprecated fields (lines 138–163) with a single `macro_rules! warn_deprecated` macro (defined locally in the file or in a utils module) that takes the field name and value and emits the warning. Call the macro 3 times.
  - Notes: Example macro shape: `macro_rules! warn_deprecated { ($field:expr, $name:literal) => { if $field.is_some() { warn!("Config field '{}' is deprecated", $name); } }; }`

- [ ] Task 22: Introduce `ApiKeyProviderConfig` trait for API-key providers
  - File: `src/providers/factory.rs`
  - Action: Define a `pub trait ApiKeyProviderConfig` with methods: `fn api_key(&self) -> &str`, `fn base_url(&self) -> &str`, `fn default_model(&self) -> &str`. Implement this trait for `OpenRouterConfig`, `OpenAiConfig`, and `KimiConfig`. Do NOT implement for `OllamaConfig` (no api_key). Use the trait where common provider config access is needed.
  - Notes: Ollama is intentionally excluded. This trait does not need to be `dyn`-safe unless needed for dynamic dispatch.

- [ ] Task 23: Consolidate `AgentLoop` constructors into builder pattern
  - File: `src/agent/agent_loop.rs`
  - Action: Replace the 4 constructors (`new`, `with_inbound_receiver`, `with_model`, `with_model_and_receiver`) with:
    1. A single `AgentLoop::new(provider: Arc<dyn LlmProvider>) -> AgentLoopBuilder` that returns a builder
    2. An `AgentLoopBuilder` struct with optional fields: `model: Option<String>`, `inbound_receiver: Option<mpsc::Receiver<InboundMessage>>`, `session_manager: Option<Arc<SessionManager>>`
    3. Builder methods: `.with_model(model: String) -> Self`, `.with_inbound_receiver(rx: Receiver) -> Self`, `.with_session_manager(sm: Arc<SessionManager>) -> Self`
    4. A `.build() -> AgentLoop` finalization method
       Update all call sites in `gateway.rs` and elsewhere.
  - Notes: The builder pattern is the idiomatic Rust solution for optional constructor parameters. Read all 4 existing constructors carefully before implementing.

- [ ] Task 24: Add dirty tracking to `SessionManager`
  - File: `src/session/manager.rs`
  - Action: Add a `dirty_sessions: HashSet<String>` field (wrapped in the existing `RwLock` or a separate `Mutex`) to `SessionManager`. Mark sessions dirty on modification (writes, tool calls). In `save_all_sessions()`, skip sessions not in `dirty_sessions`. Clear the dirty set after successful save.
  - Notes: The current implementation saves ALL sessions every 30s regardless of changes — this is wasteful. After this change, only modified sessions are written to disk.

- [ ] Task 25: Parallelize `SessionManager::save_all_sessions()`
  - File: `src/session/manager.rs`
  - Action: Replace the sequential save loop in `save_all_sessions()` (lines 117–134) with parallel saves using `tokio::task::JoinSet` or `futures::future::join_all`. Each session save becomes a spawned task. Collect all results and log any individual failures without failing the entire save.
  - Notes: Only sessions marked dirty (from Task 24) need to be saved. Error handling: log failures per session, do not propagate (fire-and-forget with logging).

- [ ] Task 26: Unify `MemoryEntry` types across memory modules
  - Files: `src/memory/types.rs`, `src/memory/short_term.rs`, `src/memory/daily_notes.rs`, `src/memory/long_term.rs`
  - Action: Remove the local `MemoryEntry` struct from `short_term.rs` (which has only `content` + `timestamp`) and the `DailyNoteEntry` struct from `daily_notes.rs`. Update both modules to use `crate::memory::types::MemoryEntry` instead. The canonical `MemoryEntry` already has `memory_type: MemoryType` and `file_path: Option<String>` — set appropriate values when constructing entries in short-term (use `MemoryType::ShortTerm`, `file_path: None`) and daily notes (use `MemoryType::Daily`, `file_path` = note file path).
  - Notes: `daily_notes.rs` already uses `types::MemoryEntry` for some things — read it carefully to understand what changes. `long_term.rs` also has its own `LongTermMemoryEntry` — assess whether to unify that too or leave for a follow-up.

- [ ] Task 27: Extract provider display helpers in `onboard.rs`
  - File: `src/config/onboard.rs`
  - Action: Create a private helper function `fn format_provider_summary(config: &Config) -> String` that renders the provider/telegram display block once. Call this helper from `confirm_configuration()`, `display_completion_summary()`, and `save_configuration()` (whichever functions currently duplicate this display logic).
  - Notes: Read these 3 functions carefully to identify the exact duplicated lines before extracting.

- [ ] Task 28: Run full validation after Phase 3
  - Action: Run `cargo test --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check`. Fix any issues before proceeding to Phase 4.

#### Phase 4 — Tests

- [ ] Task 29: Delete `tests/version_tests.rs` (already in Phase 1 Task 9)
  - Notes: If not already done in Phase 1, delete this file now. All 4 tests are present in `cli_tests.rs`.

- [ ] Task 30: Consolidate session tests between `gateway.rs` inline tests and `session_integration_tests.rs`
  - Files: `src/gateway.rs`, `tests/session_integration_tests.rs`
  - Action: Review the 7 inline `#[cfg(test)]` tests in `gateway.rs` and the tests in `session_integration_tests.rs`. Move any non-gateway-specific session persistence tests to `session_integration_tests.rs`. Keep only gateway-specific tests (e.g., shutdown flush, auto-persistence timing) inline in `gateway.rs`.
  - Notes: Avoid deleting tests without confirming coverage is preserved. The goal is to reduce duplication, not coverage.

- [ ] Task 31: Add missing CLI error-path tests to `cli_tests.rs`
  - File: `tests/cli_tests.rs`
  - Action: Add integration tests for error paths identified in the audit:
    1. Test: running `agent` command without a configured provider exits with non-zero code and prints an actionable error message
    2. Test: running with an invalid `--config` path prints a meaningful error
  - Notes: Use `assert_cmd::Command::cargo_bin("miniclaw")` with `.assert().failure()` and `.stderr(predicate::str::contains("..."))`.

- [ ] Task 32: Add gateway integration tests for signal handling
  - File: `tests/gateway_tests.rs`
  - Action: Add tests verifying:
    1. Sessions are flushed to disk on graceful shutdown
    2. The gateway starts up successfully (basic smoke test)
  - Notes: Signal handling tests may require spawning a subprocess with `assert_cmd` and sending signals. Scope to what's feasible without complex OS-level test infrastructure.

- [ ] Task 33: Run full final validation
  - Action: Run `cargo test --all`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, and `cargo check --benches`. All must pass with zero warnings (clippy) and zero failures (tests).

### Acceptance Criteria

- [ ] AC 1: Given Phase 1 is complete, when `cargo clippy --all-targets -- -D warnings` is run, then no warnings about dead code exist for `prompt_api_key` or `_built_in_tools`, and no duplicate const definitions are reported.

- [ ] AC 2: Given Phase 1 is complete, when `cargo test --all` is run, then all tests pass and `tests/version_tests.rs` no longer exists in the repository.

- [ ] AC 3: Given the version string tests in `cli_tests.rs` have been updated, when the `CARGO_PKG_VERSION` environment variable reflects the current version, then the version command tests pass without hardcoded string comparison.

- [ ] AC 4: Given `src/agent/tools/security.rs` exists, when `exec.rs` or `spawn.rs` are read, then neither file contains a local definition of `EXEC_BLACKLIST`, `SPAWN_BLACKLIST`, `is_blacklisted()`, or `validate_cwd()` — all are delegated to `security.rs`.

- [ ] AC 5: Given Phase 2 is complete, when `ChatHub` is inspected, then `inbound_rx` and `outbound_rx` fields are plain `mpsc::Receiver<T>` (not `Arc<RwLock<mpsc::Receiver<T>>>`), and `recv_inbound()` / `recv_outbound()` take `&mut self`.

- [ ] AC 6: Given Phase 2 is complete, when `cargo test --all` is run (including chat/hub integration tests), then all tests pass with the new `&mut self` receiver signature.

- [ ] AC 7: Given `long_term.rs` has been updated, when a memory file does not exist, then the existence check uses `tokio::fs` (async) instead of blocking `std::fs` / `Path::exists()`.

- [ ] AC 8: Given Phase 3 is complete, when `AgentLoop` is constructed in `gateway.rs`, then only a single `SessionManager` instance exists and it is shared between the gateway and the agent loop.

- [ ] AC 9: Given the `AgentLoopBuilder` is implemented, when `AgentLoop` is constructed, then all optional parameters are set via builder methods (`.with_model()`, `.with_inbound_receiver()`, etc.) and the old `with_model_and_receiver` constructor no longer exists.

- [ ] AC 10: Given dirty tracking is implemented in `SessionManager`, when no sessions are modified between save cycles, then `save_all_sessions()` writes zero files to disk (verified by checking file modification times or a mock).

- [ ] AC 11: Given parallel saves are implemented, when `save_all_sessions()` is called with N dirty sessions, then all N sessions are saved concurrently (using `JoinSet` or equivalent), not sequentially.

- [ ] AC 12: Given Phase 3 is complete, when `short_term.rs` and `daily_notes.rs` are inspected, then neither file defines its own `MemoryEntry` or `DailyNoteEntry` struct — both use `crate::memory::types::MemoryEntry`.

- [ ] AC 13: Given Phase 4 is complete, when `cargo test --all` is run, then the total test count is lower than before Phase 4 (duplicate `version_tests.rs` removed) and all remaining tests pass.

- [ ] AC 14: Given the new CLI error-path tests are added, when `cargo test --test cli_tests` is run, then tests for missing provider config and invalid config path both pass with appropriate exit codes and error messages.

- [ ] AC 15: Given all phases are complete, when `cargo clippy --all-targets -- -D warnings` is run, then zero warnings are reported. When `cargo fmt -- --check` is run, then zero formatting issues are reported. When `cargo check --benches` is run, then benchmarks compile without errors.

## Additional Context

### Dependencies

- `tokio::fs` — already a dependency; use for async file existence check replacing `std::fs::metadata` / `file_path.exists()` in async contexts
- `tokio::task::JoinSet` — available in `tokio` ≥ 1.21; use for parallel session saves in Phase 3 (Task 25)
- `futures::future::join_all` — alternative to `JoinSet`; prefer `JoinSet` for better error handling
- No new external crate dependencies required; all changes use existing crates in `Cargo.toml`
- Task ordering within phases respects dependencies: Tasks 3–4 before Task 5; Task 19 before Task 20; Task 24 before Task 25; Tasks 3–5 (Phase 1) before Task 29 (Phase 4)

### Testing Strategy

- **After Phase 1**: `cargo test --all` + `cargo clippy --all-targets -- -D warnings` — verify dead code gone, no regressions
- **After Phase 2**: `cargo test --all` — focus on chat/hub tests and memory tests; verify `RwLock` removal compiles
- **After Phase 3**: `cargo test --all` — focus on session tests (dirty tracking, parallel saves); verify `AgentLoop` builder works end-to-end in gateway
- **After Phase 4**: Full suite: `cargo test --all` + `cargo clippy` + `cargo fmt -- --check` + `cargo check --benches`
- **Pre-mortem risk items**:
  - `ChatHub` `&mut self` change may break callers that hold shared references — audit all call sites before Task 11
  - `MemoryEntry` unification (Task 26) may silently change serialization format if `DailyNoteEntry` had different field names — verify JSON output
  - Parallel session saves (Task 25) introduce concurrent file writes — ensure each session writes to its own distinct file path
  - `AgentLoop` builder (Task 23) — all 4 existing constructors have call sites; find all with `grep` before removing

### Notes

- Source: `docs/OPTIMIZE_PROJECT.md` — audit report generated 18 February 2026.
- **High-risk tasks**: Task 11 (RwLock), Task 23 (AgentLoop builder), Task 26 (MemoryEntry unification), Task 25 (parallel saves) — implement these with extra care and run tests immediately after each.
- **Low-risk tasks** (can be done quickly): Tasks 1, 2, 6, 8, 9, 15, 21, 27, 29
- The `LongTermMemoryEntry` in `long_term.rs` has its own struct — Task 26 scopes to `ShortTermMemory::MemoryEntry` and `DailyNoteEntry`; `LongTermMemoryEntry` unification is a follow-up if desired.
- Future consideration: `gateway.rs` signal handler tests (Task 32) may be limited by OS-level test complexity — scope to what's achievable with `assert_cmd` subprocess model.
