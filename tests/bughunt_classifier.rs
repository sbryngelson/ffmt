//! Regression tests for classifier bugs found in the 2026-06 bug hunt.
//!
//! Covers: scope names read from trailing comments, prefixed module functions,
//! `type::name`, parameterized-derived-type function prefixes, named `else`,
//! non-block labeled DO, numeric statement labels, `block data`, and
//! end-keywords followed by `=`.

use ffmt::classifier::{classify, extract_scope_name, LineKind};

fn format(src: &str) -> String {
    ffmt::format_string(src)
}

fn assert_idempotent(src: &str) {
    let first = format(src);
    let second = format(&first);
    assert_eq!(first, second, "formatting is not idempotent for:\n{src}");
}

// --- Finding 1: extract_scope_name reads name from trailing comment ---

#[test]
fn scope_name_ignores_trailing_comment() {
    assert_eq!(
        extract_scope_name("program main ! function test"),
        Some("main".to_string())
    );
    assert_eq!(
        extract_scope_name("subroutine foo(a) ! helper for subroutine bar"),
        Some("foo".to_string())
    );
    assert_eq!(
        extract_scope_name("module m ! function helper"),
        Some("m".to_string())
    );
}

#[test]
fn e2e_end_name_not_taken_from_comment() {
    let out = format("program main ! function test\n  x = 1\nend program\n");
    assert!(
        out.contains("end program main"),
        "expected 'end program main' in:\n{out}"
    );
    assert!(
        !out.contains("end program test"),
        "bogus end-name from comment in:\n{out}"
    );
}

// --- Finding 2: 'pure module function' misclassified as statement ---

#[test]
fn prefixed_module_function_is_block_open() {
    assert_eq!(
        classify("pure module function f(x) result(y)"),
        LineKind::FortranBlockOpen
    );
    assert_eq!(
        classify("impure elemental module function g()"),
        LineKind::FortranBlockOpen
    );
    assert_eq!(classify("module function h()"), LineKind::FortranBlockOpen);
    // Guard: 'module procedure' must remain a statement.
    assert_eq!(
        classify("module procedure my_proc"),
        LineKind::FortranStatement
    );
}

#[test]
fn e2e_prefixed_module_function_indents_body() {
    let src = "submodule (m) sm\ncontains\npure module function f(x) result(y)\ninteger :: x, y\ny = x\nend function f\nend submodule sm\n";
    let out = format(src);
    let lines: Vec<&str> = out.lines().collect();
    let body = lines
        .iter()
        .find(|l| l.trim_start().starts_with("y = x"))
        .expect("body line present");
    let func = lines
        .iter()
        .find(|l| l.trim_start().starts_with("pure module function"))
        .expect("function line present");
    let body_indent = body.len() - body.trim_start().len();
    let func_indent = func.len() - func.trim_start().len();
    assert!(
        body_indent > func_indent,
        "function body not indented inside function:\n{out}"
    );
    assert!(
        out.lines().any(|l| l == "end submodule sm"),
        "end submodule not at column 0:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 3: 'type::name' (no space) misclassified as statement ---

#[test]
fn type_double_colon_no_space_is_block_open() {
    assert_eq!(classify("type::foo"), LineKind::FortranBlockOpen);
    assert_eq!(classify("type ::foo"), LineKind::FortranBlockOpen);
    // Guards
    assert_eq!(classify("type(my_type) :: x"), LineKind::FortranStatement);
    assert_eq!(classify("type is (integer)"), LineKind::FortranContinuation);
}

#[test]
fn e2e_type_double_colon_idempotent_and_balanced() {
    let src =
        "module m\n  type::foo\n    integer :: x\n  end type foo\n  integer :: y\nend module m\n";
    let out = format(src);
    assert!(
        out.lines().any(|l| l == "end module m"),
        "end module not at column 0:\n{out}"
    );
    let lines: Vec<&str> = out.lines().collect();
    let member = lines
        .iter()
        .find(|l| l.trim_start().starts_with("integer :: x"))
        .expect("member line present");
    let type_line = lines
        .iter()
        .find(|l| l.trim_start().starts_with("type"))
        .expect("type line present");
    let member_indent = member.len() - member.trim_start().len();
    let type_indent = type_line.len() - type_line.trim_start().len();
    assert!(
        member_indent > type_indent,
        "type body not indented:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 4: parameterized-derived-type function prefix ---

#[test]
fn pdt_prefixed_function_is_block_open() {
    assert_eq!(
        classify("type(point(8)) function f(x)"),
        LineKind::FortranBlockOpen
    );
    assert_eq!(
        classify("type(matrix(kind=8,n=3)) function f(x)"),
        LineKind::FortranBlockOpen
    );
    // Simple form must keep working.
    assert_eq!(
        classify("type(point) function f(x)"),
        LineKind::FortranBlockOpen
    );
}

#[test]
fn e2e_pdt_function_indents_body() {
    let src =
        "module m\ncontains\ntype(point(8)) function f(x)\nf = x\nend function f\nend module m\n";
    let out = format(src);
    assert!(
        out.lines().any(|l| l == "end module m"),
        "end module not at column 0:\n{out}"
    );
    let lines: Vec<&str> = out.lines().collect();
    let body = lines
        .iter()
        .find(|l| l.trim_start().starts_with("f = x"))
        .expect("body line present");
    let func = lines
        .iter()
        .find(|l| l.trim_start().contains("function f"))
        .expect("function line present");
    let body_indent = body.len() - body.trim_start().len();
    let func_indent = func.len() - func.trim_start().len();
    assert!(
        body_indent > func_indent,
        "function body not indented:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 5: named 'else <construct-name>' ---

#[test]
fn named_else_is_continuation() {
    assert_eq!(classify("else outer"), LineKind::FortranContinuation);
    // Guards
    assert_eq!(classify("else"), LineKind::FortranContinuation);
    assert_eq!(classify("elsewhere"), LineKind::FortranContinuation);
    assert_eq!(classify("elsevar = 3"), LineKind::FortranStatement);
}

#[test]
fn e2e_named_else_aligned_with_if() {
    let src = "program p\n  outer: if (x) then\n    y = 1\n  else outer\n    y = 2\n  end if outer\nend program p\n";
    let out = format(src);
    let lines: Vec<&str> = out.lines().collect();
    let if_line = lines
        .iter()
        .find(|l| l.trim_start().starts_with("outer:"))
        .expect("if line present");
    let else_line = lines
        .iter()
        .find(|l| l.trim_start().starts_with("else outer"))
        .expect("else line present");
    let if_indent = if_line.len() - if_line.trim_start().len();
    let else_indent = else_line.len() - else_line.trim_start().len();
    assert_eq!(
        if_indent, else_indent,
        "'else outer' not aligned with its 'if':\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 6: non-block labeled DO must not open a scope ---

#[test]
fn labeled_do_is_not_block_open() {
    assert_eq!(classify("do 10 i = 1, n"), LineKind::FortranStatement);
    assert_eq!(classify("do 10, i = 1, n"), LineKind::FortranStatement);
    assert_eq!(classify("do 10 while (x < 1)"), LineKind::FortranStatement);
    // Guards: ordinary DO forms still open.
    assert_eq!(classify("do i = 1, n"), LineKind::FortranBlockOpen);
    assert_eq!(classify("do"), LineKind::FortranBlockOpen);
    assert_eq!(classify("do while (x < 1)"), LineKind::FortranBlockOpen);
    assert_eq!(classify("outer: do i = 1, n"), LineKind::FortranBlockOpen);
}

#[test]
fn e2e_labeled_do_keeps_file_balanced() {
    let src = "program p\n  do 10 i = 1, n\n    x = x + i\n10 continue\n  y = 1\nend program p\n";
    let out = format(src);
    assert!(
        out.lines().any(|l| l == "end program p"),
        "end program not at column 0:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 7: numeric statement labels stripped before classification ---

#[test]
fn numeric_labels_stripped_for_classification() {
    assert_eq!(classify("10 continue"), LineKind::FortranStatement);
    assert_eq!(classify("100 end if"), LineKind::FortranBlockClose);
    assert_eq!(classify("20 if (x > 0) then"), LineKind::FortranBlockOpen);
    assert_eq!(
        classify("30 end subroutine foo"),
        LineKind::FortranBlockClose
    );
    // A labeled 'end do' may terminate a non-block labeled DO; since
    // 'do <label>' does not push a scope, this must not pop one.
    assert_eq!(classify("10 end do"), LineKind::FortranStatement);
}

#[test]
fn e2e_labeled_end_if_closes_scope() {
    let src = "program t\n  if (x > 0) then\n    y = 1\n100 end if\nend program t\n";
    let out = format(src);
    assert!(
        out.lines().any(|l| l == "end program t"),
        "end program not at column 0:\n{out}"
    );
    assert_idempotent(src);
}

#[test]
fn e2e_labeled_if_opener_opens_scope() {
    let src = "program t\n20 if (x > 0) then\n  y = 1\nend if\nend program t\n";
    let out = format(src);
    let lines: Vec<&str> = out.lines().collect();
    let body = lines
        .iter()
        .find(|l| l.trim_start().starts_with("y = 1"))
        .expect("body line present");
    let opener = lines
        .iter()
        .find(|l| l.trim_start().starts_with("20 if"))
        .expect("labeled if present");
    let body_indent = body.len() - body.trim_start().len();
    let opener_indent = opener.len() - opener.trim_start().len();
    assert!(
        body_indent > opener_indent,
        "labeled if body not indented:\n{out}"
    );
    assert!(
        out.lines().any(|l| l == "end program t"),
        "end program not at column 0:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 8: 'block data' program unit ---

#[test]
fn block_data_is_block_open() {
    assert_eq!(classify("block data setup"), LineKind::FortranBlockOpen);
    assert_eq!(classify("block data"), LineKind::FortranBlockOpen);
    assert_eq!(classify("blockdata setup"), LineKind::FortranBlockOpen);
    assert_eq!(
        classify("end block data setup"),
        LineKind::FortranBlockClose
    );
    // Guard: assignment to a variable named blockdata.
    assert_eq!(classify("blockdata = 5"), LineKind::FortranStatement);
}

#[test]
fn e2e_block_data_balanced() {
    let src = "block data setup\ncommon /c/ x\ndata x/1.0/\nend block data setup\nprogram p\nx = 1\nend program p\n";
    let out = format(src);
    let lines: Vec<&str> = out.lines().collect();
    let body = lines
        .iter()
        .find(|l| l.trim_start().starts_with("common"))
        .expect("common line present");
    let body_indent = body.len() - body.trim_start().len();
    assert!(body_indent > 0, "block data body not indented:\n{out}");
    assert!(
        out.lines().any(|l| l == "end program p"),
        "end program not at column 0:\n{out}"
    );
    assert_idempotent(src);
}

// --- Finding 9: end-keyword followed by '=' is not a block close ---

#[test]
fn end_keyword_assignment_is_statement() {
    assert_eq!(classify("endif = 5"), LineKind::FortranStatement);
    assert_eq!(classify("endif=5"), LineKind::FortranStatement);
    assert_eq!(classify("end if = 5"), LineKind::FortranStatement);
    assert_eq!(classify("endwhere = 2"), LineKind::FortranStatement);
    assert_eq!(classify("enddo => p"), LineKind::FortranStatement);
    // Guards: real closers stay closers.
    assert_eq!(classify("end if"), LineKind::FortranBlockClose);
    assert_eq!(classify("endif"), LineKind::FortranBlockClose);
    assert_eq!(classify("end do"), LineKind::FortranBlockClose);
}

#[test]
fn e2e_endif_assignment_keeps_indentation() {
    let src = "program p\n  integer :: endif\n  endif = 5\nend program p\n";
    let out = format(src);
    // The classifier must not pop the program scope at 'endif = 5':
    // the closer stays at column 0 and the assignment stays indented.
    assert!(
        out.lines().any(|l| l == "end program p"),
        "end program not at column 0:\n{out}"
    );
    let assign = out
        .lines()
        .find(|l| l.trim_start().contains("= 5"))
        .expect("assignment present");
    let indent = assign.len() - assign.trim_start().len();
    assert!(indent > 0, "assignment dedented to column 0:\n{out}");
    assert_idempotent(src);
}
