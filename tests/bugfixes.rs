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
        // Disable the (intended) unicode->ascii comment conversion: this
        // test pins byte-level UTF-8 integrity, not conversion policy.
        unicode_to_ascii: false,
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
    assert_eq!(classify("if (ch == '(') then"), LineKind::FortranBlockOpen);
}

#[test]
fn test_classify_if_then_with_close_paren_in_string() {
    assert_eq!(classify("if (ch == ')') then"), LineKind::FortranBlockOpen);
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

// --- Blank line inside a continued statement (MFC m_riemann_solvers.fpp) ---

#[test]
fn test_blank_line_inside_continuation_removed() {
    // ffmt <= 0.4.1 preserved a stray blank line between a trailing-& line and
    // its leading-& continuation forever. The statement must be rejoined and
    // rewrapped with the blank gone.
    let src = "\
module m

    implicit none

contains

    subroutine s_riemann_solver(qL_prim_rsx_vf, dqL_prim_dx_vf, dqL_prim_dy_vf, dqL_prim_dz_vf, &

        & qL_prim_vf, qR_prim_rsx_vf, dqR_prim_dx_vf, dqR_prim_dy_vf, dqR_prim_dz_vf, qR_prim_vf, q_prim_vf, flux_vf, &
            & flux_src_vf, flux_gsrc_vf, norm_dir, ix, iy, iz)

        integer :: norm_dir

    end subroutine s_riemann_solver

end module m
";
    let out = ffmt::format_string(src);
    // No blank line may remain between a `&` line and its continuation
    let lines: Vec<&str> = out.lines().collect();
    for w in lines.windows(2) {
        assert!(
            !(w[0].trim_end().ends_with('&') && w[1].trim().is_empty()),
            "blank line after continuation `&` survived:\n{out}"
        );
    }
    assert!(
        out.contains("norm_dir, ix, iy, iz)"),
        "argument list lost:\n{out}"
    );
}

#[test]
fn test_short_continuation_with_blank_rejoined() {
    // Also for statements short enough that no rewrap is triggered.
    let src = "program t\n    x = a + &\n\n        & b\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        !out.contains("+ &\n\n"),
        "blank inside continuation survived:\n{out}"
    );
    assert!(
        out.contains("x = a + b") || out.contains("& b"),
        "statement content lost:\n{out}"
    );
}

// --- Comments are never left (or lost) inside a continuation ---

#[test]
fn test_mid_continuation_comment_hoisted_not_dropped() {
    // ffmt <= 0.4.1 silently DELETED a comment line placed between
    // continuation lines. It must be hoisted above the statement instead.
    let src = "program t\n    call foo(a, &\n        ! note\n        & b)\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(out.contains("! note"), "comment was deleted:\n{out}");
    let note_pos = out.find("! note").unwrap();
    let call_pos = out.find("call foo").unwrap();
    assert!(
        note_pos < call_pos,
        "comment not hoisted above statement:\n{out}"
    );
    assert!(
        out.contains("call foo(a, b)"),
        "statement not rejoined:\n{out}"
    );
}

#[test]
fn test_trailing_amp_comment_hoisted_not_dropped() {
    let src = "program t\n    call foo(a, & ! why a\n        & b)\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(out.contains("! why a"), "comment was deleted:\n{out}");
    assert!(
        out.contains("call foo(a, b)"),
        "statement not rejoined:\n{out}"
    );
    assert!(
        out.find("! why a").unwrap() < out.find("call foo").unwrap(),
        "comment not hoisted above statement:\n{out}"
    );
}

#[test]
fn test_blank_and_comment_inside_continuation_both_handled() {
    let src = "program t\n    call foo(a, &\n\n        ! note\n        & b)\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(out.contains("! note"), "comment was deleted:\n{out}");
    assert!(
        out.contains("call foo(a, b)"),
        "statement not rejoined:\n{out}"
    );
}

#[test]
fn test_hoisted_comments_idempotent() {
    let src = "program t\n    call foo(a, &\n        ! note one\n        ! note two\n        & b)\nend program t\n";
    let once = ffmt::format_string(src);
    let twice = ffmt::format_string(&once);
    assert_eq!(once, twice, "hoisting is not idempotent");
    // Consecutive short comments are merged by join_short_comments (general
    // ffmt comment policy) — both texts must survive, in order.
    let one = once.find("note one").expect("first comment lost");
    let two = once.find("note two").expect("second comment lost");
    assert!(one < two, "comment order not preserved:\n{once}");
    let comment_pos = once.find("! note").expect("comment marker lost");
    assert!(
        comment_pos < once.find("call foo").unwrap(),
        "comments not hoisted above statement:\n{once}"
    );
}

#[test]
fn test_ffmt_off_keeps_mid_continuation_comment_verbatim() {
    let src = "program t\n    ! ffmt off\n    call foo(a, &\n        ! note\n        & b)\n    ! ffmt on\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("call foo(a, &\n        ! note\n        & b)"),
        "ffmt off region was not preserved verbatim:\n{out}"
    );
}

#[test]
fn test_trailing_amp_comment_no_space_hoisted() {
    // `& !this is a comment` (no space after `!`) must also be preserved.
    let src = "program t\n    call foo(a, & !this is a comment\n        & b)\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("this is a comment"),
        "comment was deleted:\n{out}"
    );
    assert!(
        out.contains("call foo(a, b)"),
        "statement not rejoined:\n{out}"
    );
}

// --- ffmt off: verbatim text AND scope tracking through the region ---

#[test]
fn test_ffmt_off_keeps_bang_amp_and_spacing_verbatim() {
    let src = "program t\n    ! ffmt off\n    x   =   1 !&\n    y = 2 ! note!&\n    ! ffmt on\nend program t\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("    x   =   1 !&\n    y = 2 ! note!&"),
        "ffmt off region was altered:\n{out}"
    );
}

#[test]
fn test_scope_tracked_through_ffmt_off_region() {
    // A block opener inside the off region, closed outside: indentation and
    // the named end after the region must still be correct.
    let src = "\
subroutine s()
    ! ffmt off
    if (cond) then
    ! ffmt on
        x = 1
    end if
end subroutine
";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("\n        x = 1\n"),
        "depth lost across ffmt off:\n{out}"
    );
    assert!(out.contains("\n    end if\n"), "end if depth wrong:\n{out}");
    assert!(
        out.contains("end subroutine s"),
        "named end wrong after ffmt off:\n{out}"
    );
}

// --- Doubled-quote escapes in rewrap/align scanners ---

#[test]
fn test_split_trailing_comment_escaped_quote_long_line() {
    // The `!` is inside the string (after a '' escape); a long line must not
    // be split at it as if it were a trailing comment.
    let lit = "'it''s ! not a comment padding padding padding padding padding'";
    let src = format!(
        "program t\n    call some_subroutine_name(argument_one, argument_two, argument_three, {lit}, argument_four)\nend program t\n"
    );
    let out = ffmt::format_string(&src);
    let rejoined = out
        .replace(" &\n", " ")
        .replace("\n", " ")
        .replace("& ", "");
    assert!(
        rejoined.contains("it''s ! not a comment padding"),
        "string literal split at interior '!':\n{out}"
    );
}

#[test]
fn test_align_assignments_escaped_quote_string_not_padded() {
    let config = ffmt::Config {
        align_assignments: ffmt::config::Toggle::Enable,
        ..ffmt::Config::default()
    };
    let src = "program t\n    a = 'x''= y'\n    longer_name = 2\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("'x''= y'"),
        "padding inserted inside string literal:\n{out}"
    );
}

#[test]
fn test_rewrap_paren_align_ignores_paren_in_escaped_string() {
    // The only `(` on the line is inside a string (after a '' escape at a
    // position that confuses a broken scanner). Wrapping must not align
    // continuations to it, and must not break inside the string.
    let lit = "'aa''bb (cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt uu vv ww xx yy zz'";
    let src = format!("program t\n    result_variable_name = {lit}//suffix_variable//another_suffix_variable//yet_another_one\nend program t\n");
    let out = ffmt::format_string(&src);
    let rejoined = out
        .replace(" &\n", " ")
        .replace("\n", " ")
        .replace("& ", "");
    assert!(
        rejoined.contains("aa''bb (cc"),
        "string literal damaged by rewrap:\n{out}"
    );
    let twice = ffmt::format_string(&out);
    assert_eq!(out, twice, "rewrap not idempotent");
}

// --- Long directives must never be rewrapped as comments ---

#[test]
fn test_long_acc_directive_not_rewrapped() {
    let dir = "!$acc parallel loop gang vector collapse(3) default(present) private(alpha_rho_k, alpha_k, velocity_components) reduction(+:total_energy_sum) copyin(boundary_conditions)";
    let src = format!("program t\n    {dir}\n    x = 1\nend program t\n");
    let out = ffmt::format_string(&src);
    assert!(out.contains(dir), "directive was altered/rewrapped:\n{out}");
}

#[test]
fn test_long_omp_directive_not_rewrapped() {
    let dir = "!$omp parallel do schedule(dynamic) default(none) shared(very_long_array_name_one, very_long_array_name_two) private(loop_index_variable_i, loop_index_variable_j) reduction(+:accumulator)";
    let src = format!("program t\n    {dir}\n    x = 1\nend program t\n");
    let out = ffmt::format_string(&src);
    assert!(out.contains(dir), "directive was altered/rewrapped:\n{out}");
}

// --- Doxygen / comment passes ---

#[test]
fn test_email_in_doxygen_not_split() {
    let src = "module m\n    !> Contact bob@example.com with questions\n    implicit none\nend module m\n";
    let out = ffmt::format_string(src);
    assert!(
        out.contains("bob@example.com"),
        "email split as doxygen command:\n{out}"
    );
    assert_eq!(
        out.matches("!>").count() + out.matches("!!").count(),
        1,
        "comment was split:\n{out}"
    );
}

#[test]
fn test_doxygen_split_at_real_commands_still_works() {
    let src = "module m\n    !> @file demo.f90 @brief Does demo things\n    implicit none\nend module m\n";
    let out = ffmt::format_string(src);
    assert!(out.contains("@file"), "{out}");
    assert!(out.contains("@brief"), "{out}");
    assert!(
        out.matches('@').count() == 2 && out.contains("!!"),
        "real doxygen commands no longer split:\n{out}"
    );
}

#[test]
fn test_doxygen_not_joined_when_rewrap_comments_disabled() {
    let config = ffmt::Config {
        rewrap_comments: ffmt::config::Toggle::Disable,
        ..ffmt::Config::default()
    };
    let src = "module m\n    !> First line of doc\n    !! second line kept separate\n    implicit none\nend module m\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("!> First line of doc\n") && out.contains("!! second line kept separate"),
        "doxygen joined despite rewrap-comments=false:\n{out}"
    );
}

#[test]
fn test_range_format_does_not_eat_doxygen_outside_range() {
    // Range covers ONLY line 2 (the !> line); the !! continuation on line 3
    // must remain untouched and unconsumed.
    let src = "module m\n!> doc start\n        !!    weird   continuation   spacing\n    implicit none\nend module m\n";
    let out = ffmt::format_range(src, 2, 2);
    assert!(
        out.contains("        !!    weird   continuation   spacing"),
        "doxygen continuation outside range was modified:\n{out}"
    );
}

#[test]
fn test_unicode_to_ascii_applies_to_inline_comments() {
    let config = ffmt::Config {
        unicode_to_ascii: true,
        ..ffmt::Config::default()
    };
    let src =
        "program t\n    x = 1  ! \u{2014} em\u{2013}dash \u{201c}quoted\u{201d}\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        !out.contains('\u{2014}') && !out.contains('\u{201c}'),
        "unicode left in inline comment:\n{out}"
    );
}

// --- Alignment / blank-line post-passes ---

#[test]
fn test_blanks_kept_around_keyword_prefixed_identifiers() {
    // blocksize/critical_temp/else_branch are ordinary variables; the
    // blank-line management around openers/closers must not fire on them.
    let src = "\
program t
    integer :: a

    blocksize = 1

    critical_temp = 2.0

    else_branch = 3

    a = blocksize
end program t
";
    let out = ffmt::format_string(src);
    assert_eq!(
        out.matches("\n\n").count(),
        5,
        "blank lines were added/removed around ordinary statements:\n{out}"
    );
}

#[test]
fn test_class_default_still_treated_as_closer() {
    // The classifier-driven closer detection must still remove the blank
    // before select-type continuations like `class default`.
    let src = "\
subroutine s(v)
    select type (v)
    type is (integer)
        i = 1

    class default
        i = 2
    end select
end subroutine s
";
    let out = ffmt::format_string(src);
    assert!(
        !out.contains("\n\n    class default"),
        "blank before class default not removed:\n{out}"
    );
}

#[test]
fn test_align_assignments_skips_do_control_and_indent_groups() {
    let config = ffmt::Config {
        align_assignments: ffmt::config::Toggle::Enable,
        ..ffmt::Config::default()
    };
    let src = "program t\n    do i = 1, n\n        x = 1\n        long_name = 2\n    end do\nend program t\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("do i = 1, n"),
        "do-loop control '=' was aligned:\n{out}"
    );
    assert!(
        out.contains("x         = 1"),
        "body assignments not aligned together:\n{out}"
    );
}

#[test]
fn test_two_space_comment_respects_line_length() {
    // "    " + name + " = 1 " + "! c" with name of 120 chars == exactly 132.
    let long_name = "y".repeat(120);
    let src = format!("program t\n    {long_name} = 1 ! c\nend program t\n");
    let line_len_before = src.lines().nth(1).unwrap().len();
    assert_eq!(line_len_before, 132);
    let out = ffmt::format_string(&src);
    let line = out.lines().find(|l| l.contains("! c")).unwrap();
    assert!(
        line.len() <= 132,
        "two-space pass pushed line to {} chars:\n{line}",
        line.len()
    );
}

// --- use-statement reformatting, split-statements, range containment ---

#[test]
fn test_use_reformat_preserves_trailing_comment() {
    let config = ffmt::Config {
        use_formatting: ffmt::config::Toggle::Enable,
        ..ffmt::Config::default()
    };
    let src = "module m\n    use other, only: a, b ! important comment\nend module m\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        out.contains("important comment"),
        "trailing comment deleted by use reformatting:\n{out}"
    );
}

#[test]
fn test_use_reformat_ignores_only_inside_comment() {
    let config = ffmt::Config {
        use_formatting: ffmt::config::Toggle::Enable,
        ..ffmt::Config::default()
    };
    let src = "module m\n    use other ! only: a, b\nend module m\n";
    let out = ffmt::format_string_with_config(src, &config);
    assert!(
        !out.contains("&"),
        "use line exploded from comment text:\n{out}"
    );
    assert!(out.contains("! only: a, b"), "comment lost:\n{out}");
}

#[test]
fn test_double_colon_then_declaration_blank_idempotent() {
    let src = "subroutine s\n    integer i\n    i = 1\nend subroutine s\n";
    let once = ffmt::format_string(src);
    let twice = ffmt::format_string(&once);
    assert_eq!(
        once, twice,
        "pass ordering non-idempotent:\nonce:\n{once}\ntwice:\n{twice}"
    );
}

#[test]
fn test_split_statements_reindents_and_is_idempotent() {
    let config = ffmt::Config {
        split_statements: ffmt::config::Toggle::Enable,
        ..ffmt::Config::default()
    };
    let src = "program t\nif (a) then; b = 1; end if\nend program t\n";
    let once = ffmt::format_string_with_config(src, &config);
    assert!(
        once.contains("if (a) then\n        b = 1\n    end if"),
        "split statements not re-indented:\n{once}"
    );
    let twice = ffmt::format_string_with_config(&once, &config);
    assert_eq!(once, twice, "split-statements not idempotent");
}

#[test]
fn test_range_format_leaves_outside_lines_untouched() {
    let src = "if (a.eq.b) x=1\ny=2\nz=3\n";
    let out = ffmt::format_range(src, 3, 3);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "if (a.eq.b) x=1",
        "line 1 outside range was rewritten:\n{out}"
    );
    assert_eq!(
        lines[1], "y=2",
        "line 2 outside range was rewritten:\n{out}"
    );
    assert_eq!(
        lines[2], "z = 3",
        "line 3 inside range not formatted:\n{out}"
    );
}
