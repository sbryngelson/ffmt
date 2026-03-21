use ffmt::case_norm::normalize_case;

#[test]
fn test_keywords_lowered() {
    assert_eq!(normalize_case("IF (x > 0) THEN"), "if (x > 0) then");
}
#[test]
fn test_end_do() {
    assert_eq!(normalize_case("END DO"), "end do");
}
#[test]
fn test_type_keyword() {
    assert_eq!(normalize_case("INTEGER :: x"), "integer :: x");
}
#[test]
fn test_logical_literal() {
    assert_eq!(normalize_case("x = .TRUE."), "x = .true.");
}
#[test]
fn test_identifiers_preserved() {
    assert_eq!(normalize_case("call MySubroutine(X_val)"), "call MySubroutine(X_val)");
}
#[test]
fn test_string_preserved() {
    assert_eq!(normalize_case("x = 'HELLO WORLD'"), "x = 'HELLO WORLD'");
}
#[test]
fn test_fypp_preserved() {
    assert_eq!(normalize_case("$:GPU_PARALLEL_LOOP(collapse=3)"), "$:GPU_PARALLEL_LOOP(collapse=3)");
}
#[test]
fn test_comment_preserved() {
    assert_eq!(normalize_case("x = 1 ! IMPORTANT NOTE"), "x = 1 ! IMPORTANT NOTE");
}
#[test]
fn test_mixed() {
    assert_eq!(
        normalize_case("SUBROUTINE s_compute_rhs(q_vf, rhs_vf)"),
        "subroutine s_compute_rhs(q_vf, rhs_vf)"
    );
}
#[test]
fn test_fypp_inline_expression_preserved() {
    assert_eq!(
        normalize_case("if (recon_type == ${WENO_TYPE}$) then"),
        "if (recon_type == ${WENO_TYPE}$) then"
    );
}
#[test]
fn test_at_inline_expression_preserved() {
    assert_eq!(normalize_case("@{GPU_MACRO}@"), "@{GPU_MACRO}@");
}
#[test]
fn test_dot_operator_lowered() {
    assert_eq!(normalize_case("IF (a .AND. b .OR. c) THEN"), "if (a .and. b .or. c) then");
}
#[test]
fn test_logical_not() {
    assert_eq!(normalize_case(".NOT. flag"), ".not. flag");
}
