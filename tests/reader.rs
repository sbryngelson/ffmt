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
    assert_eq!(lines[0].joined, "subroutine foo(a, b, c, d, e)");
}

#[test]
fn test_blank_then_comment_inside_continuation_joined_and_hoisted() {
    // Comments are never left in the middle of a continuation: the statement
    // is joined and the comment is hoisted so the formatter can emit it
    // above the statement.
    let input = "call foo(a, &\n\n    ! note\n    & b)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].joined, "call foo(a, b)");
    assert_eq!(lines[0].hoisted_comments, vec!["! note"]);
}

#[test]
fn test_comment_then_blank_inside_continuation_hoisted() {
    let input = "call foo(a, &\n    ! note\n\n    & b)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines[0].joined, "call foo(a, b)");
    assert_eq!(lines[0].hoisted_comments, vec!["! note"]);
}

#[test]
fn test_trailing_amp_comment_hoisted() {
    // `& ! why` — comment after the continuation ampersand must be captured,
    // not dropped.
    let input = "call foo(a, & ! why a\n    & b, & ! why b\n    & c)\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].joined, "call foo(a, b, c)");
    assert_eq!(lines[0].hoisted_comments, vec!["! why a", "! why b"]);
}

#[test]
fn test_blank_after_amp_without_leading_amp_not_joined() {
    // Conservative: if the next code line does not begin with `&`, keep the
    // old behavior (stop the continuation at the blank).
    let input = "x = a + &\n\nend subroutine\n";
    let lines = read_logical_lines(input);
    assert_eq!(lines.len(), 3);
}
