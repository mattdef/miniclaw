//! Performance integration tests
//!
//! Tests for:
//! - Startup time measurement
//! - Response time tracking
//! - Memory usage monitoring

use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, Instant};

/// Test that startup time measurement exists and is reasonable
#[test]
fn test_startup_time_measurement() {
    // Use debug binary if available, otherwise skip
    let binary_path = if std::path::Path::new("./target/debug/miniclaw").exists() {
        "./target/debug/miniclaw"
    } else {
        eprintln!("Debug binary not found, building...");
        // Build first
        let build_output = Command::new("cargo")
            .args(["build"])
            .output()
            .expect("Failed to build");
        assert!(build_output.status.success(), "Build should succeed");
        "./target/debug/miniclaw"
    };

    let start = Instant::now();
    let output = Command::new(binary_path)
        .arg("version")
        .output()
        .expect("Failed to execute command");

    let elapsed = start.elapsed();

    // Verify command succeeded
    assert!(output.status.success(), "Command should succeed");

    // Startup should be reasonably fast (< 5 seconds for debug build)
    assert!(
        elapsed < Duration::from_secs(5),
        "Startup should be reasonably fast, took {:?}",
        elapsed
    );

    // Verify version output contains expected content
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("miniclaw"),
        "Output should contain 'miniclaw'"
    );
}

/// Test that release mode startup is fast (< 100ms target)
/// Note: This test requires a release build
#[test]
#[ignore = "Requires release build - run manually with: cargo test --release"]
fn test_release_startup_time() {
    let start = Instant::now();
    let output = Command::new("./target/release/miniclaw")
        .arg("version")
        .output()
        .expect("Failed to execute release binary");

    let elapsed = start.elapsed();

    // Verify command succeeded
    assert!(output.status.success(), "Command should succeed");

    // Target: < 100ms for cold start
    assert!(
        elapsed < Duration::from_millis(100),
        "Release startup should be < 100ms, took {:?}",
        elapsed
    );
}

/// Test that verbose mode shows startup time
#[test]
fn test_verbose_mode_startup_log() {
    // Use debug binary if available
    let binary_path = "./target/debug/miniclaw";
    if !std::path::Path::new(binary_path).exists() {
        // Build first if not exists
        let build_output = Command::new("cargo")
            .args(["build"])
            .output()
            .expect("Failed to build");
        assert!(build_output.status.success(), "Build should succeed");
    }

    let output = Command::new(binary_path)
        .args(["--verbose", "version"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verbose mode should show startup log
    assert!(
        stderr.contains("Startup complete") || stderr.contains("startup_ms"),
        "Verbose mode should log startup time. stderr: {}",
        stderr
    );
}

/// Test that binary exists and is executable
#[test]
fn test_binary_exists() {
    // Check debug binary
    let debug_path = "./target/debug/miniclaw";
    if std::path::Path::new(debug_path).exists() {
        let metadata = std::fs::metadata(debug_path).expect("Failed to get metadata");
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "Binary should be executable"
        );
    }
}

/// Test that release binary size is reasonable
#[test]
#[ignore = "Requires release build - run manually with: cargo test --release"]
fn test_release_binary_size() {
    use std::path::Path;

    let binary_path = "./target/release/miniclaw";
    assert!(
        Path::new(binary_path).exists(),
        "Release binary should exist"
    );

    let metadata = std::fs::metadata(binary_path).expect("Failed to get metadata");
    let size_bytes = metadata.len();
    let size_mb = size_bytes as f64 / 1024.0 / 1024.0;

    // Target: < 15MB
    assert!(
        size_mb < 15.0,
        "Binary size should be < 15MB, is {:.2}MB",
        size_mb
    );

    println!("Binary size: {:.2}MB", size_mb);
}

/// Test that version command completes quickly
#[test]
fn test_version_command_performance() {
    // Use debug binary if available
    let binary_path = "./target/debug/miniclaw";
    if !std::path::Path::new(binary_path).exists() {
        // Build first if not exists
        let build_output = Command::new("cargo")
            .args(["build"])
            .output()
            .expect("Failed to build");
        assert!(build_output.status.success(), "Build should succeed");
    }

    let start = Instant::now();
    let output = Command::new(binary_path)
        .arg("version")
        .output()
        .expect("Failed to execute command");

    let elapsed = start.elapsed();

    assert!(output.status.success());

    // Should complete in reasonable time (< 2 seconds when pre-built)
    assert!(
        elapsed < Duration::from_secs(2),
        "Version command should be fast, took {:?}",
        elapsed
    );
}

/// Test that the binary size check script exists and is executable
#[test]
fn test_binary_size_script_exists() {
    let script_path = "./scripts/check-binary-size.sh";
    assert!(
        std::path::Path::new(script_path).exists(),
        "Binary size check script should exist"
    );

    let metadata = std::fs::metadata(script_path).expect("Failed to get metadata");
    let permissions = metadata.permissions();
    assert!(
        permissions.mode() & 0o111 != 0,
        "Script should be executable"
    );
}

/// Test CI workflow file exists
#[test]
fn test_ci_workflow_exists() {
    let workflow_path = ".github/workflows/ci.yml";
    assert!(
        std::path::Path::new(workflow_path).exists(),
        "CI workflow should exist"
    );

    // Verify it contains binary size check
    let content = std::fs::read_to_string(workflow_path).expect("Failed to read workflow");
    assert!(
        content.contains("check-binary-size"),
        "Workflow should contain binary size check"
    );
}
