use ffmt::classifier::{classify, LineKind};

// --- Fortran block openers ---
#[test]
fn test_if_then() {
    assert_eq!(classify("if (x > 0) then"), LineKind::FortranBlockOpen);
}
#[test]
fn test_single_line_if() {
    assert_eq!(
        classify("if (x > 0) call foo()"),
        LineKind::FortranStatement
    );
}
#[test]
fn test_do_loop() {
    assert_eq!(classify("do i = 1, n"), LineKind::FortranBlockOpen);
}
#[test]
fn test_do_concurrent() {
    assert_eq!(classify("do concurrent(i=1:n)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_subroutine() {
    assert_eq!(
        classify("subroutine s_foo(x, y)"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_pure_function() {
    assert_eq!(
        classify("pure function f_bar(x) result(res)"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_module() {
    assert_eq!(classify("module m_rhs"), LineKind::FortranBlockOpen);
}
#[test]
fn test_type_definition() {
    assert_eq!(classify("type :: my_type"), LineKind::FortranBlockOpen);
}
#[test]
fn test_type_usage_not_opener() {
    assert_eq!(classify("type(my_type) :: x"), LineKind::FortranStatement);
}
#[test]
fn test_select_case() {
    assert_eq!(classify("select case (idir)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_interface() {
    assert_eq!(classify("interface"), LineKind::FortranBlockOpen);
}
// --- Fortran block closers ---
#[test]
fn test_end_subroutine() {
    assert_eq!(
        classify("end subroutine s_foo"),
        LineKind::FortranBlockClose
    );
}
#[test]
fn test_end_do() {
    assert_eq!(classify("end do"), LineKind::FortranBlockClose);
}
#[test]
fn test_bare_end() {
    assert_eq!(classify("end"), LineKind::FortranBlockClose);
}
// --- Fortran continuations ---
#[test]
fn test_else() {
    assert_eq!(classify("else"), LineKind::FortranContinuation);
}
#[test]
fn test_else_if() {
    assert_eq!(
        classify("else if (y > 0) then"),
        LineKind::FortranContinuation
    );
}
#[test]
fn test_case() {
    assert_eq!(classify("case (1)"), LineKind::FortranContinuation);
}
#[test]
fn test_contains() {
    assert_eq!(classify("contains"), LineKind::FortranContains);
}
// --- where block vs statement ---
#[test]
fn test_where_block() {
    assert_eq!(classify("where (mask > 0)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_where_statement() {
    assert_eq!(
        classify("where (mask > 0) array = 1.0"),
        LineKind::FortranStatement
    );
}
#[test]
fn test_elsewhere() {
    assert_eq!(classify("elsewhere"), LineKind::FortranContinuation);
}
#[test]
fn test_end_where() {
    assert_eq!(classify("end where"), LineKind::FortranBlockClose);
}
// --- select type/rank ---
#[test]
fn test_select_type() {
    assert_eq!(classify("select type (obj)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_select_rank() {
    assert_eq!(classify("select rank (arr)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_type_is() {
    assert_eq!(classify("type is (integer)"), LineKind::FortranContinuation);
}
#[test]
fn test_class_is() {
    assert_eq!(
        classify("class is (my_type)"),
        LineKind::FortranContinuation
    );
}
#[test]
fn test_class_default() {
    assert_eq!(classify("class default"), LineKind::FortranContinuation);
}
#[test]
fn test_rank_continuation() {
    assert_eq!(classify("rank (1)"), LineKind::FortranContinuation);
}
// --- associate/block/critical/enum ---
#[test]
fn test_associate() {
    assert_eq!(
        classify("associate (x => obj%field)"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_end_associate() {
    assert_eq!(classify("end associate"), LineKind::FortranBlockClose);
}
#[test]
fn test_block() {
    assert_eq!(classify("block"), LineKind::FortranBlockOpen);
}
#[test]
fn test_end_block() {
    assert_eq!(classify("end block"), LineKind::FortranBlockClose);
}
#[test]
fn test_critical() {
    assert_eq!(classify("critical"), LineKind::FortranBlockOpen);
}
#[test]
fn test_end_critical() {
    assert_eq!(classify("end critical"), LineKind::FortranBlockClose);
}
#[test]
fn test_enum() {
    assert_eq!(classify("enum, bind(c)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_end_enum() {
    assert_eq!(classify("end enum"), LineKind::FortranBlockClose);
}
// --- Fypp ---
#[test]
fn test_fypp_if() {
    assert_eq!(
        classify("#:if defined('MFC_OpenACC')"),
        LineKind::FyppBlockOpen
    );
}
#[test]
fn test_fypp_for() {
    assert_eq!(classify("#:for VAR in VARS"), LineKind::FyppBlockOpen);
}
#[test]
fn test_fypp_endif() {
    assert_eq!(classify("#:endif"), LineKind::FyppBlockClose);
}
#[test]
fn test_fypp_else() {
    assert_eq!(classify("#:else"), LineKind::FyppContinuation);
}
#[test]
fn test_fypp_elif() {
    assert_eq!(
        classify("#:elif defined('BAR')"),
        LineKind::FyppContinuation
    );
}
#[test]
fn test_fypp_include() {
    assert_eq!(classify("#:include 'macros.fpp'"), LineKind::FyppStatement);
}
#[test]
fn test_fypp_set() {
    assert_eq!(classify("#:set FOO = 1"), LineKind::FyppStatement);
}
#[test]
fn test_fypp_comment() {
    assert_eq!(
        classify("#! This is a Fypp comment"),
        LineKind::FyppStatement
    );
}
#[test]
fn test_fypp_call() {
    assert_eq!(
        classify("#:call GPU_PARALLEL(collapse=3)"),
        LineKind::FyppBlockOpen
    );
}
#[test]
fn test_fypp_endcall() {
    assert_eq!(classify("#:endcall GPU_PARALLEL"), LineKind::FyppBlockClose);
}
#[test]
fn test_fypp_def() {
    assert_eq!(
        classify("#:def GPU_PARALLEL(collapse)"),
        LineKind::FyppBlockOpen
    );
}
#[test]
fn test_fypp_enddef() {
    assert_eq!(classify("#:enddef GPU_PARALLEL"), LineKind::FyppBlockClose);
}
#[test]
fn test_fypp_block() {
    assert_eq!(classify("#:block DEBUG_BLOCK"), LineKind::FyppBlockOpen);
}
#[test]
fn test_fypp_endblock() {
    assert_eq!(classify("#:endblock DEBUG_BLOCK"), LineKind::FyppBlockClose);
}
#[test]
fn test_fypp_mute() {
    assert_eq!(classify("#:mute"), LineKind::FyppBlockOpen);
}
#[test]
fn test_fypp_endmute() {
    assert_eq!(classify("#:endmute"), LineKind::FyppBlockClose);
}
// --- Preprocessor ---
#[test]
fn test_ifdef() {
    assert_eq!(
        classify("#ifdef MFC_OpenACC"),
        LineKind::PreprocessorDirective
    );
}
#[test]
fn test_ifndef() {
    assert_eq!(classify("#ifndef MFC_MPI"), LineKind::PreprocessorDirective);
}
#[test]
fn test_cpp_else() {
    assert_eq!(classify("#else"), LineKind::PreprocessorContinuation);
}
#[test]
fn test_endif_cpp() {
    assert_eq!(classify("#endif"), LineKind::PreprocessorClose);
}
// --- Directives ---
#[test]
fn test_acc_directive() {
    assert_eq!(classify("!$acc parallel loop"), LineKind::Directive);
}
#[test]
fn test_omp_directive() {
    assert_eq!(classify("!$omp parallel do"), LineKind::Directive);
}
// --- Inline Fypp ---
#[test]
fn test_inline_fypp_dollar() {
    assert_eq!(
        classify("$:GPU_PARALLEL_LOOP(collapse=3)"),
        LineKind::InlineFypp
    );
}
#[test]
fn test_inline_fypp_at() {
    assert_eq!(
        classify("@:ALLOCATE(q_cons_qp%vf(1:sys_size))"),
        LineKind::InlineFypp
    );
}
// --- Other ---
#[test]
fn test_comment() {
    assert_eq!(classify("! This is a comment"), LineKind::Comment);
}
#[test]
fn test_blank() {
    assert_eq!(classify(""), LineKind::Blank);
    assert_eq!(classify("   "), LineKind::Blank);
}
#[test]
fn test_plain_statement() {
    assert_eq!(classify("x = y + z"), LineKind::FortranStatement);
}
#[test]
fn test_use_statement() {
    assert_eq!(classify("use m_derived_types"), LineKind::FortranStatement);
}
#[test]
fn test_implicit_none() {
    assert_eq!(classify("implicit none"), LineKind::FortranStatement);
}
#[test]
fn test_labeled_do() {
    assert_eq!(classify("outer: do i = 1, n"), LineKind::FortranBlockOpen);
}
#[test]
fn test_module_procedure_is_statement() {
    assert_eq!(
        classify("module procedure my_proc"),
        LineKind::FortranStatement
    );
}
#[test]
fn test_program() {
    assert_eq!(classify("program main"), LineKind::FortranBlockOpen);
}
#[test]
fn test_end_program() {
    assert_eq!(classify("end program main"), LineKind::FortranBlockClose);
}
#[test]
fn test_case_default() {
    assert_eq!(classify("case default"), LineKind::FortranContinuation);
}
#[test]
fn test_forall() {
    assert_eq!(classify("forall (i=1:n)"), LineKind::FortranBlockOpen);
}
#[test]
fn test_end_forall() {
    assert_eq!(classify("end forall"), LineKind::FortranBlockClose);
}
#[test]
fn test_submodule() {
    assert_eq!(
        classify("submodule (parent) child"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_end_submodule() {
    assert_eq!(classify("end submodule child"), LineKind::FortranBlockClose);
}
#[test]
fn test_type_extends() {
    assert_eq!(
        classify("type, extends(base_type) :: derived_type"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_type_abstract() {
    assert_eq!(
        classify("type, abstract :: my_abstract"),
        LineKind::FortranBlockOpen
    );
}
#[test]
fn test_end_interface() {
    assert_eq!(classify("end interface"), LineKind::FortranBlockClose);
}
#[test]
fn test_interface_operator() {
    assert_eq!(
        classify("interface operator(+)"),
        LineKind::FortranBlockOpen
    );
}
