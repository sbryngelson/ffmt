/// A logical Fortran line: one or more raw lines joined at `&` continuations.
#[derive(Debug, Clone)]
pub struct LogicalLine {
    pub joined: String,
    pub raw_lines: Vec<String>,
    pub line_number: usize,
    /// Comments found inside the `&` continuation: full-line comments between
    /// continuation lines and `& ! comment` trailers. Comments are never left
    /// in the middle of a continuation (some compilers reject them and they
    /// are lost when the statement is re-emitted from `joined`) — the
    /// formatter hoists these above the statement. OpenMP/OpenACC `!$`
    /// sentinels are excluded: hoisting one to its own line would turn an
    /// inert trailer into an active directive.
    pub hoisted_comments: Vec<String>,
    /// Emit `raw_lines` verbatim instead of rebuilding from `joined`. Set when
    /// the continuation contains something that cannot be moved or rebuilt
    /// safely: a `!$` directive sentinel (full-line or as a trailer after the
    /// `&`), or a malformed character-literal continuation.
    pub preserve: bool,
}

impl LogicalLine {
    fn single(line: &str, line_number: usize) -> Self {
        LogicalLine {
            joined: line.to_string(),
            raw_lines: vec![line.to_string()],
            line_number,
            hoisted_comments: Vec::new(),
            preserve: false,
        }
    }
}

/// Result of scanning one physical line with the string state carried in
/// from a preceding character-literal continuation.
struct LineScan {
    /// Byte position of a trailing continuation `&` (the last non-blank
    /// character before commentary outside strings, or the last non-blank
    /// character of the line while inside a character literal), or None.
    amp: Option<usize>,
    /// Byte position where commentary (`!` outside any string) starts.
    comment_start: Option<usize>,
    /// String state after this line: Some(delim) only when the line ends
    /// with a continuation `&` while inside a character literal.
    end_state: Option<char>,
    /// Whether the trailing `&` sits inside a character literal.
    amp_in_string: bool,
}

/// Scan the body of a line, beginning in string state `start`, looking for
/// the comment start and the trailing continuation `&`.
fn scan_line(line: &str, start: Option<char>) -> LineScan {
    let bytes_len = line.len();
    let mut in_string = start;
    let mut comment_start: Option<usize> = None;

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut k = 0;
    while k < chars.len() {
        let (bi, ch) = chars[k];
        if let Some(d) = in_string {
            if ch == d {
                if k + 1 < chars.len() && chars[k + 1].1 == d {
                    k += 2; // doubled-quote escape
                    continue;
                }
                in_string = None;
            }
        } else if ch == '\'' || ch == '"' {
            in_string = Some(ch);
        } else if ch == '!' {
            comment_start = Some(bi);
            break;
        }
        k += 1;
    }

    // The candidate continuation `&` is the last non-blank character of the
    // code region (everything before commentary; the whole line when the
    // line ends inside a string, since `!` in a string is not commentary).
    let scan_end = comment_start.unwrap_or(bytes_len);
    let code = line[..scan_end].trim_end();
    let amp = if code.ends_with('&') {
        Some(code.len() - 1)
    } else {
        None
    };

    // String state at the trailing `&` equals the state at end of scan
    // (the `&` is the last non-blank, so no quote can follow it).
    let amp_in_string = amp.is_some() && in_string.is_some();

    let end_state = if amp.is_some() { in_string } else { None };

    LineScan {
        amp,
        comment_start,
        end_state,
        amp_in_string,
    }
}

/// Public helper kept for compatibility: trailing continuation `&` position
/// for a line that does not start inside a string.
fn find_continuation_amp(line: &str) -> Option<usize> {
    scan_line(line, None).amp
}

/// Strip trailing `!&` from a line, but ONLY when the `!&` run is the entire
/// comment (`code !&`, `code !&!&`). `!&` is a no-op marker used to suppress
/// continuation warnings; it carries no meaning. A `!&` at the end of
/// ordinary comment text (`! see foo!&`) is text and must be kept.
fn strip_trailing_bang_amp(line: &str, start_state: Option<char>) -> String {
    if !line.trim_end().ends_with("!&") {
        return line.to_string();
    }
    let scan = scan_line(line, start_state);
    let Some(cs) = scan.comment_start else {
        // `!&` is inside a string literal — keep.
        return line.to_string();
    };
    let comment = line[cs..].trim_end();
    // The whole comment must be a run of `!&` (optionally blank-separated).
    let is_noop_run = {
        let mut rest = comment;
        let mut ok = !rest.is_empty();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix("!&") {
                rest = r.trim_start();
            } else {
                ok = false;
                break;
            }
        }
        ok
    };
    if is_noop_run {
        line[..cs].trim_end().to_string()
    } else {
        line.to_string()
    }
}

/// Detect a `& ! comment` line (continuation marker followed only by a
/// comment) and convert it to a plain comment line. Cray ftn (ftn-71)
/// rejects these. Returns Some(converted) when the line matched.
/// Must only be called for lines that do not start inside a string.
fn amp_comment_to_comment(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let stripped = trimmed.strip_prefix('&')?;
    let after_amp = stripped.trim_start();
    if after_amp.starts_with('!') && !after_amp.starts_with("!$") {
        let indent = line.len() - trimmed.len();
        Some(format!("{}{}", " ".repeat(indent), after_amp))
    } else {
        None
    }
}

fn is_ffmt_off_marker(line: &str) -> bool {
    let t = line.trim().to_ascii_lowercase();
    t == "! ffmt off" || t == "! ffmt: off"
}

fn is_ffmt_on_marker(line: &str) -> bool {
    let t = line.trim().to_ascii_lowercase();
    t == "! ffmt on" || t == "! ffmt: on"
}

pub fn read_logical_lines(source: &str) -> Vec<LogicalLine> {
    let raw: Vec<&str> = source.split('\n').collect();
    let raw_count = if source.ends_with('\n') && !raw.is_empty() {
        raw.len() - 1
    } else {
        raw.len()
    };

    // --- Pass 1: cleaning + per-line string-state scan -------------------
    // Cleaning (`!&` stripping, `& !` conversion) is suppressed inside
    // `! ffmt off` regions (those must be verbatim) and on lines that begin
    // inside a continued character literal (their text is string content).
    let mut cleaned: Vec<String> = Vec::with_capacity(raw_count);
    let mut was_amp_comment = vec![false; raw_count];
    let mut starts_in_string: Vec<Option<char>> = Vec::with_capacity(raw_count);
    let mut in_off_region = vec![false; raw_count];

    let mut state: Option<char> = None;
    let mut off = false;
    for (i, &line) in raw.iter().take(raw_count).enumerate() {
        if state.is_none() {
            if is_ffmt_off_marker(line) {
                off = true;
            } else if is_ffmt_on_marker(line) {
                off = false;
            }
        }
        in_off_region[i] = off;
        starts_in_string.push(state);

        let mut l = line.to_string();
        if !off && state.is_none() {
            l = strip_trailing_bang_amp(&l, None);
            if let Some(conv) = amp_comment_to_comment(&l) {
                l = conv;
                was_amp_comment[i] = true;
            }
        }

        // Advance string state. Blank lines and comment lines inside a
        // character-literal continuation pass the state through (they are
        // comment lines per F2018 6.3.2.3); a content line must begin with
        // `&` and resumes the literal right after it.
        let t = l.trim_start();
        state = if let Some(d) = state {
            if t.is_empty() || t.starts_with('!') {
                Some(d)
            } else if let Some(rest) = t.strip_prefix('&') {
                scan_line(rest, Some(d)).end_state
            } else {
                None // malformed character continuation — reset
            }
        } else {
            scan_line(&l, None).end_state
        };
        cleaned.push(l);
    }

    // --- Pass 2: build logical lines -------------------------------------
    let mut result: Vec<LogicalLine> = Vec::new();
    let mut i = 0usize;

    while i < raw_count {
        let raw_line = cleaned[i].as_str();
        let line_number = i + 1;

        // Verbatim inside `! ffmt off` regions: one LogicalLine per raw line.
        if in_off_region[i] {
            result.push(LogicalLine::single(raw_line, line_number));
            i += 1;
            continue;
        }

        // Blank line — always its own LogicalLine.
        if raw_line.trim().is_empty() {
            result.push(LogicalLine::single(raw_line, line_number));
            i += 1;
            continue;
        }

        // Full-line comment (`! ...`, including `!$` directives).
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with('!') {
            result.push(LogicalLine::single(raw_line, line_number));
            i += 1;
            continue;
        }

        let first_scan = scan_line(raw_line, None);
        let Some(first_amp) = first_scan.amp else {
            // Simple line, no continuation.
            result.push(LogicalLine::single(raw_line, line_number));
            i += 1;
            continue;
        };

        // --- Fortran continuation: gather lines joined by `&` ------------
        let mut raw_lines_acc: Vec<String> = vec![raw_line.to_string()];
        let mut joined = String::new();
        let mut hoisted: Vec<String> = Vec::new();
        let mut preserve = false;
        // String state carried across the junction (Some(d) when the
        // pending trailing `&` is inside a character literal).
        let mut carry: Option<char> = first_scan.end_state;
        // Pending text contributed up to (not including) the trailing `&`,
        // NOT yet trimmed: trimming depends on the next line's form.
        let mut pending: String = raw_line[..first_amp].to_string();

        // A `& ! comment` trailer after the continuation `&` (outside
        // strings) is hoisted; a `& !$dir` trailer forces preserve.
        let check_trailer = |line: &str,
                             amp: usize,
                             amp_in_string: bool,
                             comment_start: Option<usize>,
                             hoisted: &mut Vec<String>,
                             preserve: &mut bool| {
            if amp_in_string {
                return;
            }
            if let Some(cs) = comment_start {
                if cs > amp {
                    let trailer = line[cs..].trim_end();
                    if trailer.starts_with("!$") {
                        *preserve = true;
                    } else {
                        hoisted.push(trailer.to_string());
                    }
                }
            }
        };
        check_trailer(
            raw_line,
            first_amp,
            first_scan.amp_in_string,
            first_scan.comment_start,
            &mut hoisted,
            &mut preserve,
        );

        loop {
            if i + 1 >= raw_count {
                i += 1;
                break;
            }
            let next_idx = i + 1;
            let cont_line = cleaned[next_idx].as_str();
            let cont_trim = cont_line.trim_start();

            // ffmt-off marker terminates the continuation conservatively.
            if in_off_region[next_idx] {
                preserve = true;
                i += 1;
                break;
            }

            // Preprocessor directives interrupt the continuation — the
            // formatter preserves the raw structure for these.
            if cont_trim.starts_with('#') {
                i += 1;
                break;
            }

            // Inside a character literal the continuation line must begin
            // with `&` (blank/comment lines may intervene).
            let in_string_here = carry.is_some();

            // Blank line: a comment line per the standard — skip it; the
            // statement resumes at the next non-comment line.
            if cont_trim.is_empty() {
                raw_lines_acc.push(cont_line.to_string());
                i += 1;
                continue;
            }

            // Full-line `!$` directive sentinel inside the continuation
            // (bare or behind a leading `&`): cannot be hoisted (it would
            // change semantics) and cannot stay mid-statement when
            // rebuilding — preserve the raw structure.
            let is_directive_line = cont_trim.starts_with("!$")
                || cont_trim
                    .strip_prefix('&')
                    .is_some_and(|r| r.trim_start().starts_with("!$"));
            if !in_string_here && is_directive_line {
                preserve = true;
                raw_lines_acc.push(cont_line.to_string());
                i += 1;
                continue;
            }

            // Comment line.
            if !in_string_here && cont_trim.starts_with('!') {
                if was_amp_comment[next_idx] {
                    // Originally `& ! comment`: a continuation line with
                    // empty content that does not itself end with `&` — the
                    // statement TERMINATES here (gfortran semantics). The
                    // dangling `&` is repaired unless code follows on a
                    // leading-`&` line (then we join and hoist: the input
                    // was invalid, this repairs it).
                    let mut peek = next_idx + 1;
                    while peek < raw_count {
                        let p = cleaned[peek].as_str().trim_start();
                        if p.is_empty() || (p.starts_with('!') && !p.starts_with("!$")) {
                            peek += 1;
                            continue;
                        }
                        break;
                    }
                    let amp_follows =
                        peek < raw_count && cleaned[peek].as_str().trim_start().starts_with('&');
                    if amp_follows {
                        hoisted.push(cont_trim.trim_end().to_string());
                        raw_lines_acc.push(cont_line.to_string());
                        i += 1;
                        continue;
                    } else {
                        // Terminate: strip the dangling `&` from the last
                        // code line; the comment becomes its own line.
                        if let Some(last_raw) = raw_lines_acc.last_mut() {
                            if let Some(pos) = find_continuation_amp(last_raw) {
                                *last_raw = last_raw[..pos].trim_end().to_string();
                            }
                        }
                        i += 1;
                        break;
                    }
                }
                // Plain comment line inside the continuation: hoist it and
                // keep going — the statement resumes at the next
                // non-comment line whether or not it has a leading `&`.
                hoisted.push(cont_trim.trim_end().to_string());
                raw_lines_acc.push(cont_line.to_string());
                i += 1;
                continue;
            }

            // --- Code line: splice it ------------------------------------
            let had_leading_amp = cont_trim.starts_with('&');

            if in_string_here && !had_leading_amp {
                // Malformed character continuation — keep everything as-is.
                preserve = true;
                raw_lines_acc.push(cont_line.to_string());
                // Recover: treat this line as ending the statement.
                joined.push_str(&pending);
                pending = String::new();
                joined.push(' ');
                joined.push_str(cont_trim.trim_end());
                i = next_idx + 1;
                break;
            }

            raw_lines_acc.push(cont_line.to_string());

            // The contributed content cuts at the trailing `&` when the
            // statement continues further. On the FINAL line (no trailing
            // `&`), keep the whole text INCLUDING any inline comment — it
            // belongs to the statement and is emitted as a normal trailing
            // comment.
            let (content, this_scan): (String, LineScan) = if had_leading_amp {
                let amp_off = cont_line.len() - cont_trim.len();
                let after = &cont_line[amp_off + 1..];
                let scan = scan_line(after, carry);
                let content = match scan.amp {
                    Some(a) => after[..a].to_string(),
                    None => after.trim_end().to_string(),
                };
                (content, scan)
            } else {
                let scan = scan_line(cont_line, None);
                let content = match scan.amp {
                    Some(a) => cont_line[..a].to_string(),
                    None => cont_line.trim_end().to_string(),
                };
                (content, scan)
            };

            if let Some(a_rel) = this_scan.amp {
                // Find absolute amp pos for trailer check.
                let (line_for_trailer, abs_amp) = if had_leading_amp {
                    let amp_off = cont_line.len() - cont_trim.len();
                    (cont_line, amp_off + 1 + a_rel)
                } else {
                    (cont_line, a_rel)
                };
                let abs_comment = this_scan.comment_start.map(|c| {
                    if had_leading_amp {
                        let amp_off = cont_line.len() - cont_trim.len();
                        amp_off + 1 + c
                    } else {
                        c
                    }
                });
                check_trailer(
                    line_for_trailer,
                    abs_amp,
                    this_scan.amp_in_string,
                    abs_comment,
                    &mut hoisted,
                    &mut preserve,
                );
            }

            // Splice `pending` + this content per F2018 6.3.2.4:
            //  - inside a character literal: verbatim — every character on
            //    both sides of the junction is string content;
            //  - leading `&`, no blanks at the junction: the two pieces glue
            //    into one token (split identifiers/literals) — verbatim;
            //  - leading `&` with blanks on either side: the tokens are
            //    separate; normalize to a single blank (the historical
            //    joined form, which downstream passes expect). `%` member
            //    access glues tight to keep rewrapped statements stable;
            //  - no leading `&`: continue at the first non-blank character,
            //    with one implicit blank.
            if had_leading_amp {
                if in_string_here {
                    joined.push_str(&pending);
                    joined.push_str(&content);
                } else if content.trim_start().starts_with('%') {
                    joined.push_str(pending.trim_end());
                    joined.push_str(content.trim_start());
                } else {
                    let ws_boundary = pending.ends_with(char::is_whitespace)
                        || pending.is_empty()
                        || content.starts_with(char::is_whitespace)
                        || content.is_empty();
                    if ws_boundary {
                        joined.push_str(pending.trim_end());
                        joined.push(' ');
                        joined.push_str(content.trim_start());
                    } else {
                        joined.push_str(&pending);
                        joined.push_str(&content);
                    }
                }
            } else {
                joined.push_str(pending.trim_end());
                joined.push(' ');
                joined.push_str(content.trim_start());
            }

            carry = this_scan.end_state;
            i = next_idx; // this line is consumed

            if this_scan.amp.is_none() {
                pending = String::new();
                i += 1; // point past the final line of the statement
                break;
            }
            // The spliced content is now the prefix of the next junction:
            // move it back into pending.
            pending = joined;
            joined = String::new();
        }

        if !pending.is_empty() {
            // Continuation ran off the end of the file (or terminated at a
            // `& !` comment): emit what we have, without the dangling text
            // being lost.
            joined.push_str(pending.trim_end());
        }

        result.push(LogicalLine {
            joined,
            raw_lines: raw_lines_acc,
            line_number,
            hoisted_comments: hoisted,
            preserve,
        });
    }

    result
}
