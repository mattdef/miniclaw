//! Integration tests for workspace functionality
//!
//! These tests verify the workspace module integration with the rest of the system.

use std::process::Command;
use tempfile::TempDir;

/// Test that workspace directory is created during onboarding
#[test]
fn test_onboard_creates_workspace_directory() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard command with custom path and --yes flag
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output()
        .expect("Failed to execute command");

    // Check that workspace directory was created
    let workspace_path = base_path.join("workspace");
    assert!(
        workspace_path.exists(),
        "Workspace directory should exist after onboarding"
    );
}

/// Test that all workspace files are created
#[test]
fn test_onboard_creates_all_workspace_files() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard command
    let _ = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output();

    // Check that all workspace files exist
    let workspace_path = base_path.join("workspace");
    let expected_files = [
        "SOUL.md",
        "AGENTS.md",
        "USER.md",
        "TOOLS.md",
        "HEARTBEAT.md",
    ];

    for filename in &expected_files {
        let file_path = workspace_path.join(filename);
        assert!(
            file_path.exists(),
            "{} should exist after onboarding",
            filename
        );
    }
}

/// Test that workspace files have correct default content
#[test]
fn test_workspace_files_have_default_content() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard command
    let _ = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output();

    // Check SOUL.md content
    let soul_content = std::fs::read_to_string(base_path.join("workspace/SOUL.md")).unwrap();
    assert!(soul_content.contains("Agent Soul"));
    assert!(soul_content.contains("Personality Traits"));

    // Check AGENTS.md content
    let agents_content = std::fs::read_to_string(base_path.join("workspace/AGENTS.md")).unwrap();
    assert!(agents_content.contains("Agent Guidelines"));
    assert!(agents_content.contains("Available Tools"));

    // Check USER.md content
    let user_content = std::fs::read_to_string(base_path.join("workspace/USER.md")).unwrap();
    assert!(user_content.contains("User Profile"));
    assert!(user_content.contains("Preferences"));

    // Check TOOLS.md content
    let tools_content = std::fs::read_to_string(base_path.join("workspace/TOOLS.md")).unwrap();
    assert!(tools_content.contains("Tool Documentation"));
    assert!(tools_content.contains("Example"));

    // Check HEARTBEAT.md content
    let heartbeat_content =
        std::fs::read_to_string(base_path.join("workspace/HEARTBEAT.md")).unwrap();
    assert!(heartbeat_content.contains("Heartbeat System"));
    assert!(heartbeat_content.contains("cron"));
}

/// Test that existing workspace files are preserved during re-onboarding
#[test]
fn test_existing_workspace_files_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");
    let workspace_path = base_path.join("workspace");

    // Create initial workspace with custom SOUL.md
    std::fs::create_dir_all(&workspace_path).unwrap();
    std::fs::write(workspace_path.join("SOUL.md"), "Custom SOUL Content").unwrap();

    // Run onboard command with --yes to skip reconfiguration prompt
    let _ = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output();

    // Verify custom SOUL.md is preserved
    let soul_content = std::fs::read_to_string(workspace_path.join("SOUL.md")).unwrap();
    assert_eq!(soul_content, "Custom SOUL Content");
}

/// Test that workspace directory has correct permissions (Unix only)
#[test]
#[cfg(unix)]
fn test_workspace_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard command
    let _ = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output();

    // Check workspace directory permissions
    let workspace_path = base_path.join("workspace");
    let metadata = std::fs::metadata(&workspace_path).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode() & 0o777;

    assert_eq!(
        mode, 0o755,
        "Workspace directory should have 0755 permissions, got {:o}",
        mode
    );
}

/// Test that workspace markdown files have correct permissions (Unix only)
#[test]
#[cfg(unix)]
fn test_workspace_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard command
    let _ = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
        ])
        .output();

    // Check SOUL.md permissions
    let soul_path = base_path.join("workspace/SOUL.md");
    let metadata = std::fs::metadata(&soul_path).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode() & 0o777;

    assert_eq!(
        mode, 0o644,
        "SOUL.md should have 0644 permissions, got {:o}",
        mode
    );
}

/// Test onboard with verbose flag creates workspace with logging
#[test]
fn test_onboard_verbose_creates_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("miniclaw_test");

    // Run onboard with verbose flag
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "onboard",
            "--path",
            &base_path.to_string_lossy(),
            "--yes",
            "--verbose",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify workspace was created
    let workspace_path = base_path.join("workspace");
    assert!(workspace_path.exists());

    // In verbose mode, we should see some logging
    // Note: This may vary depending on logging configuration
    assert!(
        output.status.success(),
        "Command should succeed. stderr: {}",
        stderr
    );
}
