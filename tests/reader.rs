use ffmt::reader::read_logical_lines;

#[test]
fn test_simple_lines() {
    let input = "integer :: x\nreal :: y\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].joined.trim(), "integer :: x");
    assert_eq!(lines[1].joined.trim(), "real :: y");
}

#[test]
fn test_continuation_join() {
    let input = "call foo(a, &\n         b, &\n         c)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].joined.contains("call foo(a,"));
    assert!(lines[0].joined.contains("c)"));
    assert_eq!(lines[0].raw_lines.len(), 3);
}

#[test]
fn test_blank_lines_preserved() {
    let input = "integer :: x\n\n\nreal :: y\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 4); // x, blank, blank, y
}

#[test]
fn test_fypp_continuation_not_joined() {
    // !& is a Fypp-level continuation — the formatter treats each line independently
    let input = "#:if defined('FOO') !&\n    & .or. defined('BAR')\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_opaque_regions_strings() {
    let input = "x = 'hello & world'\n";
    let lines = read_logical_lines(input);
    // The & inside a string is NOT a continuation
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].raw_lines.len(), 1);
}

#[test]
fn test_comment_line() {
    let input = "! This is a comment\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].raw_lines.len(), 1);
}

#[test]
fn test_directive_not_comment() {
    let input = "!$acc parallel loop\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    // It's a single logical line, not split or treated as blank
}

#[test]
fn test_line_numbers() {
    let input = "a\nb\nc\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines[0].line_number, 1);
    assert_eq!(lines[1].line_number, 2);
    assert_eq!(lines[2].line_number, 3);
}

#[test]
fn test_continuation_strips_leading_ampersand() {
    let input = "x = a + &\n    & b\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    // The joined line should have the content without the leading &
    assert!(lines[0].joined.contains("b"));
    assert!(
        !lines[0].joined.contains("& b")
            || lines[0].joined.matches('&').count() == 0
            || lines[0].joined.contains("a +  b")
            || lines[0].joined.contains("a + b")
    );
}

#[test]
fn test_blank_line_inside_continuation_joined() {
    // A blank line between a trailing-& line and a leading-& continuation is a
    // comment line per the free-form standard (F2018 6.3.2.3) — the statement
    // resumes on the next non-comment line. The reader must join across it.
    let input = "subroutine foo(a, b, &\n\n    & c, d, &\n        & e)\n";
    let lines = read_logical_lines(input);
    assert_eq!(
        lines.len(),
        1,
        "blank inside continuation split the logical line"
    );
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "subroutine foo(a, b, c, d, e)");
}

#[test]
fn test_blank_then_comment_inside_continuation_joined_and_hoisted() {
    // Comments are never left in the middle of a continuation: the statement
    // is joined and the comment is hoisted so the formatter can emit it
    // above the statement.
    let input = "call foo(a, &\n\n    ! note\n    & b)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "call foo(a, b)");
    assert_eq!(lines[0].hoisted_comments, vec!["! note"]);
}

#[test]
fn test_comment_then_blank_inside_continuation_hoisted() {
    let input = "call foo(a, &\n    ! note\n\n    & b)\n";
    let lines = read_logical_lines(input);
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "call foo(a, b)");
    assert_eq!(lines[0].hoisted_comments, vec!["! note"]);
}

#[test]
fn test_trailing_amp_comment_hoisted() {
    // `& ! why` — comment after the continuation ampersand must be captured,
    // not dropped.
    let input = "call foo(a, & ! why a\n    & b, & ! why b\n    & c)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "call foo(a, b, c)");
    assert_eq!(lines[0].hoisted_comments, vec!["! why a", "! why b"]);
}

#[test]
fn test_blank_then_no_amp_resume_joins() {
    // Per the standard (and gfortran): blank lines are comment lines; the
    // statement resumes at the next non-comment line with or without a
    // leading `&`.
    let input = "x = a + &\n\n    b\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "x = a + b");
}

// --- Bug hunt: F2018 6.3.2.4 splice semantics ---

#[test]
fn test_split_string_literal_glued() {
    // Trailing & + leading & splice with NO inserted space: split string
    // literals must reassemble exactly.
    let input = "print *, 'abc&\n&def'\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].joined.contains("'abcdef'"),
        "string splice corrupted: {}",
        lines[0].joined
    );
}

#[test]
fn test_split_string_literal_interior_spaces_kept() {
    // Blanks before the trailing & inside a character literal are content.
    let input = "print *, 'a  &\n&b'\n";
    let lines = read_logical_lines(input);
    assert!(
        lines[0].joined.contains("'a  b'"),
        "interior spaces lost: {}",
        lines[0].joined
    );
}

#[test]
fn test_split_identifier_glued() {
    let input = "x = verylong&\n&name + 1\n";
    let lines = read_logical_lines(input);
    assert!(
        lines[0].joined.contains("verylongname"),
        "split identifier not glued: {}",
        lines[0].joined
    );
}

#[test]
fn test_separate_tokens_not_merged() {
    // `integer &` has a blank before the &, so the tokens stay separate.
    let input = "integer &\n& x\n";
    let lines = read_logical_lines(input);
    let j = &lines[0].joined;
    assert!(
        j.contains("integer x") || j.contains("integer  x"),
        "tokens wrongly merged: {j}"
    );
    assert!(!j.contains("integerx"), "tokens wrongly merged: {j}");
}

#[test]
fn test_amp_comment_inside_continued_string_not_converted() {
    // The `& !` on the second line is INSIDE a continued character literal —
    // it must not be converted to a comment line.
    let input = "x = 'one &\n& ! two'\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].joined.contains("! two'"),
        "string contents destroyed: {}",
        lines[0].joined
    );
}

#[test]
fn test_plain_comment_then_no_amp_resume_joins() {
    // Valid per the standard (and gfortran): a plain comment line inside a
    // continuation, statement resumes WITHOUT a leading &.
    let input = "x = a + &\n    ! note\n    b\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1, "statement was fragmented");
    let j = lines[0]
        .joined
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(j, "x = a + b");
    assert_eq!(lines[0].hoisted_comments, vec!["! note"]);
}

#[test]
fn test_amp_comment_line_terminates_statement() {
    // `& ! comment` is a continuation line with empty content that does not
    // itself end with & — per gfortran the statement TERMINATES. The dangling
    // & on the previous line is repaired (stripped).
    let input = "case (101) &\n    & ! dangling\nx = 2\n";
    let lines = read_logical_lines(input);
    assert!(
        lines.len() >= 3,
        "statement wrongly joined across & ! comment"
    );
    assert_eq!(lines[0].joined.trim(), "case (101)");
}

#[test]
fn test_bang_amp_inside_comment_text_kept() {
    let input = "! see foo!&\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines[0].joined, "! see foo!&");
}

#[test]
fn test_directive_line_mid_continuation_preserved() {
    // A !$omp sentinel inside a continuation must not be absorbed as text;
    // the logical line is marked preserve and emitted verbatim.
    let input = "x = a + &\n!$omp parallel\n& b\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].preserve,
        "directive-bearing continuation not preserved"
    );
    assert_eq!(lines[0].raw_lines.len(), 3);
}

#[test]
fn test_directive_trailer_after_amp_preserved() {
    let input = "x = a + & !$acc some clause\n& b\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].preserve, "directive trailer not preserved");
}
