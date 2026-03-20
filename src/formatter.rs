use crate::case_norm::normalize_case;
use crate::classifier::{classify, end_block_keyword, end_statement_has_name, extract_scope_name, LineKind};
use crate::config::{Config, KeywordCase};
use crate::keyword_norm::normalize_keywords;
use crate::reader::read_logical_lines;
use crate::scope::ScopeTracker;
use crate::whitespace::normalize_whitespace;


/// Ensure a space after `!` in regular comments (not directives, Doxygen, Fypp).
fn normalize_comment_space(comment: &str) -> String {
    let bytes = comment.as_bytes();
    if bytes.is_empty() || bytes[0] != b'!' {
        return comment.to_string();
    }
    if bytes.len() == 1 {
        return comment.to_string();
    }
    let second = bytes[1];
    if second == b' '
        || second == b'$'
        || second == b'<'
        || second == b'>'
        || second == b'!'
        || second == b'*'
        || second == b'@'
        || second == b'&'
    {
        return comment.to_string();
    }
    format!("! {}", &comment[1..])
}

/// Apply indentation to a line. Strips existing leading whitespace and adds
/// `depth * indent_width` spaces. Blank lines remain blank.
fn apply_indent(line: &str, depth: usize, indent_width: usize) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }
    let indent = " ".repeat(depth * indent_width);
    format!("{}{}", indent, trimmed)
}

/// Check if a line is an `end subroutine` or `end function` (for procedure spacing).
fn is_end_procedure(line: &str) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let re_end_proc = regex::Regex::new(r"(?i)^end\s+(subroutine|function)\b").unwrap();
    re_end_proc.is_match(&lower)
}

/// Check if a line starts a new procedure (subroutine or function).
fn is_procedure_start(line: &str) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let re_proc = regex::Regex::new(
        r"(?i)^(?:(?:pure|elemental|impure|recursive|module|integer|real|double\s+precision|complex|character|logical|type\s*\([^)]*\))\s+)*(subroutine|function)\b"
    ).unwrap();
    re_proc.is_match(&lower)
}

/// Format a Fortran source string using default config.
pub fn format(source: &str) -> String {
    format_with_config(source, &Config::default(), None)
}

/// Format with an optional line range (1-based, inclusive).
pub fn format_with_range(source: &str, range: Option<(usize, usize)>) -> String {
    format_with_config(source, &Config::default(), range)
}

/// Format with full config and optional range.
pub fn format_with_config(
    source: &str,
    config: &Config,
    range: Option<(usize, usize)>,
) -> String {
    let logical_lines = read_logical_lines(source);
    let mut tracker = ScopeTracker::new();
    let mut output_lines: Vec<String> = Vec::new();
    let mut consecutive_blanks: usize = 0;
    let mut skip_next_blank = false;
    // Track whether we just emitted an end procedure line (for blank line insertion).
    let mut just_ended_procedure = false;
    // Track if we're inside a procedure body (for collapsing blanks to 1).
    let mut procedure_depth: Vec<usize> = Vec::new();

    let ll_count = logical_lines.len();
    let mut idx = 0;

    while idx < ll_count {
        let ll = &logical_lines[idx];
        let kind = classify(&ll.joined);

        // Check range
        let last_raw_line = ll.line_number + ll.raw_lines.len() - 1;
        let in_range = match range {
            None => true,
            Some((start, end)) => ll.line_number <= end && last_raw_line >= start,
        };

        // --- Blank line handling ---
        if kind == LineKind::Blank {
            if in_range && skip_next_blank {
                skip_next_blank = false;
                idx += 1;
                continue;
            }
            if in_range {
                consecutive_blanks += 1;
                // Inside procedures: collapse 2+ blanks to 1
                // Outside procedures (but in contains): collapse 2+ to 1
                // General: collapse 3+ to 2
                let max_blanks = if !procedure_depth.is_empty() || tracker.in_contains() {
                    1
                } else {
                    2
                };
                if consecutive_blanks > max_blanks {
                    idx += 1;
                    continue;
                }
            }
            let _ = tracker.process(kind);
            output_lines.push(String::new());
            idx += 1;
            continue;
        }

        // If we just ended a procedure and this non-blank line is not another end/contains,
        // we may need to ensure exactly one blank line before it.
        if just_ended_procedure && in_range && tracker.in_contains() {
            let is_end_or_contains = kind == LineKind::FortranBlockClose || kind == LineKind::FortranContains;
            if !is_end_or_contains {
                // This should be the start of a new procedure. Ensure exactly one blank line.
                // Remove excess blank lines (already handled by collapse above).
                // If there are zero blank lines, insert one.
                if consecutive_blanks == 0 {
                    output_lines.push(String::new());
                }
            }
        }

        consecutive_blanks = 0;
        skip_next_blank = false;
        just_ended_procedure = false;

        // --- Track procedure entry/exit ---
        if kind == LineKind::FortranBlockOpen && tracker.in_contains() && is_procedure_start(&ll.joined) {
            procedure_depth.push(tracker.current_depth());
        }

        // --- Scope tracking (always runs) ---
        // Extract scope name for block openers
        let scope_name = if kind == LineKind::FortranBlockOpen {
            extract_scope_name(&ll.joined)
        } else {
            None
        };

        let mut depth = tracker.process_with_name(kind, scope_name);

        // Track procedure exit
        if kind == LineKind::FortranBlockClose {
            if let Some(&proc_depth) = procedure_depth.last() {
                if tracker.current_depth() <= proc_depth {
                    procedure_depth.pop();
                }
            }
        }

        if !config.indent_fypp
            && matches!(
                kind,
                LineKind::FyppBlockOpen
                    | LineKind::FyppBlockClose
                    | LineKind::FyppContinuation
                    | LineKind::FyppStatement
                    | LineKind::InlineFypp
            )
        {
            depth = 0;
        }

        // Check for acc loop directive
        if kind == LineKind::Directive {
            let content_lower = ll.joined.trim().to_ascii_lowercase();
            if content_lower.contains("!$acc loop")
                || content_lower.contains("!$acc parallel loop")
            {
                skip_next_blank = true;
            }
        }

        // --- If outside range, emit unchanged ---
        if !in_range {
            for raw_line in &ll.raw_lines {
                output_lines.push(raw_line.trim_end().to_string());
            }
            idx += 1;
            continue;
        }

        // --- Emit each raw line with formatting ---
        // Track the indent delta from the first line so continuation lines
        // can be shifted proportionally.
        

        for (raw_idx, raw_line) in ll.raw_lines.iter().enumerate() {
            let trimmed = raw_line.trim_end();

            match kind {
                LineKind::Comment => {
                    let content = trimmed.trim_start();
                    let content = normalize_comment_space(content);
                    if content.starts_with("!!") {
                        // Doxygen continuation that wasn't preceded by !> — preserve
                        output_lines.push(trimmed.to_string());
                    } else if content.starts_with("!>") {
                        // Doxygen start: collect any following !! continuation lines,
                        // join the text, and re-wrap as a single block
                        let marker_text = extract_comment_text(&content, "!>");
                        let mut full_text = marker_text.to_string();

                        // Look ahead for !! continuation lines
                        while idx + 1 < ll_count {
                            let next_ll = &logical_lines[idx + 1];
                            let next_kind = classify(&next_ll.joined);
                            if next_kind != LineKind::Comment {
                                break;
                            }
                            let next_content = next_ll.joined.trim().trim_start();
                            if !next_content.starts_with("!!") {
                                break;
                            }
                            let cont_text = extract_comment_text(next_content, "!!");
                            // Don't join if continuation starts with @
                            // (separate Doxygen command) or is blank
                            if cont_text.starts_with('@') || cont_text.trim().is_empty() {
                                break;
                            }
                            full_text.push(' ');
                            full_text.push_str(cont_text);
                            idx += 1;
                            // Process scope for skipped lines
                            let _ = tracker.process(next_kind);
                        }

                        let indent_str = " ".repeat(depth * config.indent_width);
                        let full_trimmed = full_text.trim();
                        let reconstructed = if full_trimmed.is_empty() {
                            format!("{}!>", indent_str)
                        } else {
                            format!("{}!> {}", indent_str, full_trimmed)
                        };
                        let wrapped = wrap_comment(&reconstructed, config.line_length, depth, config.indent_width);
                        output_lines.extend(wrapped);
                    } else {
                        let indented = apply_indent(&content, depth, config.indent_width);
                        let wrapped = wrap_comment(&indented, config.line_length, depth, config.indent_width);
                        output_lines.extend(wrapped);
                    }
                }
                LineKind::Blank => unreachable!(),
                LineKind::PreprocessorDirective
                | LineKind::PreprocessorContinuation
                | LineKind::PreprocessorClose => {
                    output_lines.push(trimmed.to_string());
                }
                LineKind::FyppStatement
                | LineKind::FyppBlockOpen
                | LineKind::FyppBlockClose
                | LineKind::FyppContinuation => {
                    if raw_idx == 0 {
                        output_lines.push(apply_indent(
                            trimmed.trim_start(),
                            depth,
                            config.indent_width,
                        ));
                    } else {
                        // Fypp continuation lines: preserve entirely
                        output_lines.push(trimmed.to_string());
                    }
                }
                _ => {
                    if ll.raw_lines.len() == 1 {
                        // Single-line: full normalization + rewrap if over limit
                        let mut processed = process_line(trimmed, config);
                        if kind == LineKind::InlineFypp {
                            processed = normalize_fypp_lists(&processed);
                        }
                        let mut formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);

                        if kind == LineKind::FortranBlockClose {
                            formatted = maybe_add_end_name(&formatted, &tracker, config);
                        }

                        let wrapped = rewrap_line(&formatted, config.line_length, config.indent_width);
                        output_lines.extend(wrapped);
                    } else if raw_idx == 0 {
                        // Multi-line: unravel joined line, normalize, rewrap
                        let mut processed = process_line(&ll.joined, config);
                        if kind == LineKind::InlineFypp {
                            processed = normalize_fypp_lists(&processed);
                        }
                        let formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);

                        let formatted = if kind == LineKind::FortranBlockClose {
                            maybe_add_end_name(&formatted, &tracker, config)
                        } else {
                            formatted
                        };

                        let wrapped = rewrap_line(&formatted, config.line_length, config.indent_width);
                        output_lines.extend(wrapped);
                        break; // Skip remaining raw lines
                    } else {
                        unreachable!("raw_idx > 0 not reached after break");
                    }
                }
            }
        }

        // After emitting an end subroutine/function, mark for blank line insertion
        if kind == LineKind::FortranBlockClose {
            if let Some(last_line) = output_lines.last() {
                if is_end_procedure(last_line) {
                    just_ended_procedure = true;
                }
            }
        }

        idx += 1;
    }

    // Strip trailing blank lines
    while output_lines.last().is_some_and(|l| l.is_empty()) {
        output_lines.pop();
    }

    // Align :: in consecutive declaration lines
    output_lines = crate::align::align_declarations(&output_lines);

    let mut result = output_lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Add the scope name to a bare `end subroutine/function/module/program/type` line.
fn maybe_add_end_name(line: &str, tracker: &ScopeTracker, config: &Config) -> String {
    // Only apply to named block types, not end if/do/select etc.
    if end_statement_has_name(line) {
        return line.to_string();
    }

    // Check if this is a nameable end statement
    if let Some(_keyword) = end_block_keyword(line) {
        if let Some(name) = tracker.last_closed_name() {
            // Apply case normalization to the name
            let final_name = match config.keyword_case {
                KeywordCase::Lower => name.to_ascii_lowercase(),
                KeywordCase::Upper => name.to_ascii_uppercase(),
                KeywordCase::Preserve => name.to_string(),
            };
            // Rebuild the line: preserve indentation, add name
            let trimmed = line.trim();
            let indent = &line[..line.len() - line.trim_start().len()];
            // Strip any trailing comment from the end statement
            let comment_start = trimmed.find('!');
            let (stmt_part, comment_part) = if let Some(pos) = comment_start {
                (trimmed[..pos].trim_end(), Some(&trimmed[pos..]))
            } else {
                (trimmed, None)
            };
            let mut result = format!("{}{} {}", indent, stmt_part, final_name);
            if let Some(comment) = comment_part {
                result = format!("{} {}", result, comment);
            }
            return result;
        }
    }

    line.to_string()
}

/// Process a single Fortran line: keyword norm -> whitespace norm -> case norm.
fn process_line(line: &str, config: &Config) -> String {
    let mut result = line.to_string();

    if config.normalize_keywords {
        result = normalize_keywords(&result);
    }

    result = normalize_whitespace(&result, &config.whitespace);

    match config.keyword_case {
        KeywordCase::Lower => result = normalize_case(&result),
        KeywordCase::Upper => result = normalize_case_upper(&result),
        KeywordCase::Preserve => {}
    }

    result
}



/// Count leading spaces in a string.
fn leading_spaces(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// Uppercase Fortran keywords (inverse of normalize_case).
fn normalize_case_upper(line: &str) -> String {
    line.to_string()
}

/// Extract the text content from a comment line, stripping the marker and leading space.
fn extract_comment_text<'a>(line: &'a str, marker: &str) -> &'a str {
    let after_marker = &line[marker.len()..];
    after_marker.strip_prefix(' ').unwrap_or(after_marker)
}

/// Wrap a long comment line at word boundaries.
/// Preserves the comment marker style (!, !>, !<, etc.)
fn wrap_comment(line: &str, max_length: usize, _depth: usize, _indent_width: usize) -> Vec<String> {
    if line.len() <= max_length || max_length >= 1000 {
        return vec![line.to_string()];
    }

    let indent = leading_spaces(line);
    let content = line.trim_start();

    // Extract comment marker: !, !>, !<, !* etc.
    let marker = if content.starts_with("!>") {
        "!>"
    } else if content.starts_with("!<") {
        "!<"
    } else if content.starts_with("!*") {
        "!*"
    } else {
        "!"
    };

    // Get the text after the marker (and optional space)
    let text_start = if content.len() > marker.len() && content.as_bytes()[marker.len()] == b' ' {
        marker.len() + 1
    } else {
        marker.len()
    };
    let text = &content[text_start..];

    let prefix = " ".repeat(indent);
    let first_prefix = format!("{}{} ", prefix, marker);
    // Doxygen: first line uses !>, continuation lines use !!
    let cont_marker = if marker == "!>" { "!!" } else { marker };
    let cont_prefix = format!("{}{} ", prefix, cont_marker);
    let first_avail = if max_length > first_prefix.len() {
        max_length - first_prefix.len()
    } else {
        40
    };
    let cont_avail = if max_length > cont_prefix.len() {
        max_length - cont_prefix.len()
    } else {
        40
    };

    // Split text at word boundaries
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut is_first = true;

    for word in &words {
        let avail = if is_first { first_avail } else { cont_avail };
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= avail {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            let pfx = if is_first { &first_prefix } else { &cont_prefix };
            result.push(format!("{}{}", pfx, current_line));
            current_line = word.to_string();
            is_first = false;
        }
    }
    if !current_line.is_empty() {
        let pfx = if is_first { &first_prefix } else { &cont_prefix };
        result.push(format!("{}{}", pfx, current_line));
    }

    if result.is_empty() {
        vec![line.to_string()]
    } else {
        result
    }
}

/// Token-aware line re-wrapping. Breaks a long line at token boundaries,
/// preferring commas, then operators. Never breaks inside tokens (strings,
/// numbers, identifiers), preventing issues like splitting `1.e-32`.
///
/// Returns a Vec of output lines. If the input fits within `max_length`,
/// returns it unchanged as a single-element Vec.
fn rewrap_line(line: &str, max_length: usize, indent_width: usize) -> Vec<String> {
    if line.len() <= max_length || max_length >= 1000 {
        return vec![line.to_string()];
    }

    let indent = leading_spaces(line);
    let content = line.trim_start();

    // Find token boundaries in the content
    let breaks = find_token_breaks(content);

    if breaks.is_empty() {
        return vec![line.to_string()];
    }

    let cont_indent = indent + indent_width;
    let cont_prefix = " ".repeat(cont_indent);

    // Available width: max_length minus indent minus " &" suffix
    let first_avail = max_length.saturating_sub(indent + 2);
    let cont_avail = max_length.saturating_sub(cont_indent + 4); // "& " prefix + " &" suffix

    let mut result: Vec<String> = Vec::new();
    let mut pos = 0usize;
    let mut is_first = true;

    while pos < content.len() {
        let avail = if is_first { first_avail } else { cont_avail };
        let remaining = &content[pos..];

        // If remaining fits, emit as final line
        if remaining.len() <= avail + 2 {
            if is_first {
                result.push(format!("{}{}", " ".repeat(indent), remaining));
            } else {
                result.push(format!("{}& {}", cont_prefix, remaining));
            }
            break;
        }

        // Find best break point within avail characters
        let abs_limit = pos + avail;
        

        // Walk through token breaks, find the last one before the limit
        // Prefer commas (BreakKind::Comma) over operators
        let mut last_comma = 0usize;
        let mut last_other = 0usize;

        for &(bp, kind) in &breaks {
            if bp <= pos {
                continue;
            }
            let rel = bp - pos;
            if bp > abs_limit {
                break;
            }
            match kind {
                BreakKind::Comma => last_comma = rel,
                _ => last_other = rel,
            }
        }

        let best_break = if last_comma > 0 {
            last_comma
        } else if last_other > 0 {
            last_other
        } else {
            // No break point found — emit as-is (over-length)
            if is_first {
                result.push(format!("{}{}", " ".repeat(indent), remaining));
            } else {
                result.push(format!("{}& {}", cont_prefix, remaining));
            }
            break;
        };

        let chunk = content[pos..pos + best_break].trim_end();
        if is_first {
            result.push(format!("{}{} &", " ".repeat(indent), chunk));
        } else {
            result.push(format!("{}& {} &", cont_prefix, chunk));
        }

        pos += best_break;
        // Skip leading whitespace on next chunk
        while pos < content.len() && content.as_bytes()[pos] == b' ' {
            pos += 1;
        }
        is_first = false;
    }

    if result.is_empty() {
        vec![line.to_string()]
    } else {
        result
    }
}

#[derive(Clone, Copy)]
enum BreakKind {
    Comma,
    Operator,
    CloseParen,
}

/// Find positions in `content` where it's safe to break the line.
/// Returns (position_after_break_char, kind) pairs.
/// Positions are byte offsets where a line break can be inserted.
fn find_token_breaks(content: &str) -> Vec<(usize, BreakKind)> {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut breaks = Vec::new();
    let mut i = 0;
    let mut in_string = false;
    let mut quote_char = b' ';
    let mut paren_depth = 0i32;

    while i < len {
        let b = bytes[i];

        // Track strings — never break inside
        if in_string {
            if b == quote_char {
                if i + 1 < len && bytes[i + 1] == quote_char {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        if b == b'\'' || b == b'"' {
            in_string = true;
            quote_char = b;
            i += 1;
            continue;
        }

        // Track parens
        if b == b'(' || b == b'[' {
            paren_depth += 1;
        } else if b == b')' || b == b']' {
            paren_depth -= 1;
            if paren_depth < 0 {
                paren_depth = 0;
            }
            // Break after close paren at any depth
            breaks.push((i + 1, BreakKind::CloseParen));
        }

        // Comma — best break point (after the comma + space)
        if b == b',' {
            // Skip trailing space after comma
            let mut end = i + 1;
            if end < len && bytes[end] == b' ' {
                end += 1;
            }
            breaks.push((end, BreakKind::Comma));
        }

        // Binary operators at any paren depth
        {
            // Multi-char operators: check for //, /=, ==, <=, >=, =>, **
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                // // (concat) — break before it if preceded by space
                if i > 0 && bytes[i - 1] == b' ' {
                    breaks.push((i, BreakKind::Operator));
                }
                i += 2;
                continue;
            }

            // + and - as binary operators (not unary, not exponent)
            if (b == b'+' || b == b'-') && i > 0 {
                let prev = bytes[i - 1];
                // Binary if preceded by space, close paren, alphanumeric, or _
                if prev == b' ' || prev == b')' || prev == b']'
                    || prev.is_ascii_alphanumeric() || prev == b'_'
                {
                    // But not if it's part of exponent notation (e+3, d-5)
                    if !(i >= 2
                        && (bytes[i - 1] == b'e'
                            || bytes[i - 1] == b'E'
                            || bytes[i - 1] == b'd'
                            || bytes[i - 1] == b'D')
                        && bytes[i - 2].is_ascii_digit())
                    {
                        // Break before the operator if preceded by space
                        if prev == b' ' {
                            breaks.push((i, BreakKind::Operator));
                        }
                    }
                }
            }
        }

        // Comment starts — stop
        if b == b'!' {
            break;
        }

        i += 1;
    }

    breaks
}

/// Normalize comma spacing inside Fypp '[...]' list arguments.
/// E.g., `'[x,y,z,Ys_L, Ys_R]'` -> `'[x, y, z, Ys_L, Ys_R]'`
/// Only modifies content inside `'[` ... `]'` patterns.
fn normalize_fypp_lists(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for '[
        if i + 1 < len && bytes[i] == b'\'' && bytes[i + 1] == b'[' {
            // Find the matching ]'
            let start = i;
            i += 2; // skip '[
            let mut content = String::new();
            while i < len {
                if bytes[i] == b']' && i + 1 < len && bytes[i + 1] == b'\'' {
                    // Found ]' -- normalize commas in content
                    let normalized_content = normalize_comma_spacing(&content);
                    result.push_str("'[");
                    result.push_str(&normalized_content);
                    result.push_str("]'");
                    i += 2; // skip ]'
                    break;
                }
                content.push(bytes[i] as char);
                i += 1;
            }
            // If we didn't find ]', emit original
            if i >= len && !result.ends_with("]'") {
                result.push_str(&line[start..]);
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Normalize comma spacing: ensure exactly one space after each comma.
fn normalize_comma_spacing(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b',' {
            result.push(',');
            result.push(' ');
            // Skip any whitespace after the comma
            i += 1;
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}
