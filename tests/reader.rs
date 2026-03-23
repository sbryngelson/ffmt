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
            || lines[0].joined.matches('&').count() <= 0
            || lines[0].joined.contains("a +  b")
            || lines[0].joined.contains("a + b")
    );
}
