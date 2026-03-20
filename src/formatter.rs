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
        let mut indent_delta: isize = 0;

        for (raw_idx, raw_line) in ll.raw_lines.iter().enumerate() {
            let trimmed = raw_line.trim_end();

            match kind {
                LineKind::Comment => {
                    let content = trimmed.trim_start();
                    // Normalize space after ! for regular comments
                    let content = normalize_comment_space(content);
                    if content.starts_with("!!") {
                        output_lines.push(trimmed.to_string());
                    } else {
                        output_lines.push(apply_indent(&content, depth, config.indent_width));
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
                        // Single-line statement: full normalization
                        let mut processed = process_line(trimmed, config);
                        if kind == LineKind::InlineFypp {
                            processed = normalize_fypp_lists(&processed);
                        }
                        let mut formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);

                        // Feature 3: Named end statements
                        if kind == LineKind::FortranBlockClose {
                            formatted = maybe_add_end_name(&formatted, &tracker, config);
                        }

                        output_lines.push(formatted);
                    } else if raw_idx == 0 {
                        // First line of multi-line statement: keyword + case only,
                        // preserve whitespace (developer has intentional alignment)
                        let orig_indent = leading_spaces(trimmed);
                        let mut processed = trimmed.to_string();
                        if config.normalize_keywords {
                            processed = normalize_keywords(&processed);
                        }
                        match config.keyword_case {
                            KeywordCase::Lower => processed = normalize_case(&processed),
                            KeywordCase::Upper => processed = normalize_case_upper(&processed),
                            KeywordCase::Preserve => {}
                        }
                        if kind == LineKind::InlineFypp {
                            processed = normalize_fypp_lists(&processed);
                        }
                        let formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);
                        let new_indent = leading_spaces(&formatted);
                        indent_delta = new_indent as isize - orig_indent as isize;
                        output_lines.push(formatted);
                    } else {
                        // Continuation line: full normalization + proportional re-indent
                        let formatted = process_continuation_line(trimmed, config, indent_delta);
                        output_lines.push(formatted);
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

/// Process a continuation line with full normalization and proportional re-indentation.
///
/// Continuation lines may start with optional leading whitespace and an optional `&`.
/// We:
/// 1. Strip the leading `&` (if present) and note its position
/// 2. Apply full normalization to the content
/// 3. Re-indent by applying the same delta that the first line received
fn process_continuation_line(line: &str, config: &Config, indent_delta: isize) -> String {
    let orig_indent = leading_spaces(line);
    let stripped = line.trim_start();

    // Check for leading & (continuation marker)
    let (has_leading_amp, content) = if let Some(after_amp) = stripped.strip_prefix('&') {
        (true, after_amp.trim_start())
    } else {
        (false, stripped)
    };

    // Check for trailing & (this line continues further)
    let (main_content, trailing) = split_trailing_amp(content);

    // Keyword normalization only on continuation lines.
    // Whitespace normalization is NOT applied because the normalizer
    // lacks context from the previous line and can't distinguish
    // binary operators (+ log(...)) from unary (-buff_size:n).
    let mut normalized = main_content.to_string();

    if config.normalize_keywords {
        normalized = normalize_keywords(&normalized);
    }

    match config.keyword_case {
        KeywordCase::Lower => normalized = normalize_case(&normalized),
        KeywordCase::Upper => normalized = normalize_case_upper(&normalized),
        KeywordCase::Preserve => {}
    }

    let normalized = normalized.trim();

    // Re-add trailing & (preserving any !& Fypp continuation after it)
    let with_trailing = if trailing.is_empty() {
        normalized.to_string()
    } else {
        let trail = trailing.trim();
        format!("{} {}", normalized, trail)
    };

    // Re-add leading & if it was present
    let with_leading = if has_leading_amp {
        format!("& {}", with_trailing)
    } else {
        with_trailing
    };

    // Apply proportional re-indentation
    let new_indent = if indent_delta >= 0 {
        orig_indent + indent_delta as usize
    } else {
        orig_indent.saturating_sub((-indent_delta) as usize)
    };

    let indent_str = " ".repeat(new_indent);
    format!("{}{}", indent_str, with_leading)
}

/// Split a line at its trailing `&` continuation marker (if present).
/// Returns (content_before_amp, trailing_amp_and_comment).
/// The `&` must be outside strings and comments.
fn split_trailing_amp(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut quote_char = b' ';
    let mut last_amp: Option<usize> = None;

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == quote_char {
                if i + 1 < bytes.len() && bytes[i + 1] == quote_char {
                    continue;
                }
                in_string = false;
            }
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_string = true;
            quote_char = b;
            continue;
        }
        if b == b'!' {
            // Comment starts -- stop looking
            break;
        }
        if b == b'&' {
            last_amp = Some(i);
        } else if !b.is_ascii_whitespace() {
            last_amp = None;
        }
    }

    match last_amp {
        Some(pos) => (line[..pos].trim_end(), &line[pos..]),
        None => (line.trim_end(), ""),
    }
}

/// Count leading spaces in a string.
fn leading_spaces(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// Uppercase Fortran keywords (inverse of normalize_case).
fn normalize_case_upper(line: &str) -> String {
    line.to_string()
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
