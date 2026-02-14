use std::process::Command;

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("miniclaw"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_version_flag_long() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("miniclaw"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_version_flag_short() {
    let output = Command::new("cargo")
        .args(["run", "--", "-V"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("miniclaw"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_version_exit_code() {
    let output = Command::new("cargo")
        .args(["run", "--", "version"])
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(0));
}

// CLI Framework Setup Tests - Story 1.2

#[test]
fn test_no_args_shows_help() {
    let output = Command::new("cargo")
        .args(["run", "--"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Should exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:") || stdout.contains("Commands:"),
        "Should display help message"
    );
}

#[test]
fn test_no_args_exit_code() {
    let output = Command::new("cargo")
        .args(["run", "--"])
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(0), "Should exit with code 0");
}

#[test]
fn test_invalid_command_shows_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid_command"])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail with non-zero exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown command") || stderr.contains("error"),
        "Should show unknown command error"
    );
}

#[test]
fn test_invalid_command_exit_code() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid_command"])
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(1), "Should exit with code 1");
}

#[test]
fn test_invalid_flag_shows_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "version", "--invalid"])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail with non-zero exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("unrecognized") || stderr.contains("invalid"),
        "Should show error about invalid flag"
    );
}

#[test]
fn test_invalid_flag_exit_code() {
    let output = Command::new("cargo")
        .args(["run", "--", "version", "--invalid"])
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2), "Should exit with code 2");
}

#[test]
fn test_errors_go_to_stderr() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid_command"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stderr.is_empty(), "Errors should go to stderr");
    assert!(
        !stdout.contains("error") && !stdout.contains("unknown command"),
        "Errors should not go to stdout"
    );
}

#[test]
fn test_help_goes_to_stdout() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("Usage:") || stdout.contains("Commands:"),
        "Help should go to stdout"
    );
    assert!(
        !stderr.contains("Usage:") && !stderr.contains("Commands:"),
        "Help should not go to stderr"
    );
}
