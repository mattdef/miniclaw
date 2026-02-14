use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("miniclaw").unwrap()
}

#[test]
fn test_main_help_shows_about_and_commands() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("miniclaw"))
        .stdout(predicate::str::contains("version"))
        .stdout(predicate::str::contains("--verbose"))
        .stdout(predicate::str::contains("-v"))
        .stdout(predicate::str::contains("--help"))
        .stdout(predicate::str::contains("-h"));
}

#[test]
fn test_help_command_same_as_help_flag() {
    let output1 = cmd().arg("--help").output().unwrap();
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    let output2 = cmd().arg("help").output().unwrap();
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    assert!(
        stdout1.contains("miniclaw"),
        "Expected --help to contain 'miniclaw'"
    );
    assert!(
        stdout2.contains("miniclaw"),
        "Expected help to contain 'miniclaw'"
    );
    assert!(
        stdout1.contains("version"),
        "Expected --help to contain 'version'"
    );
    assert!(
        stdout2.contains("version"),
        "Expected help to contain 'version'"
    );
}

#[test]
fn test_version_command_help() {
    cmd()
        .args(["version", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Display version information"));
}

#[test]
fn test_onboard_command_help() {
    cmd()
        .args(["onboard", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize"))
        .stdout(predicate::str::contains("workspace"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"))
        .stdout(predicate::str::contains("--path"))
        .stdout(predicate::str::contains("-p"));
}

#[test]
fn test_help_exit_code_is_zero() {
    cmd().arg("--help").assert().success().code(0);
}

#[test]
fn test_help_shows_usage_line() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_help_shows_commands_section() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_help_shows_options_section() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Options:"));
}

#[test]
fn test_help_onboard_command_syntax() {
    cmd()
        .args(["help", "onboard"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize"))
        .stdout(predicate::str::contains("workspace"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("-y"))
        .stdout(predicate::str::contains("--path"))
        .stdout(predicate::str::contains("-p"));
}
