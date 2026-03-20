use std::fs;

#[test]
fn test_ffmt_toml_indent_width() {
    let dir = std::env::temp_dir().join("ffmt_test_config_1");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Write config
    fs::write(dir.join("ffmt.toml"), r#"indent-width = 2"#).unwrap();

    // Write test file
    fs::write(dir.join("test.f90"), "module m\nimplicit none\nend module\n").unwrap();

    // Format using the binary
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    assert!(result.contains("  implicit none"), "Expected 2-space indent, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_ffmt_toml_keyword_case_preserve() {
    let dir = std::env::temp_dir().join("ffmt_test_config_2");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("ffmt.toml"), r#"keyword-case = "preserve""#).unwrap();
    fs::write(dir.join("test.f90"), "MODULE m\nIMPLICIT NONE\nEND MODULE\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    assert!(result.contains("MODULE"), "Keywords should be preserved, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_ffmt_toml_whitespace_multdiv() {
    let dir = std::env::temp_dir().join("ffmt_test_config_3");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("ffmt.toml"), "[whitespace]\nmultdiv = true").unwrap();
    fs::write(dir.join("test.f90"), "x = a*b\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    assert!(result.contains("a * b"), "Expected spaces around *, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_pyproject_toml_tool_ffmt() {
    let dir = std::env::temp_dir().join("ffmt_test_config_4");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("pyproject.toml"), "[tool.ffmt]\nindent-width = 3\n").unwrap();
    fs::write(dir.join("test.f90"), "module m\nimplicit none\nend module\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    assert!(result.contains("   implicit none"), "Expected 3-space indent, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_no_config_uses_defaults() {
    let dir = std::env::temp_dir().join("ffmt_test_config_5");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // No config file
    fs::write(dir.join("test.f90"), "MODULE m\nIMPLICIT NONE\nEND MODULE\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    // Default: 4-space indent, lowercase keywords
    assert!(result.contains("    implicit none"), "Expected 4-space indent lowercase, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_normalize_keywords_false() {
    let dir = std::env::temp_dir().join("ffmt_test_config_6");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("ffmt.toml"), "normalize-keywords = false\n").unwrap();
    fs::write(dir.join("test.f90"), "enddo\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    // Should stay as "enddo" (lowercased but not split)
    assert!(result.contains("enddo"), "enddo should not be split, got: {}", result);

    fs::remove_dir_all(&dir).ok();
}
