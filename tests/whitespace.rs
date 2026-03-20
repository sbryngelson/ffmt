use ffmt::config::WhitespaceConfig;
use ffmt::whitespace::normalize_whitespace as normalize_whitespace_with_config;

fn normalize_whitespace(line: &str) -> String {
    normalize_whitespace_with_config(line, &WhitespaceConfig::default())
}

// --- Relational operators ---
#[test]
fn test_relational_eq() {
    assert_eq!(normalize_whitespace("if (x==y) then"), "if (x == y) then");
}
#[test]
fn test_relational_ne() {
    assert_eq!(normalize_whitespace("if (x/=y) then"), "if (x /= y) then");
}
#[test]
fn test_relational_le() {
    assert_eq!(normalize_whitespace("if (x<=y) then"), "if (x <= y) then");
}
// --- Logical operators ---
#[test]
fn test_logical_and() {
    assert_eq!(
        normalize_whitespace("if (a .AND. b) then"),
        "if (a .AND. b) then"
    );
}
// --- Assignment ---
#[test]
fn test_assignment_spaces() {
    assert_eq!(normalize_whitespace("x=y + z"), "x = y + z");
}
#[test]
fn test_keyword_arg_no_spaces() {
    assert_eq!(
        normalize_whitespace("call foo(bar = 1)"),
        "call foo(bar=1)"
    );
}
// --- Pointer/rename ---
#[test]
fn test_pointer_assignment() {
    assert_eq!(normalize_whitespace("ptr=>target"), "ptr => target");
}
// --- Multiply/divide no spaces ---
#[test]
fn test_multiply_no_space() {
    assert_eq!(normalize_whitespace("x = a * b"), "x = a*b");
}
#[test]
fn test_divide_no_space() {
    assert_eq!(normalize_whitespace("x = a / b"), "x = a/b");
}
#[test]
fn test_exponent_no_space() {
    assert_eq!(normalize_whitespace("x = a ** 2"), "x = a**2");
}
// --- String concat ---
#[test]
fn test_concat_spaces() {
    assert_eq!(normalize_whitespace("s = a//b"), "s = a // b");
}
// --- Commas ---
#[test]
fn test_comma_spacing() {
    assert_eq!(
        normalize_whitespace("call f(a,b ,c)"),
        "call f(a, b, c)"
    );
}
// --- Colons ---
#[test]
fn test_array_slice_no_spaces() {
    assert_eq!(normalize_whitespace("a(1 : n)"), "a(1:n)");
}
#[test]
fn test_declaration_double_colon() {
    assert_eq!(normalize_whitespace("integer::x"), "integer :: x");
}
// --- Parentheses ---
#[test]
fn test_no_internal_padding() {
    assert_eq!(normalize_whitespace("call f( x, y )"), "call f(x, y)");
}
// --- Trailing whitespace ---
#[test]
fn test_trailing_whitespace() {
    assert_eq!(normalize_whitespace("x = 1   "), "x = 1");
}
// --- Strings preserved ---
#[test]
fn test_string_not_modified() {
    assert_eq!(
        normalize_whitespace("x = 'hello==world'"),
        "x = 'hello==world'"
    );
}
// --- Unary minus ---
#[test]
fn test_unary_minus() {
    assert_eq!(normalize_whitespace("x = -y"), "x = -y");
}
#[test]
fn test_exponent_notation() {
    assert_eq!(normalize_whitespace("x = 1.0e+3"), "x = 1.0e+3");
}
// --- Inline comment spacing ---
#[test]
fn test_inline_comment() {
    assert_eq!(normalize_whitespace("x = 1!comment"), "x = 1 !comment");
}
// --- Unary minus/plus after various operators ---
#[test]
fn test_unary_minus_after_exponent() {
    assert_eq!(normalize_whitespace("x = a**-2"), "x = a**-2");
}
#[test]
fn test_unary_minus_after_divide() {
    assert_eq!(normalize_whitespace("x = a/-b"), "x = a/-b");
}
#[test]
fn test_unary_minus_after_comma() {
    assert_eq!(
        normalize_whitespace("call f(a, -b)"),
        "call f(a, -b)"
    );
}
#[test]
fn test_unary_minus_after_paren() {
    assert_eq!(normalize_whitespace("x = (-y)"), "x = (-y)");
}
#[test]
fn test_unary_minus_after_double_colon() {
    assert_eq!(
        normalize_whitespace("integer :: x = -1"),
        "integer :: x = -1"
    );
}
// --- Fypp inline expression protection ---
#[test]
fn test_fypp_inline_not_modified() {
    assert_eq!(
        normalize_whitespace("if (recon_type == ${TYPE}$) then"),
        "if (recon_type == ${TYPE}$) then"
    );
}
#[test]
fn test_fypp_at_inline_not_modified() {
    assert_eq!(
        normalize_whitespace("x = @{MACRO}@ + 1"),
        "x = @{MACRO}@ + 1"
    );
}
// --- Edge cases ---
#[test]
fn test_semicolon_preserved() {
    assert_eq!(
        normalize_whitespace("private; public :: s_foo"),
        "private; public :: s_foo"
    );
}
#[test]
fn test_binary_plus() {
    assert_eq!(normalize_whitespace("x = a+b"), "x = a + b");
}
#[test]
fn test_binary_minus() {
    assert_eq!(normalize_whitespace("x = a-b"), "x = a - b");
}
#[test]
fn test_doxygen_comment_spacing_preserved() {
    // Doxygen !< comment — spacing before ! should be preserved
    assert_eq!(
        normalize_whitespace("real(wp), parameter :: x = 1.0_wp                !< Default value"),
        "real(wp), parameter :: x = 1.0_wp                !< Default value"
    );
}
#[test]
fn test_doxygen_double_bang_preserved() {
    assert_eq!(
        normalize_whitespace("use m_types        !! Type definitions"),
        "use m_types        !! Type definitions"
    );
}
#[test]
fn test_regular_comment_one_space() {
    // Regular ! comment (not Doxygen) still gets single space
    assert_eq!(
        normalize_whitespace("x = 1      ! regular comment"),
        "x = 1 ! regular comment"
    );
}
#[test]
fn test_scientific_notation_dot_e() {
    // 1.e-16_wp must NOT have spaces around the minus
    assert_eq!(normalize_whitespace("x = 1.e-16_wp"), "x = 1.e-16_wp");
}
#[test]
fn test_scientific_notation_dot_e_plus() {
    assert_eq!(normalize_whitespace("x = 1.e+6_wp"), "x = 1.e+6_wp");
}
