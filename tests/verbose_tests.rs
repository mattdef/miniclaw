#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_verbose_flag_enables_debug_logging() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // In verbose mode, DEBUG logs go to stderr, version string to stdout
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stderr(predicate::str::contains("DEBUG"));
}

#[test]
fn test_short_verbose_flag_works() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("-v").arg("version");

    // -v should behave the same as --verbose
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stderr(predicate::str::contains("DEBUG"));
}

#[test]
fn test_default_mode_shows_only_info() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");

    // Default mode should NOT show DEBUG logs in stdout or stderr
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stdout(predicate::str::contains("DEBUG").not())
        .stderr(predicate::str::contains("DEBUG").not());
}

#[test]
fn test_log_format_includes_timestamp() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // Log in stderr should contain timestamp in RFC 3339 format (contains T)
    cmd.assert().success().stderr(predicate::str::contains("T"));
}

#[test]
fn test_log_format_includes_level() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // Log in stderr should contain log level (DEBUG)
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"));
}

#[test]
fn test_log_format_includes_module() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--verbose").arg("version");

    // Log in stderr should contain module path (miniclaw::)
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("miniclaw"));
}

#[test]
fn test_help_shows_verbose_option() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--verbose"))
        .stdout(predicate::str::contains("-v"))
        .stdout(predicate::str::contains("verbose"));
}

#[test]
#[ignore = "AC4: Requires config loading implementation (future story)"]
fn test_sensitive_data_not_logged_in_verbose_mode() {
    todo!(
        "When config loading is implemented, verify:
    1. API keys are never logged (use boolean indicators)
    2. Tokens are never logged
    3. Sensitive paths are redacted
    4. Passwords are never logged"
    );
}
