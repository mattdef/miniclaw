//! Skill system constants
//!
//! Centralized constants for skill management to ensure consistency across modules.

/// Built-in tool names that cannot be used as skill names or deleted
///
/// This is the single source of truth for built-in tool protection.
/// Used by both SkillsManager validation and tool implementations.
pub const BUILT_IN_TOOLS: &[&str] = &[
    "filesystem",
    "exec",
    "web",
    "message",
    "spawn",
    "cron",
    "memory",
    "create_skill",
    "list_skills",
    "read_skill",
    "delete_skill",
];
