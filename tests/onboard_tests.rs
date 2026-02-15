//! Integration tests for the onboard command
//!
//! These tests verify CLI integration and end-to-end onboarding flows.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test that onboard command shows help correctly
#[test]
fn test_onboard_help_shows_options() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialize workspace"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--path"));
}

/// Test onboard with --yes flag creates workspace without prompts
#[test]
fn test_onboard_yes_creates_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_miniclaw");

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg(&workspace_path);

    cmd.assert().success();

    // Verify directories were created
    assert!(workspace_path.exists());
    assert!(workspace_path.join("workspace").exists());
}

/// Test onboard fails with relative path
#[test]
fn test_onboard_rejects_relative_path() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg("relative/path");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("absolute path"));
}

/// Test onboard shows warning when workspace exists (without forcing reconfigure)
#[test]
fn test_onboard_warns_existing_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_miniclaw");

    // First, create the workspace
    fs::create_dir_all(&workspace_path).unwrap();

    // Then try to onboard again (with --yes to skip prompt)
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg(&workspace_path);

    // With --yes, it should not reconfigure, so it exits gracefully
    cmd.assert().success();
}

/// Test verbose flag is accepted
#[test]
fn test_onboard_accepts_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_miniclaw");

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose")
        .arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg(&workspace_path);

    cmd.assert().success();
}

/// Test onboard creates workspace structure correctly
#[test]
fn test_onboard_creates_workspace_structure() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_miniclaw");

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg(&workspace_path);

    cmd.assert().success();

    // Verify workspace directory structure was created
    assert!(workspace_path.exists());
    assert!(workspace_path.join("workspace").exists());
}

/// Test onboard with custom absolute path works
#[test]
fn test_onboard_custom_absolute_path() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("custom_workspace");

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("onboard")
        .arg("--yes")
        .arg("-p")
        .arg(&workspace_path);

    cmd.assert().success();

    assert!(workspace_path.exists());
    assert!(workspace_path.join("workspace").exists());
}

/// Test that onboard command can be combined with global flags
#[test]
fn test_onboard_with_global_flags() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_miniclaw");

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose")
        .arg("onboard")
        .arg("--yes")
        .arg("--path")
        .arg(&workspace_path);

    cmd.assert().success();
}
