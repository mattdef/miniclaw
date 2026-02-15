# Story 2.4: Skills Directory Setup

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want a dedicated directory for custom skills,
So that I can extend miniclaw's capabilities with reusable skill packages.

## Acceptance Criteria

1. **Skills Directory Creation** (AC: 1)
   - **Given** workspace initialization
   - **When** the system creates the workspace structure
   - **Then** it creates `~/.miniclaw/workspace/skills/` directory

2. **Skill Package Recognition** (AC: 2)
   - **Given** the skills directory exists
   - **When** I create a new skill folder (e.g., `skills/weather/`)
   - **Then** miniclaw recognizes it as a valid skill package

3. **SKILL.md Parsing** (AC: 3)
   - **Given** a skill directory exists
   - **When** I add a `SKILL.md` file inside it
   - **Then** the system parses the skill definition
   - **And** makes it available to the agent

4. **Skill Loading** (AC: 4)
   - **Given** a skill package with SKILL.md
   - **When** the agent loads skills
   - **Then** it reads the skill name, description, and parameters
   - **And** validates the skill format

5. **Multiple Skills Support** (AC: 5)
   - **Given** multiple skills exist in the skills directory
   - **When** the agent assembles context
   - **Then** it loads all valid skills
   - **And** includes them in the available tools context

6. **Invalid Skill Handling** (AC: 6)
   - **Given** a skill has invalid format
   - **When** the system tries to load it
   - **Then** it logs a warning about the invalid skill
   - **And** skips that skill but continues loading others

7. **Skill Disabling** (AC: 7)
   - **Given** I want to disable a skill
   - **When** I rename the directory with a dot prefix (e.g., `.weather/`)
   - **Then** the system ignores that skill
   - **And** does not load it into the agent context

8. **list_skills Tool Integration** (AC: 8)
   - **Given** the `list_skills` tool is called
   - **When** the agent executes it
   - **Then** it returns a list of all available skills
   - **And** shows skill names and descriptions
   - **And** indicates which skills are active

## Tasks / Subtasks

- [x] Create skills module structure (AC: 1)
  - [x] Create `src/skills/mod.rs` with skills loading logic
  - [x] Create `src/skills/loader.rs` for skill discovery and parsing
  - [x] Create `src/skills/types.rs` for skill data structures
  - [x] Export public functions from skills module
- [x] Implement skills directory creation (AC: 1)
  - [x] Create `~/.miniclaw/workspace/skills/` directory during onboarding
  - [x] Handle directory creation errors gracefully
  - [x] Set appropriate permissions (0755 for directories)
- [x] Implement skill package recognition (AC: 2)
  - [x] Create function to scan skills directory
  - [x] Filter valid skill directories (non-hidden)
  - [x] Return list of skill paths
- [x] Implement SKILL.md parsing (AC: 3-4)
  - [x] Define SKILL.md structure/schema
  - [x] Create parser for SKILL.md content
  - [x] Extract skill name, description, parameters
  - [x] Validate required fields
- [x] Implement skill loading for context (AC: 5)
  - [x] Load all valid skills from directory
  - [x] Convert skills to context format
  - [x] Include in agent context assembly
- [x] Implement invalid skill handling (AC: 6)
  - [x] Add validation for skill format
  - [x] Log warnings for invalid skills
  - [x] Continue loading other skills on error
- [x] Implement skill disabling (AC: 7)
  - [x] Filter out hidden directories (dot prefix)
  - [x] Document disabling mechanism
- [x] Implement list_skills tool (AC: 8)
  - [x] Create tool function to list skills
  - [x] Return skill names and descriptions
  - [x] Indicate active/disabled status
- [x] Integrate with workspace module (AC: 1)
  - [x] Call skills directory creation from workspace initialization
  - [x] Add verbose logging for skills directory creation
- [x] Review Follow-ups (AI)
  - [x] [AI-Review][High] Stage untracked files to git
  - [x] [AI-Review][Medium] Fix unused variable in `loader.rs`
  - [x] [AI-Review][Medium] Update story file list to reflect actual changes
- [x] Write unit tests
  - [x] Test skills directory creation
  - [x] Test skill package recognition
  - [x] Test SKILL.md parsing
  - [x] Test invalid skill handling
  - [x] Test skill disabling (dot prefix)
  - [x] Test list_skills tool

## Dev Notes

### Architecture Compliance

**Module Structure:**

- Create `src/skills/` directory with module structure
- Create `src/skills/mod.rs` as module interface
- Create `src/skills/loader.rs` for skill discovery and parsing
- Create `src/skills/types.rs` for skill data structures
- Follow Rust RFC 430 naming conventions strictly

**Skill Module Design:**

```rust
// src/skills/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub parameters: Vec<SkillParameter>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: String,
}

// src/skills/loader.rs
use std::path::PathBuf;
use anyhow::Result;

/// Scan skills directory and return list of valid skill paths
pub fn discover_skills(skills_dir: &PathBuf) -> Result<Vec<PathBuf>> {
    // Filter non-hidden directories containing SKILL.md
}

/// Parse a single skill from its directory
pub fn load_skill(skill_path: &PathBuf) -> Result<Skill> {
    // Read SKILL.md and parse content
}

/// Load all valid skills for context assembly
pub fn load_all_skills(skills_dir: &PathBuf) -> Result<Vec<Skill>> {
    // Discover and load all skills, skip invalid ones
}

/// List skills for list_skills tool
pub fn list_available_skills(skills_dir: &PathBuf) -> Result<Vec<(String, String, bool)>> {
    // Return (name, description, is_active) for all skills
}
```

**SKILL.md Structure:**

```markdown
# Skill: {name}

## Description
{description of what the skill does}

## Parameters
- `param1` (string, required): Description of parameter 1
- `param2` (number, optional): Description of parameter 2

## Usage
{example usage or detailed content}
```

**File Permissions:**

- Directories: 0755 (rwxr-xr-x)
- SKILL.md files: 0644 (rw-r--r--)
- Consistent with permissions used in Story 2.3 (workspace)

**Error Handling:**

- Use `anyhow` for CLI/main error handling
- Use `thiserror` for skills-specific errors
- Provide clear error messages for invalid skills
- Log warnings but continue loading on invalid skill

**Integration with Workspace:**

- Reuse workspace path detection from Story 2.2/2.3
- Call skills directory creation from `initialize_workspace()`
- Skills loading interface will be used by Epic 5 (Agent Core) Context Builder

### Source Tree Components

**Files to Create:**

- `src/skills/mod.rs` - Skills module interface
- `src/skills/loader.rs` - Skill discovery and parsing
- `src/skills/types.rs` - Skill data structures

**Files to Modify:**

- `src/workspace/mod.rs` - Call skills directory creation
- `src/workspace/templates.rs` - Add skills directory creation
- `src/main.rs` - Add skills module declaration
- `Cargo.toml` - No new dependencies (use existing anyhow, serde, regex)

**Skills Directory Structure:**

```
~/.miniclaw/
├── config.json              # Created in Story 2.1/2.2
└── workspace/               # Created in Story 2.3
    ├── SOUL.md              # Agent personality
    ├── AGENTS.md            # Agent behavior guidelines
    ├── USER.md              # User profile and preferences
    ├── TOOLS.md             # Tool documentation
    ├── HEARTBEAT.md         # Scheduled tasks
    ├── skills/              # Created in this story
    │   ├── weather/         # Example skill
    │   │   └── SKILL.md
    │   └── reminder/        # Example skill
    │       └── SKILL.md
    ├── sessions/            # Created in Story 2.5
    └── memory/              # Created in Epic 8
```

**Integration with Context Builder (Epic 5):**

The skills loaded here will be used by the Context Builder (Story 5.2) to assemble the agent context. Skills should be formatted as available capabilities in the context.

```rust
// Example integration with ContextBuilder
pub fn load_skills_context(skills_dir: &PathBuf) -> Result<String> {
    let skills = load_all_skills(skills_dir)?;
    let mut context = String::from("## Available Skills\n\n");
    
    for skill in skills {
        context.push_str(&format!("### {}\n", skill.name));
        context.push_str(&format!("{}\n\n", skill.description));
        if !skill.parameters.is_empty() {
            context.push_str("**Parameters:**\n");
            for param in &skill.parameters {
                context.push_str(&format!("- `{}` ({}, {}): {}\n",
                    param.name,
                    param.param_type,
                    if param.required { "required" } else { "optional" },
                    param.description
                ));
            }
            context.push('\n');
        }
    }
    
    Ok(context)
}
```

### Testing Standards

- Co-located tests in `#[cfg(test)]` module at bottom of each file
- Integration tests in `tests/skills_tests.rs`
- Test scenarios:
  - Skills directory creation
  - Skill discovery (valid and hidden directories)
  - SKILL.md parsing (valid and invalid)
  - Skill loading with format validation
  - Multiple skills loading
  - list_skills tool functionality
  - Error handling for malformed skills

### Project Structure Notes

- Skills directory is foundation for Story 7.3 (Skill Management Tools)
- Skill parsing will be extended in Epic 7 with create/read/list/delete tools
- Skills integrate with Context Builder in Epic 5
- Hidden directory filtering (dot prefix) is a simple disabling mechanism
- Aligns with unified project structure defined in architecture.md

### Previous Story Intelligence

**From Story 2.1 (Configuration File Management):**

- Config directory: `~/.miniclaw/` established
- Pattern for module creation: create module file structure
- File permission handling: use `std::fs::set_permissions()` with `Permissions::from_mode()`
- Error handling pattern: `anyhow` for CLI, `thiserror` for library errors

**From Story 2.2 (Interactive Onboarding Command):**

- Workspace base path: `~/.miniclaw/` determined
- Integration point: add skills directory creation to workspace initialization
- Verbose logging pattern: `debug!("Creating directory: {:?}", path);`

**From Story 2.3 (Workspace Structure Creation):**

- Workspace module structure established: `src/workspace/mod.rs`
- Template pattern: `src/workspace/templates.rs` with string constants
- Directory creation pattern: `std::fs::create_dir_all()`
- File permissions: 0755 for directories, 0644 for files
- Integration: call from `initialize_workspace()` in workspace module
- Error handling: use `with_context()` from anyhow

**Key Implementation Details from 2.3:**

```rust
// From src/workspace/mod.rs - reuse patterns
use std::path::Path;
use anyhow::{Context, Result};

// Directory creation pattern
std::fs::create_dir_all(&skills_path)
    .with_context(|| format!("Failed to create skills directory: {:?}", skills_path))?;

// Permission setting pattern (Unix)
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(&skills_path, permissions)?;
}
```

**Git Intelligence - Recent Patterns:**

- Modular development approach (config module first, then workspace)
- Pattern: Add module structure → implement core functions → add tests → integrate
- Recent commits show clean module separation
- Test files added separately in `tests/` directory

### Anti-Pattern Prevention

**CRITICAL: Avoid these common mistakes:**

1. **Don't reinvent workspace path logic** - Reuse workspace module functions
2. **Don't parse SKILL.md with regex only** - Use structured parsing (serde frontmatter or manual parsing with validation)
3. **Don't fail on single invalid skill** - Log warning and continue (resilience)
4. **Don't hardcode skills directory path** - Use workspace path from config
5. **Don't forget Unix permissions** - Set 0755 on skills directory
6. **Don't create skills during every run** - Check existence first (idempotent)
7. **Don't ignore hidden directory convention** - Filter dot-prefix directories
8. **Don't skip validation** - Validate SKILL.md has required fields

### References

- Architecture: [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- Architecture Data Persistence: [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- Epic 2: [Source: _bmad-output/planning-artifacts/epics.md#Epic-2-Configuration-&-Workspace-Management]
- Story 2.2 (Previous): [Source: _bmad-output/implementation-artifacts/2-2-interactive-onboarding-command.md]
- Story 2.3 (Previous): [Source: _bmad-output/implementation-artifacts/2-3-workspace-structure-creation.md]
- Story 2.4 Requirements: [Source: _bmad-output/planning-artifacts/epics.md#Story-2.4-Skills-Directory-Setup]
- Story 5.2 (Context Builder): [Source: _bmad-output/planning-artifacts/epics.md#Story-5.2-Context-Builder]
- Story 7.3 (Future): [Source: _bmad-output/planning-artifacts/epics.md#Story-7.3-Skill-Management-Tools]
- File Naming Conventions: [Source: _bmad-output/planning-artifacts/architecture.md#Naming-Patterns]

## Dev Agent Record

### Agent Model Used

kimi-for-coding/k2p5

### Debug Log References

### Completion Notes List

- **2026-02-15:** Implemented complete skills module with all functionality
  - Created `src/skills/types.rs` with Skill, SkillParameter, SkillSummary structs
  - Created `src/skills/loader.rs` with core skill functions
  - Created `src/skills/mod.rs` with module interface
  - Integrated with workspace module
  - Updated `src/main.rs` to include skills module declaration
  - **Review Fixes (2026-02-15):**
    - Staged all files to Git (previously untracked)
    - Updated documentation to include missing files (onboard.rs, templates.rs, tests)
    - Fixed unused variable `directory_name` in `loader.rs`
  - All 34 unit tests passing
    
  - Architecture compliance:
    - Rust RFC 430 naming conventions followed
    - `anyhow` for error handling with context
    - `thiserror` for custom error types
    - Proper file permissions on Unix (0755 for directories)
    - Idempotent directory creation (checks existence first)
    - Hidden directory filtering (dot prefix = disabled)
    - Comprehensive error handling and logging
    
  - All 8 Acceptance Criteria satisfied:
    - AC1: Skills directory creation ✓
    - AC2: Skill package recognition (valid dirs with SKILL.md) ✓
    - AC3-4: SKILL.md parsing with validation ✓
    - AC5: Multiple skills loading for context ✓
    - AC6: Invalid skill handling (warnings, continue loading) ✓
    - AC7: Skill disabling via dot prefix ✓
    - AC8: list_skills tool integration ✓

### File List

**New Files:**

- `src/skills/mod.rs` - Skills module interface with context loading
- `src/skills/loader.rs` - Skill discovery, loading, and parsing functions
- `src/skills/types.rs` - Skill data structures (Skill, SkillParameter, SkillSummary, SkillError)
- `src/config/onboard.rs` - Interactive onboarding implementation (discovered during review)
- `src/workspace/templates.rs` - Default workspace templates (discovered during review)
- `tests/onboard_tests.rs` - Integration tests for onboarding (discovered during review)
- `tests/workspace_tests.rs` - Integration tests for workspace (discovered during review)

**Modified Files:**

- `src/main.rs` - Added `mod skills;` declaration
- `src/cli.rs` - CLI structural updates for onboarding (discovered during review)
- `src/config/mod.rs` - Exported onboarding module (discovered during review)
- `src/workspace/mod.rs` - Integrated skills directory creation in `initialize_workspace()`
- `Cargo.toml` - Dependency updates for inquire/serde (discovered during review)
- `Cargo.lock` - Dependency lock updates (discovered during review)

**Test Coverage:**

- 34 unit tests in skills module (all passing)
- 19 workspace tests passing (integration verified)
- Tests cover: directory creation, skill discovery, parsing, validation, error handling

---

## Change Log

**2026-02-15:** Story 2.4 - Skills Directory Setup - Implementation Complete

- Created comprehensive skills module with full functionality
- Implemented skill discovery, parsing, loading, and context integration
- Added support for skill disabling via dot-prefix directory names
- Integrated skills directory creation with workspace initialization
- All 8 acceptance criteria satisfied
- 34 unit tests implemented and passing
- Status: ready-for-dev → in-progress → review

