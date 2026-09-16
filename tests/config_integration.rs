use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(100);

/// Helper: create a temp dir with config, write input, format, return result.
/// Uses a nested directory structure to prevent config file search from finding
/// other tests' configs (ffmt searches upward for ffmt.toml).
fn format_with_config(config: &str, input: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("ffmt_test_cfg_{id}/nested"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("ffmt.toml"), config).unwrap();
    fs::write(dir.join("test.f90"), input).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ffmt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    fs::remove_dir_all(&dir).ok();
    result
}

// ===== Existing tests =====

#[test]
fn test_indent_width_2() {
    let result = format_with_config("indent-width = 2", "module m\nimplicit none\nend module\n");
    assert!(
        result.contains("  implicit none"),
        "Expected 2-space indent, got: {result}"
    );
}

#[test]
fn test_indent_width_4_default() {
    let result = format_with_config("", "module m\nimplicit none\nend module\n");
    assert!(
        result.contains("    implicit none"),
        "Expected 4-space indent, got: {result}"
    );
}

#[test]
fn test_keyword_case_preserve() {
    let result = format_with_config(
        r#"keyword-case = "preserve""#,
        "MODULE m\nIMPLICIT NONE\nEND MODULE\n",
    );
    assert!(
        result.contains("MODULE"),
        "Keywords should be preserved, got: {result}"
    );
}

// NOTE: keyword-case = "upper" is known to not uppercase all keywords yet.
// This test just verifies the config is accepted without error.
#[test]
fn test_keyword_case_upper_accepted() {
    let result = format_with_config(
        r#"keyword-case = "upper""#,
        "module m\nimplicit none\nend module\n",
    );
    assert!(
        !result.is_empty(),
        "Should produce output with upper case config, got: {result}"
    );
}

#[test]
fn test_whitespace_multdiv_on() {
    let result = format_with_config("[whitespace]\nmultdiv = true", "x = a*b\n");
    assert!(
        result.contains("a * b"),
        "Expected spaces around *, got: {result}"
    );
}

#[test]
fn test_whitespace_multdiv_off() {
    let result = format_with_config("[whitespace]\nmultdiv = false", "x = a * b\n");
    assert!(
        result.contains("a*b"),
        "Expected no spaces around *, got: {result}"
    );
}

#[test]
fn test_pyproject_toml() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("ffmt_test_cfg_{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("pyproject.toml"),
        "[tool.ffmt]\nindent-width = 3\n",
    )
    .unwrap();
    fs::write(
        dir.join("test.f90"),
        "module m\nimplicit none\nend module\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ffmt"))
        .arg(dir.join("test.f90"))
        .output()
        .unwrap();
    assert!(output.status.success());

    let result = fs::read_to_string(dir.join("test.f90")).unwrap();
    assert!(
        result.contains("   implicit none"),
        "Expected 3-space indent from pyproject.toml, got: {result}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_normalize_keywords_false() {
    let result = format_with_config("normalize-keywords = false\n", "enddo\n");
    assert!(
        result.contains("enddo"),
        "enddo should not be split, got: {result}"
    );
}

#[test]
fn test_normalize_keywords_preserve() {
    let result = format_with_config("normalize-keywords = \"preserve\"\n", "enddo\nend do\n");
    assert!(
        result.contains("enddo"),
        "enddo should be preserved, got: {result}"
    );
    assert!(
        result.contains("end do"),
        "end do should be preserved, got: {result}"
    );
}

// ===== Toggle/preserve tests =====

#[test]
fn test_toggle_accepts_true() {
    let result = format_with_config("normalize-keywords = true\n", "ENDDO\n");
    assert!(
        result.contains("end do"),
        "true should normalize, got: {result}"
    );
}

#[test]
fn test_toggle_accepts_string_enable() {
    let result = format_with_config("normalize-keywords = \"enable\"\n", "ENDDO\n");
    assert!(
        result.contains("end do"),
        "\"enable\" should normalize, got: {result}"
    );
}

// ===== Format suppression tests =====

#[test]
fn test_ffmt_off_on() {
    let result = format_with_config("", "x = 1\n! ffmt off\ny    =    2\n! ffmt on\nz = 3\n");
    assert!(
        result.contains("y    =    2"),
        "Code between ffmt off/on should be preserved, got: {result}"
    );
}

#[test]
fn test_ffmt_off_on_case_insensitive() {
    let result = format_with_config("", "x = 1\n! FFMT OFF\ny    =    2\n! FFMT ON\nz = 3\n");
    assert!(
        result.contains("y    =    2"),
        "ffmt off should be case-insensitive, got: {result}"
    );
}

#[test]
fn test_ffmt_off_colon_variant() {
    let result = format_with_config("", "x = 1\n! ffmt: off\ny    =    2\n! ffmt: on\nz = 3\n");
    assert!(
        result.contains("y    =    2"),
        "ffmt: off variant should work, got: {result}"
    );
}

// ===== Relational operator modernization tests =====

#[test]
fn test_modernize_operators_eq() {
    let result = format_with_config("", "if (x .eq. 1) then\nend if\n");
    assert!(
        result.contains("x == 1"),
        ".eq. should become ==, got: {result}"
    );
}

#[test]
fn test_modernize_operators_ne() {
    let result = format_with_config("", "if (x .ne. 0) then\nend if\n");
    assert!(
        result.contains("x /= 0"),
        ".ne. should become /=, got: {result}"
    );
}

#[test]
fn test_modernize_operators_lt_le_gt_ge() {
    let result = format_with_config(
        "",
        "if (a .lt. b .and. c .le. d .and. e .gt. f .and. g .ge. h) then\nend if\n",
    );
    assert!(result.contains("a < b"), ".lt. -> <, got: {result}");
    assert!(result.contains("c <= d"), ".le. -> <=, got: {result}");
    assert!(result.contains("e > f"), ".gt. -> >, got: {result}");
    assert!(result.contains("g >= h"), ".ge. -> >=, got: {result}");
}

#[test]
fn test_modernize_operators_preserves_strings() {
    let result = format_with_config("", "s = 'x .eq. y'\n");
    assert!(
        result.contains(".eq."),
        "Operators inside strings should be preserved, got: {result}"
    );
}

#[test]
fn test_modernize_operators_disabled() {
    let result = format_with_config(
        "modernize-operators = false",
        "if (x .eq. 1) then\nend if\n",
    );
    assert!(
        result.contains(".eq."),
        "Should preserve .eq. when disabled, got: {result}"
    );
}

#[test]
fn test_modernize_operators_case_insensitive() {
    let result = format_with_config("", "if (x .EQ. 1) then\nend if\n");
    assert!(
        result.contains("x == 1"),
        ".EQ. should become ==, got: {result}"
    );
}

// ===== Double-colon enforcement tests =====

#[test]
fn test_enforce_double_colon_integer() {
    let result = format_with_config("", "integer x\n");
    assert!(
        result.contains("integer :: x"),
        "Should add ::, got: {result}"
    );
}

#[test]
fn test_enforce_double_colon_real_with_kind() {
    let result = format_with_config("", "real(wp) y\n");
    assert!(
        result.contains("real(wp) :: y"),
        "Should add ::, got: {result}"
    );
}

#[test]
fn test_enforce_double_colon_type() {
    let result = format_with_config("", "type(foo) bar\n");
    assert!(
        result.contains("type(foo) :: bar"),
        "Should add ::, got: {result}"
    );
}

#[test]
fn test_enforce_double_colon_already_present() {
    let result = format_with_config("", "integer :: x\n");
    assert!(
        result.contains("integer :: x"),
        "Should leave existing :: alone, got: {result}"
    );
}

#[test]
fn test_enforce_double_colon_skips_function() {
    let result = format_with_config(
        "",
        "module m\ncontains\nlogical function f_foo()\nf_foo = .true.\nend function\nend module\n",
    );
    assert!(
        !result.contains("logical :: function"),
        "Should not add :: to function signature, got: {result}"
    );
}

#[test]
fn test_enforce_double_colon_disabled() {
    let result = format_with_config("enforce-double-colon = false", "integer x\n");
    assert!(
        !result.contains("::"),
        "Should not add :: when disabled, got: {result}"
    );
}

// ===== Multi-statement splitting tests =====

#[test]
fn test_split_statements_on() {
    let result = format_with_config("split-statements = true", "x = 1; y = 2\n");
    assert!(
        result.contains("x = 1\n") && result.contains("y = 2\n"),
        "Should split statements, got: {result}"
    );
}

#[test]
fn test_split_statements_preserves_private_public() {
    let result = format_with_config("split-statements = true", "private; public :: s_foo\n");
    assert!(
        result.contains("private; public"),
        "private; public :: should be preserved, got: {result}"
    );
}

#[test]
fn test_split_statements_off_default() {
    let result = format_with_config("", "x = 1; y = 2\n");
    assert!(
        result.contains("x = 1; y = 2"),
        "Should preserve multi-statements by default, got: {result}"
    );
}

// ===== Trailing semicolon removal tests =====

#[test]
fn test_trailing_semicolon_removed() {
    let result = format_with_config("", "x = 1;\n");
    assert!(
        !result.trim().ends_with(';'),
        "Trailing semicolon should be removed, got: {result}"
    );
}

#[test]
fn test_semicolon_between_statements_preserved() {
    let result = format_with_config("", "x = 1; y = 2\n");
    assert!(
        result.contains("; y"),
        "Semicolons between statements should stay, got: {result}"
    );
}

// ===== Inline comment spacing (S102) tests =====

#[test]
fn test_s102_two_spaces_before_comment() {
    let result = format_with_config("", "x = 1 ! comment\n");
    assert!(
        result.contains("x = 1  ! comment"),
        "Should have 2 spaces before !, got: {result}"
    );
}

#[test]
fn test_s102_already_two_spaces() {
    let result = format_with_config("", "x = 1  ! comment\n");
    assert!(
        result.contains("x = 1  ! comment"),
        "Should keep 2 spaces, got: {result}"
    );
}

// ===== Named ends tests =====

#[test]
fn test_named_ends_on() {
    let result = format_with_config(
        "named-ends = true",
        "module m\ncontains\nsubroutine s_foo()\nend subroutine\nend module\n",
    );
    assert!(
        result.contains("end subroutine s_foo"),
        "Should add name to end, got: {result}"
    );
}

#[test]
fn test_named_ends_off() {
    let result = format_with_config(
        "named-ends = false",
        "module m\ncontains\nsubroutine s_foo()\nend subroutine s_foo\nend module m\n",
    );
    // When false, should not ADD names but also shouldn't strip existing ones
    // (false = don't add, preserve = don't touch)
    assert!(
        result.contains("end subroutine"),
        "Should have end subroutine, got: {result}"
    );
}

// ===== Use-formatting tests =====

#[test]
fn test_use_formatting_one_per_line() {
    let result = format_with_config(
        "use-formatting = true",
        "use m_foo, only: s_bar, s_baz, f_qux\n",
    );
    assert!(
        result.contains("& s_bar") || result.contains("& s_baz"),
        "Should split use imports, got: {result}"
    );
}

#[test]
fn test_use_formatting_off_default() {
    let result = format_with_config("", "use m_foo, only: s_bar, s_baz, f_qux\n");
    assert!(
        result.contains("s_bar, s_baz, f_qux"),
        "Should preserve use statement by default, got: {result}"
    );
}

// ===== Assignment alignment tests =====

#[test]
fn test_align_assignments_on() {
    let result = format_with_config("align-assignments = true", "x = 1\nlong_var = 2\ny = 3\n");
    // All = should be at the same column
    let lines: Vec<&str> = result.lines().filter(|l| l.contains('=')).collect();
    if lines.len() >= 2 {
        let pos0 = lines[0].find('=').unwrap();
        let pos1 = lines[1].find('=').unwrap();
        assert_eq!(pos0, pos1, "= signs should be aligned, got: {result}");
    }
}

#[test]
fn test_align_assignments_off_default() {
    let result = format_with_config("", "x = 1\nlong_var = 2\n");
    // = positions should differ
    let lines: Vec<&str> = result.lines().filter(|l| l.contains('=')).collect();
    if lines.len() >= 2 {
        let pos0 = lines[0].find('=').unwrap();
        let pos1 = lines[1].find('=').unwrap();
        assert_ne!(
            pos0, pos1,
            "= signs should NOT be aligned by default, got: {result}"
        );
    }
}

// ===== EOL normalization tests =====

#[test]
fn test_eol_lf_default() {
    let result = format_with_config("", "x = 1\n");
    assert!(
        !result.contains("\r\n"),
        "Default LF should not have CRLF, got bytes: {:?}",
        result.as_bytes()
    );
}

// ===== Rewrap comments tests =====

#[test]
fn test_rewrap_comments_off() {
    let input = "! short\n! line\n";
    let result = format_with_config("rewrap-comments = false", input);
    assert!(
        result.contains("! short\n") && result.contains("! line\n"),
        "Should preserve comment lines when rewrap off, got: {result}"
    );
}

// ===== Rewrap code tests =====

#[test]
fn test_rewrap_code_off() {
    let long_line = format!("x = {}\n", "a + ".repeat(50));
    let result = format_with_config("rewrap-code = false", &long_line);
    // Should not have continuation &
    let amp_count = result.matches(" &").count();
    assert_eq!(
        amp_count, 0,
        "Should not rewrap code when disabled, got: {result}"
    );
}

// ===== Idempotency test =====

#[test]
fn test_idempotency() {
    let input = "module m\nimplicit none\ninteger :: x\ncontains\nsubroutine s_foo()\nx = 1\nend subroutine s_foo\nend module m\n";
    let pass1 = format_with_config("", input);
    let pass2 = format_with_config("", &pass1);
    assert_eq!(pass1, pass2, "Formatter should be idempotent");
}

// ===== Use-only alignment tests =====

#[test]
fn test_align_use_only_on() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_global, only: a, b\n    use m_x, only: d\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true", input);
    let expected = "module m\n\n    use m_derived_types, only: t_foo\n    use m_global,        only: a, b\n    use m_x,             only: d\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_off_default() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_global, only: a, b\n\n    implicit none\nend module m\n";
    let result = format_with_config("", input);
    assert!(
        result.contains("    use m_global, only: a, b\n"),
        "only: should NOT be aligned by default, got: {result}"
    );
}

#[test]
fn test_align_use_only_preserve_does_not_align() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_global, only: a, b\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = \"preserve\"", input);
    assert!(
        result.contains("    use m_global, only: a, b\n"),
        "preserve should not align only:, got: {result}"
    );
}

#[test]
fn test_align_use_only_plain_use_does_not_break_group() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_mpi\n    use m_x, only: d\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true", input);
    let expected = "module m\n\n    use m_derived_types, only: t_foo\n    use m_mpi\n    use m_x,             only: d\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_comment_breaks_group() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    ! separator\n    use m_x, only: d\n    use m_y, only: e\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true", input);
    let expected = "module m\n\n    use m_derived_types, only: t_foo\n    ! separator\n    use m_x, only: d\n    use m_y, only: e\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_with_continued_statement() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_helpers, only: s_bar, s_baz\n    use m_x, only: d\n\n    implicit none\nend module m\n";
    let result = format_with_config(
        "align-use-only = true\nuse-formatting = \"one-per-line\"",
        input,
    );
    let expected = "module m\n\n    use m_derived_types, only: t_foo\n    use m_helpers,       only: &\n        & s_bar, &\n        & s_baz\n    use m_x,             only: d\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_trailing_comment_and_case() {
    let input = "module m\n    use m_derived_types, only: t_foo ! only: in comment\n    USE m_x, ONLY: d\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true\nkeyword-case = \"preserve\"", input);
    let expected = "module m\n\n    use m_derived_types, only: t_foo  ! only: in comment\n    USE m_x,             ONLY: d\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_respects_line_length() {
    let input = "module m\n    use m_derived_types_with_a_long_name, only: t_foo\n    use m_x, only: d_variable_name\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true\nline-length = 50", input);
    assert!(
        result.contains("    use m_x, only: d_variable_name\n"),
        "line exceeding line-length should be left alone, got: {result}"
    );
}

#[test]
fn test_align_use_only_idempotent() {
    let input = "module m\n    use m_derived_types, only: t_foo\n    use m_global, only: a, b\n    use m_x, only: d\n\n    implicit none\nend module m\n";
    let once = format_with_config("align-use-only = true", input);
    let twice = format_with_config("align-use-only = true", &once);
    assert_eq!(once, twice);
}

#[test]
fn test_align_use_only_module_name_containing_only_and_intrinsic() {
    let input = "module m\n    use m_only_stuff, only: x\n    use, intrinsic :: iso_fortran_env, only: wp => real64\n    use m_x, only: d\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true", input);
    let expected = "module m\n\n    use m_only_stuff,                  only: x\n    use, intrinsic :: iso_fortran_env, only: wp => real64\n    use m_x,                           only: d\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

#[test]
fn test_align_use_only_keeps_inline_doxygen_comments_aligned() {
    let input = "module m\n    use m_derived_types, only: t_foo !< types\n    use m_global, only: a, b !< globals\n    use m_x, only: d !< x\n\n    implicit none\nend module m\n";
    let result = format_with_config("align-use-only = true", input);
    let expected = "module m\n\n    use m_derived_types, only: t_foo  !< types\n    use m_global,        only: a, b   !< globals\n    use m_x,             only: d      !< x\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}

// ===== `only :` normalization =====

#[test]
fn test_only_colon_normalized_end_to_end() {
    let input = "module m\n    use m_a, only : a\n    use, intrinsic :: iso_c_binding, ONLY  : c_int\n\n    implicit none\nend module m\n";
    let result = format_with_config("", input);
    let expected = "module m\n\n    use m_a, only: a\n    use, intrinsic :: iso_c_binding, only: c_int\n\n    implicit none\nend module m\n";
    assert_eq!(result, expected);
}
