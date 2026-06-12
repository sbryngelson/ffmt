//! Regression tests for bug-hunt findings (2026-06) owned by the
//! cli/lsp/config agent:
//!
//! 1. LSP rangeFormatting: an exclusive end position at character 0 of line N
//!    must NOT format line N.
//! 2. `--diff` must emit valid unified diffs that `patch` can apply, and
//!    EOL-only changes must not produce an empty diff.
//! 3. The CLI must resolve config per file (nearest ffmt.toml), not once from
//!    the first path argument.
//! 4. An unknown key in ffmt.toml must be a loud error on the CLI path (exit
//!    non-zero, key named on stderr), not a silent fall-back to defaults.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_ffmt");

/// Create a unique temp dir for a test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("ffmt-bughunt-tests")
        .join(format!("{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// --- LSP helpers -----------------------------------------------------------

fn frame(msg: &serde_json::Value) -> Vec<u8> {
    let body = msg.to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn read_frames(mut data: &[u8]) -> Vec<serde_json::Value> {
    let mut frames = Vec::new();
    while let Some(pos) = find_subslice(data, b"\r\n\r\n") {
        let headers = std::str::from_utf8(&data[..pos]).unwrap();
        let len: usize = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .expect("missing Content-Length header")
            .trim()
            .parse()
            .unwrap();
        let body = &data[pos + 4..pos + 4 + len];
        frames.push(serde_json::from_slice(body).unwrap());
        data = &data[pos + 4 + len..];
    }
    frames
}

/// Run an LSP session: send the given messages, close stdin, collect all
/// response frames.
fn lsp_session(messages: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut child = Command::new(BIN)
        .arg("--lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for msg in messages {
            stdin.write_all(&frame(msg)).unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    read_frames(&out.stdout)
}

fn range_formatting_newtext(text: &str, range: serde_json::Value) -> String {
    let uri = "file:///nonexistent-dir-for-default-config/t.f90";
    let responses = lsp_session(&[
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "text": text}}
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/rangeFormatting",
            "params": {"textDocument": {"uri": uri}, "range": range}
        }),
    ]);
    let resp = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("no response to rangeFormatting");
    let result = &resp["result"];
    if result.as_array().is_some_and(|a| a.is_empty()) {
        return text.to_string();
    }
    result[0]["newText"].as_str().unwrap().to_string()
}

// --- 1. LSP rangeFormatting end exclusivity --------------------------------

#[test]
fn lsp_range_end_character_zero_excludes_that_line() {
    // Select exactly the line 'x=1' (0-based line 1). Per LSP, the end
    // position {line: 2, character: 0} is exclusive: line 2 ('y=2') is NOT
    // part of the selection and must not be reformatted.
    let text = "program t\nx=1\ny=2\nend program t\n";
    let new_text = range_formatting_newtext(
        text,
        serde_json::json!({
            "start": {"line": 1, "character": 0},
            "end": {"line": 2, "character": 0}
        }),
    );
    assert!(
        new_text.contains("x = 1"),
        "line in range was not formatted:\n{new_text}"
    );
    assert!(
        new_text.contains("\ny=2\n"),
        "line 2 ('y=2') is outside the exclusive end and must be untouched:\n{new_text}"
    );
}

#[test]
fn lsp_range_end_with_nonzero_character_includes_that_line() {
    // End position {line: 2, character: 3} means line 2 IS partially selected
    // and must be formatted.
    let text = "program t\nx=1\ny=2\nend program t\n";
    let new_text = range_formatting_newtext(
        text,
        serde_json::json!({
            "start": {"line": 1, "character": 0},
            "end": {"line": 2, "character": 3}
        }),
    );
    assert!(new_text.contains("x = 1"), "got:\n{new_text}");
    assert!(
        new_text.contains("y = 2"),
        "line 2 is inside the range and must be formatted:\n{new_text}"
    );
}

#[test]
fn lsp_range_single_line_with_end_character_zero_still_formats_start_line() {
    // Degenerate empty selection on one line: still format that line rather
    // than nothing (matches editor expectations for format-at-cursor).
    let text = "program t\nx=1\ny=2\nend program t\n";
    let new_text = range_formatting_newtext(
        text,
        serde_json::json!({
            "start": {"line": 1, "character": 0},
            "end": {"line": 1, "character": 0}
        }),
    );
    assert!(new_text.contains("x = 1"), "got:\n{new_text}");
    assert!(new_text.contains("\ny=2\n"), "got:\n{new_text}");
}

// --- 2. --diff unified diff validity ----------------------------------------

/// Check every hunk header's counts against the actual number of -/+ lines
/// in its body.
fn assert_hunk_counts_consistent(diff: &str) {
    let mut lines = diff.lines().peekable();
    let mut saw_hunk = false;
    while let Some(line) = lines.next() {
        if !line.starts_with("@@ ") {
            continue;
        }
        saw_hunk = true;
        // Parse '@@ -a,b +c,d @@'
        let inner = line.trim_start_matches("@@ ").trim_end_matches(" @@");
        let mut parts = inner.split(' ');
        let old = parts.next().unwrap().trim_start_matches('-');
        let new = parts.next().unwrap().trim_start_matches('+');
        let old_count: usize = old.split(',').nth(1).unwrap_or("1").parse().unwrap();
        let new_count: usize = new.split(',').nth(1).unwrap_or("1").parse().unwrap();

        let mut minus = 0usize;
        let mut plus = 0usize;
        while let Some(&next) = lines.peek() {
            if next.starts_with('-') && !next.starts_with("---") {
                minus += 1;
            } else if next.starts_with('+') && !next.starts_with("+++") {
                plus += 1;
            } else if next.starts_with('\\') || next.starts_with(' ') {
                // no-newline marker / context line
            } else {
                break;
            }
            lines.next();
        }
        assert_eq!(
            old_count, minus,
            "hunk header old count does not match body in:\n{diff}"
        );
        assert_eq!(
            new_count, plus,
            "hunk header new count does not match body in:\n{diff}"
        );
    }
    assert!(saw_hunk, "expected at least one hunk in diff:\n{diff}");
}

#[test]
fn diff_hunk_headers_match_body_counts() {
    let dir = temp_dir("diff-counts");
    fs::write(dir.join("a.f90"), "program t\nx=1\nend program t\n").unwrap();
    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "--diff", "--color", "never", "a.f90"])
        .output()
        .unwrap();
    let diff = String::from_utf8_lossy(&out.stdout);
    assert_hunk_counts_consistent(&diff);
}

#[test]
fn diff_output_applies_cleanly_with_patch() {
    let dir = temp_dir("diff-patch");
    let source = "program t\nx=1\nend program t\n";
    fs::write(dir.join("a.f90"), source).unwrap();
    // Reference: what ffmt would write in place.
    fs::write(dir.join("b.f90"), source).unwrap();
    let st = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "b.f90"])
        .status()
        .unwrap();
    assert!(st.success());
    let expected = fs::read_to_string(dir.join("b.f90")).unwrap();

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "--diff", "--color", "never", "a.f90"])
        .output()
        .unwrap();
    assert!(!out.stdout.is_empty(), "expected a non-empty diff");

    // Apply the diff with patch(1); it must succeed and reproduce the
    // formatted file exactly.
    let mut patch = Command::new("patch")
        .current_dir(&dir)
        .args(["-p0", "a.f90"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    patch
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&out.stdout)
        .unwrap();
    let patch_out = patch.wait_with_output().unwrap();
    assert!(
        patch_out.status.success(),
        "patch rejected ffmt --diff output:\nstdout: {}\nstderr: {}\ndiff:\n{}",
        String::from_utf8_lossy(&patch_out.stdout),
        String::from_utf8_lossy(&patch_out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    let patched = fs::read_to_string(dir.join("a.f90")).unwrap();
    assert_eq!(
        patched, expected,
        "patched file differs from ffmt's own output"
    );
}

#[test]
fn eol_only_change_produces_nonempty_diff() {
    let dir = temp_dir("diff-eol");
    // Build content that is already formatted, then re-encode it with CRLF
    // line endings: the ONLY change ffmt makes is CRLF -> LF.
    let source = "program t\nx=1\nend program t\n";
    fs::write(dir.join("b.f90"), source).unwrap();
    let st = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "b.f90"])
        .status()
        .unwrap();
    assert!(st.success());
    let formatted = fs::read_to_string(dir.join("b.f90")).unwrap();
    let crlf = formatted.replace('\n', "\r\n");
    fs::write(dir.join("a.f90"), &crlf).unwrap();

    // --check must flag the file...
    let check = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "--check", "a.f90"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1), "--check must flag CRLF file");

    // ...and --diff must show the change rather than printing empty headers.
    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "--diff", "--color", "never", "a.f90"])
        .output()
        .unwrap();
    let diff = String::from_utf8_lossy(&out.stdout);
    assert!(
        diff.contains("@@"),
        "--diff printed no hunks for an EOL-only change:\n{diff}"
    );
    assert_hunk_counts_consistent(&diff);
}

// --- 3. Per-file config resolution ------------------------------------------

#[test]
fn cli_resolves_config_per_file_in_subdirectories() {
    let dir = temp_dir("per-file-config");
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("ffmt.toml"), "indent-width = 8\n").unwrap();
    fs::write(sub.join("a.f90"), "program t\nx=1\nend program t\n").unwrap();

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "."])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let formatted = fs::read_to_string(sub.join("a.f90")).unwrap();
    assert!(
        formatted.contains("\n        x = 1\n"),
        "sub/ffmt.toml (indent-width = 8) was ignored; got:\n{formatted}"
    );
}

#[test]
fn cli_resolves_config_per_path_argument() {
    let dir = temp_dir("per-path-config");
    for (name, width) in [("a", "2"), ("b", "8")] {
        let d = dir.join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("ffmt.toml"), format!("indent-width = {width}\n")).unwrap();
        fs::write(d.join("f.f90"), "program t\nx=1\nend program t\n").unwrap();
    }

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "a", "b"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a = fs::read_to_string(dir.join("a/f.f90")).unwrap();
    let b = fs::read_to_string(dir.join("b/f.f90")).unwrap();
    assert!(
        a.contains("\n  x = 1\n"),
        "a/ffmt.toml (indent-width = 2) was ignored; got:\n{a}"
    );
    assert!(
        b.contains("\n        x = 1\n"),
        "b/ffmt.toml (indent-width = 8) was ignored; got:\n{b}"
    );
}

// --- 4. Loud errors for unknown config keys ---------------------------------

#[test]
fn cli_unknown_config_key_is_a_hard_error() {
    let dir = temp_dir("bad-config-key");
    fs::write(
        dir.join("ffmt.toml"),
        "indent-width = 2\nbogus-key = true\n",
    )
    .unwrap();
    let source = "program t\nx=1\nend program t\n";
    fs::write(dir.join("a.f90"), source).unwrap();

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "a.f90"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        out.status.code(),
        Some(0),
        "an unknown config key must not be silently ignored (stderr: {stderr})"
    );
    assert!(
        stderr.contains("bogus-key"),
        "error must name the offending key; stderr:\n{stderr}"
    );
    // The file must NOT have been rewritten with default settings.
    assert_eq!(
        fs::read_to_string(dir.join("a.f90")).unwrap(),
        source,
        "file was reformatted despite a broken config"
    );
}

#[test]
fn cli_unknown_config_key_fails_check_mode_loudly() {
    let dir = temp_dir("bad-config-key-check");
    fs::write(dir.join("ffmt.toml"), "bogus-key = true\n").unwrap();
    fs::write(dir.join("a.f90"), "program t\nx = 1\nend program t\n").unwrap();

    let out = Command::new(BIN)
        .current_dir(&dir)
        .args(["--no-cache", "--check", "a.f90"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--check with a broken config must exit 2, not pretend defaults are the project style (stderr: {stderr})"
    );
    assert!(stderr.contains("bogus-key"), "stderr:\n{stderr}");
}

#[test]
fn config_try_find_and_load_reports_unknown_key() {
    let dir = temp_dir("config-unit-bad");
    fs::write(
        dir.join("ffmt.toml"),
        "indent-width = 2\nbogus-key = true\n",
    )
    .unwrap();
    let err =
        ffmt::config::Config::try_find_and_load(&dir).expect_err("unknown key must be an error");
    assert!(
        err.contains("bogus-key"),
        "error must name the offending key, got: {err}"
    );
}

#[test]
fn config_try_find_and_load_parses_valid_config() {
    let dir = temp_dir("config-unit-good");
    fs::write(dir.join("ffmt.toml"), "indent-width = 2\n").unwrap();
    let cfg = ffmt::config::Config::try_find_and_load(&dir).unwrap();
    assert_eq!(cfg.indent_width, 2);
}

#[test]
fn config_try_find_and_load_defaults_when_no_config_file() {
    let dir = temp_dir("config-unit-none");
    let cfg = ffmt::config::Config::try_find_and_load(&dir).unwrap();
    assert_eq!(cfg.indent_width, 4);
}

#[test]
fn config_find_and_load_stays_lenient_for_lsp() {
    // The LSP path must not die on a broken config: find_and_load logs and
    // falls back to defaults.
    let dir = temp_dir("config-unit-lenient");
    fs::write(dir.join("ffmt.toml"), "bogus-key = true\n").unwrap();
    let cfg = ffmt::config::Config::find_and_load(&dir);
    assert_eq!(cfg.indent_width, 4);
}
