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
