use ffmt::classifier::LineKind;
use ffmt::scope::ScopeTracker;

#[test]
fn test_empty() {
    let tracker = ScopeTracker::new();
    assert_eq!(tracker.current_depth(), 0);
}

#[test]
fn test_simple_block() {
    let mut tracker = ScopeTracker::new();
    let depth = tracker.process(LineKind::FortranBlockOpen);
    assert_eq!(depth, 0);
    assert_eq!(tracker.current_depth(), 1);
    let depth = tracker.process(LineKind::FortranBlockClose);
    assert_eq!(depth, 0);
    assert_eq!(tracker.current_depth(), 0);
}

#[test]
fn test_nested_blocks() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen); // module
    tracker.process(LineKind::FortranBlockOpen); // subroutine
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockOpen); // do
    assert_eq!(tracker.current_depth(), 3);
    tracker.process(LineKind::FortranBlockClose); // end do
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockClose); // end subroutine
    assert_eq!(tracker.current_depth(), 1);
    tracker.process(LineKind::FortranBlockClose); // end module
    assert_eq!(tracker.current_depth(), 0);
}

#[test]
fn test_else_continuation() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen); // if...then
    let depth = tracker.process(LineKind::FortranContinuation); // else
    assert_eq!(depth, 0);
    assert_eq!(tracker.current_depth(), 1);
}

#[test]
fn test_contains() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen); // module at depth 0
    assert_eq!(tracker.current_depth(), 1);
    let depth = tracker.process(LineKind::FortranContains);
    assert_eq!(depth, 0);
    assert_eq!(tracker.current_depth(), 1);
}

#[test]
fn test_fypp_block() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    tracker.process(LineKind::FyppBlockOpen);
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FyppContinuation); // #:else
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FyppBlockClose);
    assert_eq!(tracker.current_depth(), 1);
}

#[test]
fn test_preprocessor_no_indent() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    let depth_before = tracker.current_depth();
    tracker.process(LineKind::PreprocessorDirective);
    assert_eq!(tracker.current_depth(), depth_before);
    tracker.process(LineKind::PreprocessorClose);
    assert_eq!(tracker.current_depth(), depth_before);
}

#[test]
fn test_directive_no_scope_change() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    tracker.process(LineKind::FortranBlockOpen);
    let depth = tracker.current_depth();
    let dir_depth = tracker.process(LineKind::Directive);
    assert_eq!(dir_depth, depth);
    assert_eq!(tracker.current_depth(), depth);
}

#[test]
fn test_mismatched_close_recovery() {
    let mut tracker = ScopeTracker::new();
    let depth = tracker.process(LineKind::FortranBlockClose);
    assert_eq!(depth, 0);
    assert_eq!(tracker.current_depth(), 0);
}

#[test]
fn test_statement_and_comment_no_change() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    assert_eq!(tracker.process(LineKind::FortranStatement), 1);
    assert_eq!(tracker.process(LineKind::Comment), 1);
    assert_eq!(tracker.process(LineKind::Blank), 1);
    assert_eq!(tracker.process(LineKind::InlineFypp), 1);
    assert_eq!(tracker.process(LineKind::FyppStatement), 1);
}

#[test]
fn test_bare_end_pops_stack() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    tracker.process(LineKind::FortranBlockOpen);
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockClose);
    assert_eq!(tracker.current_depth(), 1);
}

#[test]
fn test_nested_contains() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen); // module at 0
    assert_eq!(tracker.current_depth(), 1);
    let d = tracker.process(LineKind::FortranContains);
    assert_eq!(d, 0);
    assert_eq!(tracker.current_depth(), 1);
    tracker.process(LineKind::FortranBlockOpen); // subroutine at 1
    assert_eq!(tracker.current_depth(), 2);
    let d = tracker.process(LineKind::FortranContains);
    assert_eq!(d, 1);
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockOpen); // internal function at 2
    assert_eq!(tracker.current_depth(), 3);
    tracker.process(LineKind::FortranBlockClose);
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockClose);
    assert_eq!(tracker.current_depth(), 1);
    tracker.process(LineKind::FortranBlockClose);
    assert_eq!(tracker.current_depth(), 0);
}

#[test]
fn test_fypp_call_with_nested_fortran() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen); // subroutine at 0
    tracker.process(LineKind::FyppBlockOpen); // #:call at 1
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FortranBlockOpen); // do at 2
    assert_eq!(tracker.current_depth(), 3);
    tracker.process(LineKind::FortranBlockClose); // end do
    assert_eq!(tracker.current_depth(), 2);
    tracker.process(LineKind::FyppBlockClose); // #:endcall
    assert_eq!(tracker.current_depth(), 1);
}

#[test]
fn test_preprocessor_lines_return_current_depth() {
    let mut tracker = ScopeTracker::new();
    tracker.process(LineKind::FortranBlockOpen);
    let d = tracker.process(LineKind::PreprocessorDirective);
    assert_eq!(d, 1);
    assert_eq!(tracker.current_depth(), 1);
    let d = tracker.process(LineKind::PreprocessorClose);
    assert_eq!(d, 1);
    assert_eq!(tracker.current_depth(), 1);
}
