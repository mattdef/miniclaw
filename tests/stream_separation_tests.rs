//! Integration tests for output stream separation (stdout vs stderr)
//!
//! These tests verify AC#1-5 from Story 11.2:
//! 1. Normal results go to stdout, logs go to stderr
//! 2. `miniclaw version` outputs only to stdout with no stderr on success
//! 3. Error conditions output to stderr with non-zero exit code
//! 4. Verbose mode: DEBUG logs to stderr, output to stdout
//! 5. Piping: only stdout is piped, stderr shown on terminal

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_version_stdout_only() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");

    // AC#2: version string goes to stdout, no stderr output on success
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_version_stderr_redirect_shows_output() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");
    // Simulate: miniclaw version 2>/dev/null
    // Should show version on stdout

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"));
}

#[test]
fn test_unknown_command_stderr_output() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("invalid-command-that-does-not-exist");

    // AC#3: error message goes to stderr, non-zero exit code
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("unknown")));
}

#[test]
fn test_verbose_mode_stream_separation() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // AC#4: DEBUG logs to stderr, version output to stdout
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stdout(predicate::str::contains("DEBUG").not()) // No DEBUG in stdout
        .stderr(predicate::str::contains("DEBUG")); // DEBUG in stderr
}

#[test]
fn test_verbose_mode_no_log_leak_to_stdout() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // Verify stdout contains only version, no log artifacts
    let output = cmd.output().expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Stdout should contain version but no timestamps (logs have timestamps)
    assert!(stdout.contains("miniclaw"), "stdout should contain version");
    assert!(!stdout.contains("DEBUG"), "stdout should not contain DEBUG");
    assert!(!stdout.contains("INFO"), "stdout should not contain INFO");

    // Stderr should contain DEBUG logs
    assert!(stderr.contains("DEBUG"), "stderr should contain DEBUG logs");
}

#[test]
fn test_default_mode_no_stderr_output() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");

    // AC#2: no stderr output on success in default mode
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_piping_stdout_only() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");

    // AC#5: only stdout is piped (captured), stderr separate
    let output = cmd.output().expect("Failed to execute command");

    // Stdout should be piped (contains version)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("miniclaw"), "stdout should contain version");

    // Stderr should be empty in default mode
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty in default mode");
}

#[test]
fn test_version_with_model_info() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");

    // Version command should show version and optionally model
    let output = cmd.output().expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify stream separation
    assert!(
        stdout.contains("miniclaw"),
        "stdout should contain version string"
    );
    assert!(stderr.is_empty(), "stderr should be empty");
}

#[test]
fn test_help_stdout_only() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--help");

    // Help should go to stdout with no stderr
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stdout(predicate::str::contains("--verbose"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_error_exit_code() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("nonexistent-command");

    // AC#3: non-zero exit code for errors
    cmd.assert().failure();
}

#[test]
fn test_invalid_flag_exit_code() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--invalid-flag-that-does-not-exist");

    // AC#3: exit code 2 for CLI argument errors
    cmd.assert().code(2);
}

#[test]
fn test_verbose_logs_to_stderr_not_stdout() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.args(["--verbose", "version"]);

    let output = cmd.output().expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify separation:
    // - stdout: version string only (no timestamps, no log levels)
    // - stderr: logs with timestamps and levels

    // Stdout checks
    assert!(stdout.contains("miniclaw"), "stdout should have version");
    assert!(!stdout.contains("DEBUG"), "stdout should not have DEBUG");
    assert!(!stdout.contains("INFO"), "stdout should not have INFO");

    // Stderr checks - should have structured log format
    assert!(stderr.contains("DEBUG"), "stderr should have DEBUG logs");

    // The stderr should contain log formatting markers (ANSI codes or structured format)
    // We don't check for specific format, just that logs are there
}
