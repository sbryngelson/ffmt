//! CLI regression tests for bugs found in code review (2026-06).

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_ffmt");

/// Create a unique temp dir for a test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ffmt-cli-tests").join(format!(
        "{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_dir_walk_respects_configured_extensions() {
    let dir = temp_dir("extensions");
    fs::write(dir.join("ffmt.toml"), "[files]\nextensions = [\"f\"]\n").unwrap();
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("a.f"), "program t\nx=1\nend program t\n").unwrap();

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--check", "sub"])
        .output()
        .unwrap();
    // The .f file needs formatting, so --check must find it and exit 1.
    // (Bug: dir walk used default extensions, found nothing, exited 2.)
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_stdin_diff_without_check_exits_zero() {
    let mut child = Command::new(BIN)
        .args(["-", "--diff"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"program t\nx=1\nend program t\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    // --diff alone is informational: print the diff, exit 0.
    assert_eq!(out.status.code(), Some(0));
    assert!(!out.stdout.is_empty(), "expected a diff on stdout");
}

#[test]
fn test_stdin_check_still_exits_one_on_change() {
    let mut child = Command::new(BIN)
        .args(["-", "--check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"program t\nx=1\nend program t\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[cfg(unix)]
#[test]
fn test_write_failure_exits_nonzero_and_is_retried() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("write-failure");
    let file = dir.join("a.f90");
    fs::write(&file, "program t\nx=1\nend program t\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();
    let cache = dir.join("cache");
    let cache_arg = cache.to_str().unwrap();

    // Run 1: write fails -> nonzero exit, error on stderr.
    let out1 = Command::new(BIN)
        .current_dir(&dir)
        .args(["--cache-dir", cache_arg, "a.f90"])
        .output()
        .unwrap();
    assert_ne!(
        out1.status.code(),
        Some(0),
        "write failure must produce a nonzero exit code"
    );
    assert!(!out1.stderr.is_empty());

    // Run 2: the failed file must NOT be cache-skipped — same failure again.
    let out2 = Command::new(BIN)
        .current_dir(&dir)
        .args(["--cache-dir", cache_arg, "a.f90"])
        .output()
        .unwrap();
    assert_ne!(
        out2.status.code(),
        Some(0),
        "failed file was silently cache-skipped on the second run"
    );
    assert!(
        !out2.stderr.is_empty(),
        "failed file was silently cache-skipped on the second run (no error reported)"
    );

    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
}
