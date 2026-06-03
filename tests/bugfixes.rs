//! Regression tests for bugs found in code review (2026-06).

use ffmt::classifier::{classify, LineKind};
use ffmt::config::{Config, EndOfLine, Toggle};

// --- Doubled-quote escape scanning (formatter string scanners) ---

#[test]
fn test_inline_comment_spacing_after_escaped_quote_string() {
    // `''` is an escaped quote: the string ends at the final `'`, so the `!`
    // is an inline comment and must get two spaces before it.
    let out = ffmt::format_string("program t\n    x = 'it''s' ! note\nend program t\n");
    assert!(
        out.contains("x = 'it''s'  ! note"),
        "expected two spaces before inline comment, got:\n{out}"
    );
}

#[test]
fn test_no_space_inserted_inside_string_with_escaped_quote() {
    // The `!&` here is INSIDE the string literal; nothing may be inserted.
    let src = "program t\n    x = 'hello'' !& world'\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("x = 'hello'' !& world'"),
        "string literal content was modified:\n{out}"
    );
}

#[test]
fn test_split_statements_ignores_semicolon_inside_escaped_quote_string() {
    let config = Config {
        split_statements: Toggle::Enable,
        ..Config::default()
    };
    // The `;` is inside the string (after a `''` escape) — must not split there.
    let src = "program t\n    x = 'a''; b'\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("x = 'a''; b'"),
        "statement was split inside a string literal:\n{out}"
    );
}

#[test]
fn test_semicolon_strip_with_bang_inside_escaped_quote_string() {
    // The `!` is inside the string; the trailing `;` after the string must be stripped.
    let src = "program t\n    x = 'it''s ! str';\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("x = 'it''s ! str'\n"),
        "trailing semicolon not stripped (comment misdetected inside string):\n{out}"
    );
}

// --- normalize_intent_paren must not touch strings or comments ---

#[test]
fn test_intent_paren_not_normalized_inside_string() {
    let line = "msg = 'intent (in) is enforced'";
    assert_eq!(ffmt::whitespace::normalize_intent_paren(line), line);
}

#[test]
fn test_intent_paren_not_normalized_inside_comment() {
    let line = "x = 1  ! intent (in) matters";
    assert_eq!(ffmt::whitespace::normalize_intent_paren(line), line);
}

#[test]
fn test_intent_paren_still_normalized_in_code() {
    assert_eq!(
        ffmt::whitespace::normalize_intent_paren("integer, intent (in) :: n"),
        "integer, intent(in) :: n"
    );
}

// --- add_keyword_paren_spaces must not touch comments ---

#[test]
fn test_keyword_paren_space_not_added_inside_comment() {
    let line = "x = 1  ! if(cond) would apply";
    assert_eq!(ffmt::whitespace::add_keyword_paren_spaces(line), line);
}

// --- UTF-8 in fypp lists ---

#[test]
fn test_fypp_list_preserves_utf8() {
    let config = Config {
        fypp_list_commas: Toggle::Enable,
        ..Config::default()
    };
    let src = "program t\n    $: foo('[α,β]')\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("'[α, β]'"),
        "UTF-8 corrupted in fypp list:\n{out}"
    );
}

#[test]
fn test_modernize_operators_preserves_utf8() {
    let config = Config {
        modernize_operators: Toggle::Enable,
        ..Config::default()
    };
    let src = "program t\n    if (s .eq. 'α') x = 1  ! β note\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("== 'α'") && out.contains("! β note"),
        "UTF-8 corrupted while modernizing operators:\n{out}"
    );
}

// --- Classifier: trailing comments must not change classification ---

#[test]
fn test_classify_contains_with_comment() {
    assert_eq!(classify("contains ! public api"), LineKind::FortranContains);
}

#[test]
fn test_classify_where_block_with_comment() {
    assert_eq!(
        classify("where (mask > 0) ! apply only where positive"),
        LineKind::FortranBlockOpen
    );
}

#[test]
fn test_classify_where_one_liner_with_comment_is_statement() {
    assert_eq!(
        classify("where (mask > 0) a = b ! one-liner"),
        LineKind::FortranStatement
    );
}

#[test]
fn test_classify_forall_block_with_comment() {
    assert_eq!(
        classify("forall (i=1:n) ! vectorised"),
        LineKind::FortranBlockOpen
    );
}

#[test]
fn test_classify_block_with_comment() {
    assert_eq!(classify("block ! local scope"), LineKind::FortranBlockOpen);
}

#[test]
fn test_classify_class_default_with_comment() {
    assert_eq!(
        classify("class default ! fallback"),
        LineKind::FortranContinuation
    );
}

// --- Classifier: parens inside strings ---

#[test]
fn test_classify_if_then_with_paren_in_string() {
    assert_eq!(
        classify("if (ch == '(') then"),
        LineKind::FortranBlockOpen
    );
}

#[test]
fn test_classify_if_then_with_close_paren_in_string() {
    assert_eq!(
        classify("if (ch == ')') then"),
        LineKind::FortranBlockOpen
    );
}

// --- end-of-line = "preserve" ---

#[test]
fn test_preserve_keeps_crlf() {
    let config = Config {
        end_of_line: EndOfLine::Preserve,
        ..Config::default()
    };
    let src = "program t\r\n    x = 1\r\nend program t\r\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("\r\n"),
        "CRLF endings were not preserved:\n{out:?}"
    );
    assert!(
        !out.replace("\r\n", "").contains('\r') && !out.replace("\r\n", "").contains('\n'),
        "mixed line endings in output:\n{out:?}"
    );
}

#[test]
fn test_preserve_keeps_lf() {
    let config = Config {
        end_of_line: EndOfLine::Preserve,
        ..Config::default()
    };
    let src = "program t\n    x = 1\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(!out.contains('\r'), "LF source gained CR:\n{out:?}");
}

// --- Exponent-sign heuristic ---

#[test]
fn test_identifier_ending_in_digit_e_plus_is_binary_op() {
    let out = ffmt::format_string("program t\n    x = x2e+y\nend program t\n");
    assert!(
        out.contains("x = x2e + y"),
        "binary + after identifier `x2e` not spaced:\n{out}"
    );
}

#[test]
fn test_real_exponent_signs_untouched() {
    let out = ffmt::format_string("program t\n    x = 1.0e+3 - 2.5d-2 + .5e-1\nend program t\n");
    assert!(
        out.contains("x = 1.0e+3 - 2.5d-2 + .5e-1"),
        "exponent literals were modified:\n{out}"
    );
}
