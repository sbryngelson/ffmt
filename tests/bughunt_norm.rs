//! Regression tests for bug-hunt findings in case normalization and
//! whitespace normalization (BUG_HUNT.md findings 19/26, 41, 49, 50, 51).

use ffmt::config::{Config, Toggle, WhitespaceConfig};

fn idempotent(src: &str) {
    let once = ffmt::format_string(src);
    let twice = ffmt::format_string(&once);
    assert_eq!(once, twice, "formatting is not idempotent for:\n{src}");
}

// --- Finding 19/26 [HIGH]: non-ASCII UTF-8 in code regions mojibaked ---

#[test]
fn test_utf8_identifier_not_mojibaked() {
    let out = ffmt::format_string("z = \u{3c0}_value\n");
    assert!(
        out.contains("\u{3c0}_value"),
        "UTF-8 identifier corrupted: {out:?}"
    );
    assert!(!out.contains("\u{c3}"), "mojibake detected: {out:?}");
}

#[test]
fn test_utf8_greek_letters_pass_through() {
    // Bytes ce b1 ce b2 must stay ce b1 ce b2, not become c3 8e c2 b1 ...
    let out = ffmt::format_string("x = \u{3b1}\u{3b2} + 1\n");
    assert!(
        out.contains("\u{3b1}\u{3b2}"),
        "Greek letters corrupted: {out:?}"
    );
}

#[test]
fn test_utf8_code_text_idempotent() {
    // Mojibake compounds on every pass; a correct formatter is idempotent.
    idempotent("z = \u{3c0}_value\n");
    idempotent("x = \u{3b1}\u{3b2} + 1\n");
    idempotent("y = \u{ff} + 1\n");
}

#[test]
fn test_utf8_adjacent_to_keyword_preserved() {
    // Keyword lowercasing must still work next to non-ASCII text.
    let out = ffmt::format_string("IF (\u{3b1} > 0) x = 1\n");
    assert!(
        out.contains("if (\u{3b1} > 0)"),
        "keyword/UTF-8 mix corrupted: {out:?}"
    );
}

// --- Finding 41 [MEDIUM]: same pattern through the whitespace pass ---

#[test]
fn test_utf8_preserved_through_whitespace_normalization() {
    // Operator spacing must be applied without disturbing the UTF-8 text.
    let out = ffmt::format_string("x=\u{3b1}\u{3b2}+1\n");
    assert!(
        out.contains("x = \u{3b1}\u{3b2} + 1"),
        "whitespace pass corrupted UTF-8: {out:?}"
    );
}

// --- Finding 49 [LOW]: '::' inside parens is a stride slice, not a decl ---

#[test]
fn test_stride_slice_double_colon_stays_compact() {
    let out = ffmt::format_string("b = a(1::2)\nc = a(::2)\nd = a(1:10:2)\n");
    assert!(out.contains("b = a(1::2)"), "stride slice spaced: {out:?}");
    assert!(out.contains("c = a(::2)"), "stride slice spaced: {out:?}");
    assert!(
        out.contains("d = a(1:10:2)"),
        "plain slice changed: {out:?}"
    );
}

#[test]
fn test_declaration_double_colon_still_spaced() {
    let out = ffmt::format_string("integer::i\n");
    assert!(
        out.contains("integer :: i"),
        "declaration :: lost its spacing: {out:?}"
    );
}

#[test]
fn test_declaration_with_paren_attr_double_colon_spaced() {
    // The '::' here is at paren depth 0 (after the closing paren).
    let out = ffmt::format_string("character(len=8)::s\n");
    assert!(
        out.contains("character(len=8) :: s"),
        "declaration :: after paren attr lost spacing: {out:?}"
    );
}

// --- Finding 50 [LOW]: namelist guard needs a word boundary ---

#[test]
fn test_namelist_prefixed_identifier_is_normalized() {
    let out = ffmt::format_string("namelist_foo=1+2\n");
    assert!(
        out.contains("namelist_foo = 1 + 2"),
        "identifier starting with 'namelist' skipped normalization: {out:?}"
    );
}

#[test]
fn test_namelist_statement_still_skipped() {
    // Real namelist statements must remain untouched (the / are not division).
    let src = "namelist /grp/ a, b\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("namelist /grp/ a, b"),
        "namelist statement was modified: {out:?}"
    );
}

// --- Finding 51 [LOW]: is_io_format_star misfires on read/write/print vars ---

fn multdiv_config() -> Config {
    Config {
        whitespace: WhitespaceConfig {
            multdiv: Toggle::Enable,
            ..WhitespaceConfig::default()
        },
        ..Config::default()
    }
}

#[test]
fn test_variable_named_read_gets_multdiv_spacing() {
    let config = multdiv_config();
    let out = ffmt::format_string_with_config("z = read*2\nz = y*2\n", &config);
    assert!(
        out.contains("z = read * 2"),
        "variable 'read' skipped multdiv spacing: {out:?}"
    );
    assert!(out.contains("z = y * 2"), "control case failed: {out:?}");
}

#[test]
fn test_variable_named_read_collapses_with_default_config() {
    // Default config: multdiv compact — 'read * 2' must collapse like 'y * 2'.
    let out = ffmt::format_string("z = y * 2\nz = read * 2\n");
    assert!(out.contains("z = y*2"), "control case failed: {out:?}");
    assert!(
        out.contains("z = read*2"),
        "variable 'read' star kept spaces: {out:?}"
    );
}

#[test]
fn test_io_format_star_still_recognized() {
    // Statement-position I/O stars must keep their format-star treatment.
    let out = ffmt::format_string("print *, x\nread *, y\nwrite (*, *) z\n");
    assert!(out.contains("print *, x"), "print star mangled: {out:?}");
    assert!(out.contains("read *, y"), "read star mangled: {out:?}");
    assert!(
        out.contains("write (*, *) z"),
        "write star mangled: {out:?}"
    );
}

#[test]
fn test_io_format_star_after_if_condition() {
    let out = ffmt::format_string("if (x > 0) print *, x\n");
    assert!(
        out.contains("print *, x"),
        "format star after if-condition mangled: {out:?}"
    );
}

#[test]
fn test_io_format_star_with_multdiv_enabled() {
    let config = multdiv_config();
    let out = ffmt::format_string_with_config("print *, x\nwrite (*, *) z\n", &config);
    assert!(
        out.contains("print *, x"),
        "format star treated as multiply: {out:?}"
    );
    assert!(
        out.contains("write (*, *) z"),
        "write control-list star treated as multiply: {out:?}"
    );
}
