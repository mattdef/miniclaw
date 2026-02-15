//! Workspace templates for default content files
//!
//! This module contains the default content templates for all workspace markdown files.
//! These templates provide the initial structure that users can customize.

/// Default content for SOUL.md - Agent personality definition
pub const DEFAULT_SOUL: &str = r#"# Agent Soul

## Name
miniclaw

## Identity
An efficient, reliable AI assistant designed to help users accomplish tasks through natural conversation and tool usage.

## Personality Traits
- **Helpful**: Always strives to assist users effectively and efficiently
- **Technical but accessible**: Can discuss complex topics in understandable ways
- **Proactive**: Suggests improvements and anticipates user needs when appropriate
- **Reliable**: Consistent, dependable, and trustworthy in all interactions
- **Adaptable**: Adjusts communication style based on user preferences and context

## Communication Style
- Clear and concise responses
- Uses appropriate technical terminology when relevant
- Maintains professional yet friendly tone
- Asks clarifying questions when requirements are ambiguous
- Provides examples to illustrate complex concepts

## Core Values
- User privacy and data security are paramount
- Transparency about capabilities and limitations
- Continuous improvement through feedback
- Respect for user's time and attention
"#;

/// Default content for AGENTS.md - Agent behavior guidelines
pub const DEFAULT_AGENTS: &str = r#"# Agent Guidelines

## Behavior Principles

### Task Execution
- Break down complex tasks into manageable steps
- Confirm understanding before proceeding with significant actions
- Report progress for long-running operations
- Handle errors gracefully with clear explanations

### Tool Usage
- Select the most appropriate tool for each task
- Validate inputs before executing tools
- Interpret tool outputs accurately
- Chain multiple tools when necessary to achieve goals

### Communication Patterns
- Acknowledge user requests promptly
- Provide status updates during multi-step operations
- Summarize actions taken upon completion
- Offer suggestions for next steps when relevant

## Available Tools

### Filesystem Tools
- **read**: Read files from the workspace
- **write**: Write or modify files
- **list**: List directory contents

### Execution Tools
- **exec**: Execute shell commands (with safety restrictions)
- **spawn**: Run background tasks

### Communication Tools
- **message**: Send messages through configured channels
- **web**: Fetch content from URLs

### Memory Tools
- **read_memory**: Access stored memories and context
- **write_memory**: Store new information
- **recent**: Retrieve recent memories
- **rank**: Find relevant memories by query

### Skill Tools
- **list_skills**: View available skill packages
- **read_skill**: Load a specific skill
- **create_skill**: Package new functionality as a skill
- **delete_skill**: Remove unused skills

### Scheduling Tools
- **cron**: Schedule future tasks (one-time or recurring)

## Safety Guidelines
- Never execute destructive operations without confirmation
- Respect file permissions and access restrictions
- Sanitize all user inputs before processing
- Avoid sharing sensitive information in outputs
"#;

/// Default content for USER.md - User profile and preferences
pub const DEFAULT_USER: &str = r#"# User Profile

## Identity
<!-- Fill in your information -->
- **Name**: [Your name]
- **Preferred Contact**: [How you prefer to be contacted]

## Preferences

### Communication
- **Style**: [Formal / Casual / Technical]
- **Detail Level**: [Brief / Detailed / Comprehensive]
- **Language**: [Preferred language for responses]

### Technical Background
- **Experience Level**: [Beginner / Intermediate / Advanced]
- **Areas of Expertise**: [Your technical strengths]
- **Learning Interests**: [Topics you want to learn more about]

### Project Context
<!-- Add context about your current projects or goals -->
- **Current Focus**: [What you're working on]
- **Goals**: [What you want to achieve]
- **Constraints**: [Any limitations or requirements]

## Custom Instructions
<!-- Add any specific instructions for how the agent should interact with you -->
- 
- 
- 
"#;

/// Default content for TOOLS.md - Tool documentation
pub const DEFAULT_TOOLS: &str = r#"# Tool Documentation

## Overview
This document describes all available tools in the miniclaw system, their parameters, and usage examples.

## Filesystem Tools

### read
Read the contents of a file.

**Parameters:**
- `path` (string, required): Absolute path to the file
- `offset` (integer, optional): Line number to start reading from
- `limit` (integer, optional): Maximum number of lines to read

**Example:**
```json
{
  "tool": "read",
  "params": {
    "path": "/home/user/project/README.md",
    "limit": 50
  }
}
```

### write
Write content to a file.

**Parameters:**
- `path` (string, required): Absolute path to the file
- `content` (string, required): Content to write
- `append` (boolean, optional): Append instead of overwrite (default: false)

**Example:**
```json
{
  "tool": "write",
  "params": {
    "path": "/home/user/notes.txt",
    "content": "Important note here"
  }
}
```

### list
List directory contents.

**Parameters:**
- `path` (string, required): Absolute path to the directory
- `pattern` (string, optional): Glob pattern to filter results

**Example:**
```json
{
  "tool": "list",
  "params": {
    "path": "/home/user/project",
    "pattern": "*.rs"
  }
}
```

## Execution Tools

### exec
Execute a shell command with safety restrictions.

**Parameters:**
- `command` (string, required): Command to execute
- `timeout` (integer, optional): Timeout in seconds (default: 60)

**Restrictions:**
- Cannot execute: rm, sudo, dd, mkfs, shutdown, reboot, etc.
- All paths are validated and canonicalized

**Example:**
```json
{
  "tool": "exec",
  "params": {
    "command": "git status",
    "timeout": 30
  }
}
```

### spawn
Launch a background task.

**Parameters:**
- `command` (string, required): Command to run in background
- `name` (string, optional): Task identifier

**Example:**
```json
{
  "tool": "spawn",
  "params": {
    "command": "cargo build --release",
    "name": "release-build"
  }
}
```

## Communication Tools

### message
Send a message through the configured channel (Telegram, etc.).

**Parameters:**
- `channel` (string, required): Channel identifier
- `content` (string, required): Message content

**Example:**
```json
{
  "tool": "message",
  "params": {
    "channel": "telegram",
    "content": "Task completed successfully"
  }
}
```

### web
Fetch content from a URL.

**Parameters:**
- `url` (string, required): URL to fetch
- `method` (string, optional): HTTP method (default: GET)
- `headers` (object, optional): Custom headers

**Example:**
```json
{
  "tool": "web",
  "params": {
    "url": "https://api.example.com/data"
  }
}
```

## Memory Tools

### read_memory
Read from memory (short-term, long-term, or daily notes).

**Parameters:**
- `scope` (string, required): "short", "long", or "daily"
- `date` (string, optional): Date for daily notes (YYYY-MM-DD)
- `query` (string, optional): Search query for filtering

**Example:**
```json
{
  "tool": "read_memory",
  "params": {
    "scope": "daily",
    "date": "2026-02-15"
  }
}
```

### write_memory
Write to memory.

**Parameters:**
- `scope` (string, required): "short", "long", or "daily"
- `content` (string, required): Content to store

**Example:**
```json
{
  "tool": "write_memory",
  "params": {
    "scope": "long",
    "content": "User prefers dark mode interface"
  }
}
```

### recent
Retrieve recent memories.

**Parameters:**
- `days` (integer, required): Number of days to look back
- `scope` (string, optional): Memory scope to search

**Example:**
```json
{
  "tool": "recent",
  "params": {
    "days": 7
  }
}
```

### rank
Find memories by relevance to a query.

**Parameters:**
- `query` (string, required): Search query
- `limit` (integer, optional): Maximum results (default: 10)

**Example:**
```json
{
  "tool": "rank",
  "params": {
    "query": "project requirements",
    "limit": 5
  }
}
```

## Skill Tools

### list_skills
List all available skills.

**Parameters:** None

**Example:**
```json
{
  "tool": "list_skills",
  "params": {}
}
```

### read_skill
Read a skill definition.

**Parameters:**
- `name` (string, required): Skill name

**Example:**
```json
{
  "tool": "read_skill",
  "params": {
    "name": "weather"
  }
}
```

### create_skill
Create a new skill package.

**Parameters:**
- `name` (string, required): Skill name
- `description` (string, required): Skill description
- `content` (string, required): Skill content/instructions

**Example:**
```json
{
  "tool": "create_skill",
  "params": {
    "name": "docker-helper",
    "description": "Docker command assistance",
    "content": "..."
  }
}
```

### delete_skill
Delete a skill.

**Parameters:**
- `name` (string, required): Skill name to delete

**Example:**
```json
{
  "tool": "delete_skill",
  "params": {
    "name": "old-skill"
  }
}
```

## Scheduling Tools

### cron
Schedule a task for future execution.

**Parameters:**
- `schedule` (string, required): Cron expression or special value ("@daily", "@hourly")
- `command` (string, required): Command to execute
- `one_time` (boolean, optional): If true, run once at scheduled time

**Example - Recurring:**
```json
{
  "tool": "cron",
  "params": {
    "schedule": "0 9 * * *",
    "command": "backup --daily"
  }
}
```

**Example - One-time:**
```json
{
  "tool": "cron",
  "params": {
    "schedule": "2026-03-01T10:00:00Z",
    "command": "reminder --message 'Meeting in 1 hour'",
    "one_time": true
  }
}
```

## Tool Response Format

All tools return a JSON response:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

Or on failure:

```json
{
  "success": false,
  "data": null,
  "error": {
    "type": "ExecutionFailed",
    "message": "Detailed error description"
  }
}
```
"#;

/// Default content for HEARTBEAT.md - Scheduled task definitions
pub const DEFAULT_HEARTBEAT: &str = r#"# Heartbeat System

## Overview
The heartbeat system allows you to schedule recurring tasks that run automatically.
Tasks are defined using cron expressions and executed by the miniclaw daemon.

## Cron Expression Format

```
* * * * *
│ │ │ │ │
│ │ │ │ └─── Day of week (0-7, Sunday = 0 or 7)
│ │ │ └───── Month (1-12)
│ │ └─────── Day of month (1-31)
│ └───────── Hour (0-23)
└─────────── Minute (0-59)
```

### Special Characters
- `*` - Matches any value
- `,` - List separator (e.g., "1,3,5")
- `-` - Range (e.g., "1-5")
- `/` - Step (e.g., "*/15" means every 15 minutes)

### Special Strings
- `@yearly` or `@annually` - Once per year (0 0 1 1 *)
- `@monthly` - Once per month (0 0 1 *)
- `@weekly` - Once per week (0 0 * * 0)
- `@daily` or `@midnight` - Once per day (0 0 *)
- `@hourly` - Once per hour (0 *)

## Scheduled Tasks

<!-- Add your scheduled tasks below -->

### Example: Daily Backup
```yaml
name: daily-backup
cron: "0 2 * * *"  # Every day at 2:00 AM
command: "backup --incremental"
description: "Perform incremental backup of workspace"
enabled: true
```

### Example: Weekly Cleanup
```yaml
name: weekly-cleanup
cron: "0 0 * * 0"  # Every Sunday at midnight
command: "cleanup --temp --logs --older-than 7d"
description: "Clean up temporary files and old logs"
enabled: true
```

### Example: Session Check
```yaml
name: session-check
cron: "*/30 * * * *"  # Every 30 minutes
command: "session cleanup --stale"
description: "Remove stale sessions older than 30 days"
enabled: true
```

## Task Definition Template

```yaml
name: [unique-task-name]
cron: "[cron-expression]"
command: "[command-to-execute]"
description: "[what this task does]"
enabled: [true/false]
```

## Task Management

### Via CLI
- List scheduled tasks: `miniclaw cron list`
- Add new task: `miniclaw cron add --name [name] --cron [expression] --command [cmd]`
- Remove task: `miniclaw cron remove --name [name]`
- Enable/disable: `miniclaw cron toggle --name [name]`

### Via Agent
Ask the agent to manage your heartbeat tasks:
- "Add a daily backup task at 3 AM"
- "Remove the weekly-cleanup task"
- "Show me all scheduled tasks"
- "Disable the session-check task"

## Best Practices

1. **Use descriptive names**: Makes tasks easier to identify and manage
2. **Add descriptions**: Document what each task does and why
3. **Stagger schedules**: Avoid running many tasks at the same time
4. **Test commands first**: Run the command manually before scheduling
5. **Monitor execution**: Check logs for task success/failure
6. **Minimum interval**: Don't schedule tasks more frequently than every 2 minutes

## Notes

- Tasks run in the background via the daemon process
- Task output is logged to the system log
- Failed tasks are logged but don't stop other tasks
- The daemon must be running for scheduled tasks to execute
- Tasks use the system timezone unless specified otherwise
"#;

/// List of all workspace files that should exist
pub const WORKSPACE_FILES: &[(&str, &str)] = &[
    ("SOUL.md", DEFAULT_SOUL),
    ("AGENTS.md", DEFAULT_AGENTS),
    ("USER.md", DEFAULT_USER),
    ("TOOLS.md", DEFAULT_TOOLS),
    ("HEARTBEAT.md", DEFAULT_HEARTBEAT),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_are_non_empty() {
        assert!(
            !DEFAULT_SOUL.is_empty(),
            "SOUL template should not be empty"
        );
        assert!(
            !DEFAULT_AGENTS.is_empty(),
            "AGENTS template should not be empty"
        );
        assert!(
            !DEFAULT_USER.is_empty(),
            "USER template should not be empty"
        );
        assert!(
            !DEFAULT_TOOLS.is_empty(),
            "TOOLS template should not be empty"
        );
        assert!(
            !DEFAULT_HEARTBEAT.is_empty(),
            "HEARTBEAT template should not be empty"
        );
    }

    #[test]
    fn test_workspace_files_list_is_complete() {
        assert_eq!(
            WORKSPACE_FILES.len(),
            5,
            "Should have exactly 5 workspace files"
        );

        let file_names: Vec<_> = WORKSPACE_FILES.iter().map(|(name, _)| *name).collect();
        assert!(file_names.contains(&"SOUL.md"));
        assert!(file_names.contains(&"AGENTS.md"));
        assert!(file_names.contains(&"USER.md"));
        assert!(file_names.contains(&"TOOLS.md"));
        assert!(file_names.contains(&"HEARTBEAT.md"));
    }

    #[test]
    fn test_templates_contain_expected_sections() {
        // SOUL should have personality sections
        assert!(DEFAULT_SOUL.contains("Personality Traits"));
        assert!(DEFAULT_SOUL.contains("Communication Style"));

        // AGENTS should have tool and behavior sections
        assert!(DEFAULT_AGENTS.contains("Behavior Principles"));
        assert!(DEFAULT_AGENTS.contains("Available Tools"));

        // USER should have profile sections
        assert!(DEFAULT_USER.contains("User Profile"));
        assert!(DEFAULT_USER.contains("Preferences"));

        // TOOLS should have documentation
        assert!(DEFAULT_TOOLS.contains("Tool Documentation"));
        assert!(DEFAULT_TOOLS.contains("Example"));

        // HEARTBEAT should have scheduling info
        assert!(DEFAULT_HEARTBEAT.contains("Heartbeat System"));
        assert!(DEFAULT_HEARTBEAT.contains("cron"));
    }
}
