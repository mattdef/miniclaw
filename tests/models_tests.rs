//! Integration tests for the `models` command
//!
//! These tests verify the CLI behavior for listing available models.

use assert_cmd::Command;
use predicates::prelude::*;

/// Test that `models --help` shows the help message
#[test]
fn test_models_help() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("models").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("List available models"));
}

/// Test that `models` without a configured provider shows an error
#[test]
fn test_models_without_provider() {
    // Use a temporary config file with no provider
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    std::fs::write(
        &config_path,
        r#"{
        "log_level": "info",
        "timeout_seconds": 30
    }"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--config").arg(&config_path).arg("models");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No provider configured"));
}

/// Test that `models` command exists and accepts the command
#[test]
fn test_models_command_exists() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("models").arg("--help");

    // Should succeed (exit code 0) and show help
    cmd.assert().success();
}
