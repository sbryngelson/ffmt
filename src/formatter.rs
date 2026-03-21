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
                    let content = if config.unicode_to_ascii { crate::unicode::replace_unicode(content) } else { content.to_string() };
                    let content = if config.space_after_comment { normalize_comment_space(&content) } else { content };
                    if content.starts_with("!!") {
                        // Doxygen continuation that wasn't preceded by !>
                        let replaced = if config.unicode_to_ascii { crate::unicode::replace_unicode(trimmed) } else { trimmed.to_string() };
                        output_lines.push(replaced);
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
                            let cont_text = if config.unicode_to_ascii { crate::unicode::replace_unicode(cont_text) } else { cont_text.to_string() };
                            full_text.push(' ');
                            full_text.push_str(&cont_text);
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
                        let wrapped = if config.rewrap_comments { wrap_comment(&reconstructed, config.line_length, depth, config.indent_width) } else { vec![reconstructed.clone()] };
                        output_lines.extend(wrapped);
                    } else {
                        let indented = apply_indent(&content, depth, config.indent_width);
                        let wrapped = if config.rewrap_comments { wrap_comment(&indented, config.line_length, depth, config.indent_width) } else { vec![indented.clone()] };
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
                    if kind == LineKind::InlineFypp {
                        // InlineFypp ($:, @:): only Fypp-specific cleanups.
                        // Do NOT run whitespace normalization — Fypp lines contain
                        // Python code and Fypp quoting ({' ... '}) that the Fortran
                        // normalizer can't parse correctly.
                        // For multi-line, use the joined line (reader normalizes spacing).
                        let source = if ll.raw_lines.len() > 1 { &ll.joined } else { trimmed };
                        let mut processed = source.to_string();
                        if config.normalize_keywords {
                            processed = normalize_keywords(&processed);
                        }
                        match config.keyword_case {
                            KeywordCase::Lower => processed = normalize_case(&processed),
                            KeywordCase::Upper => processed = normalize_case_upper(&processed),
                            KeywordCase::Preserve => {}
                        }
                        if config.fypp_list_commas { processed = normalize_fypp_lists(&processed); }
                        processed = remove_fypp_macro_paren_space(&processed);
                        let formatted = apply_indent(processed.trim(), depth, config.indent_width);
                        // Rewrap long Fypp lines at top-level argument commas
                        if config.rewrap_code && formatted.len() > config.line_length {
                            let wrapped = rewrap_fypp_line(&formatted, config.line_length);
                            output_lines.extend(wrapped);
                        } else {
                            output_lines.push(formatted);
                        }
                        if ll.raw_lines.len() > 1 {
                            break; // Skip remaining raw lines
                        }
                    } else if ll.raw_lines.len() == 1 {
                        // Single-line: full normalization + rewrap if over limit
                        let processed = process_line(trimmed, config);
                        let mut formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);

                        if kind == LineKind::FortranBlockClose && config.named_ends {
                            formatted = maybe_add_end_name(&formatted, &tracker, config);
                        }

                        let wrapped = if config.rewrap_code { rewrap_line(&formatted, config.line_length, config.indent_width) } else { vec![formatted.clone()] };
                        output_lines.extend(wrapped);
                    } else if raw_idx == 0 {
                        // Check if the continuation was interrupted by a preprocessor
                        // directive (reader stopped at #ifdef/#ifndef). If so, don't
                        // unravel — preserve original structure to avoid breaking the
                        // Fortran statement that spans the preprocessor block.
                        let next_is_cpp = idx + 1 < ll_count && {
                            let next_kind = classify(&logical_lines[idx + 1].joined);
                            matches!(next_kind,
                                LineKind::PreprocessorDirective | LineKind::PreprocessorContinuation | LineKind::PreprocessorClose |
                                LineKind::FyppBlockOpen | LineKind::FyppBlockClose | LineKind::FyppContinuation | LineKind::FyppStatement
                            )
                        };

                        // Also check if the last raw line ends with & (meaning the
                        // continuation was genuinely cut short by a # directive)
                        let last_raw = ll.raw_lines.last().map(|s| s.trim_end()).unwrap_or("");
                        let continuation_interrupted = next_is_cpp && last_raw.ends_with('&');

                        if continuation_interrupted {
                            // Preserve original structure — emit each raw line
                            // with keyword + case normalization only
                            for (ri, rl) in ll.raw_lines.iter().enumerate() {
                                let t = rl.trim_end();
                                let mut processed = t.to_string();
                                if config.normalize_keywords {
                                    processed = normalize_keywords(&processed);
                                }
                                match config.keyword_case {
                                    KeywordCase::Lower => processed = normalize_case(&processed),
                                    KeywordCase::Upper => processed = normalize_case_upper(&processed),
                                    KeywordCase::Preserve => {}
                                }
                                if ri == 0 {
                                    output_lines.push(apply_indent(processed.trim(), depth, config.indent_width));
                                } else {
                                    output_lines.push(processed);
                                }
                            }
                            break;
                        }

                        // Normal multi-line: unravel joined line, normalize, rewrap
                        let processed = process_line(&ll.joined, config);
                        let formatted =
                            apply_indent(processed.trim(), depth, config.indent_width);

                        let formatted = if kind == LineKind::FortranBlockClose && config.named_ends {
                            maybe_add_end_name(&formatted, &tracker, config)
                        } else {
                            formatted
                        };

                        let wrapped = if config.rewrap_code { rewrap_line(&formatted, config.line_length, config.indent_width) } else { vec![formatted.clone()] };
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

    // Remove blank lines immediately before block closers/continuations
    output_lines = remove_blanks_before_closers(&output_lines);

    // Compact use statements (remove blank lines between consecutive use lines)
    if config.compact_use {
        output_lines = compact_use_statements(&output_lines);
    }

    // Align :: in consecutive declaration lines
    if config.align_declarations { output_lines = crate::align::align_declarations(&output_lines, config.compact_declarations, config.align_comments, config.line_length); }

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

    if config.keyword_paren_space {
        result = crate::whitespace::add_keyword_paren_spaces(&result);
    }
    result = crate::whitespace::normalize_intent_paren(&result);
    if config.collapse_double_spaces {
        result = crate::whitespace::collapse_double_spaces(&result);
    }

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

/// Split a Doxygen comment line that has multiple @ commands into separate lines.
/// E.g., `!> @file @brief Foo` → [`!> @file`, `!! @brief Foo`]
/// Returns the original line as a single-element Vec if no splitting needed.
fn split_doxygen_commands(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("!>") && !trimmed.starts_with("!!") {
        return vec![line.to_string()];
    }

    let indent = leading_spaces(line);
    let prefix = " ".repeat(indent);
    let marker = if trimmed.starts_with("!>") { "!>" } else { "!!" };
    let text = extract_comment_text(trimmed, marker);

    // Find @ commands in the text (@ followed by a letter)
    let mut at_positions: Vec<usize> = Vec::new();
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'@' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            // Don't count the first @ if it's at position 0 (that's the primary command)
            if i > 0 {
                at_positions.push(i);
            }
        }
    }

    if at_positions.is_empty() {
        return vec![line.to_string()];
    }

    // Split at each @ position
    let mut result = Vec::new();
    let mut prev = 0;
    let cont_marker = if marker == "!>" { "!!" } else { marker };

    for (idx, &pos) in at_positions.iter().enumerate() {
        let chunk = text[prev..pos].trim();
        if !chunk.is_empty() {
            let m = if idx == 0 { marker } else { cont_marker };
            result.push(format!("{}{} {}", prefix, m, chunk));
        }
        prev = pos;
    }
    // Last chunk
    let chunk = text[prev..].trim();
    if !chunk.is_empty() {
        let m = if result.is_empty() { marker } else { cont_marker };
        result.push(format!("{}{} {}", prefix, m, chunk));
    }

    if result.is_empty() {
        vec![line.to_string()]
    } else {
        result
    }
}

/// Extract the text content from a comment line, stripping the marker and leading space.
fn extract_comment_text<'a>(line: &'a str, marker: &str) -> &'a str {
    let after_marker = &line[marker.len()..];
    after_marker.strip_prefix(' ').unwrap_or(after_marker)
}

/// Wrap a long comment line at word boundaries.
/// Preserves the comment marker style (!, !>, !<, etc.)
fn wrap_comment(line: &str, max_length: usize, _depth: usize, _indent_width: usize) -> Vec<String> {
    // First: if a Doxygen line has multiple @ commands, split them
    let split_at_cmds = split_doxygen_commands(line);
    if split_at_cmds.len() > 1 {
        let mut result = Vec::new();
        for sub in &split_at_cmds {
            let sub_wrapped = wrap_comment(sub, max_length, _depth, _indent_width);
            result.extend(sub_wrapped);
        }
        return result;
    }

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

/// Remove blank lines between consecutive `use` statements.
fn compact_use_statements(lines: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim().to_ascii_lowercase();
        let is_use = trimmed.starts_with("use ") || trimmed == "use";

        // If this is a use statement, remove preceding blank lines
        // if the line before the blank was also a use statement
        if is_use && result.last().is_some_and(|l| l.trim().is_empty()) {
            // Check if there's a use statement before the blank
            let last_non_blank = result.iter().rev().find(|l| !l.trim().is_empty());
            if let Some(prev) = last_non_blank {
                let prev_lower = prev.trim().to_ascii_lowercase();
                if prev_lower.starts_with("use ") || prev_lower == "use" {
                    // Remove the blank line(s)
                    while result.last().is_some_and(|l| l.trim().is_empty()) {
                        result.pop();
                    }
                }
            }
        }

        result.push(line.clone());
    }

    result
}

/// Remove blank lines immediately before block closers/continuations
/// and immediately after block openers.
fn remove_blanks_before_closers(lines: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut prev_was_opener = false;

    for line in lines {
        let trimmed = line.trim().to_ascii_lowercase();

        let is_closer = trimmed.starts_with("else")
            || trimmed.starts_with("end ")
            || trimmed == "end"
            || trimmed.starts_with("case ")
            || trimmed == "case default"
            || trimmed.starts_with("class ")
            || trimmed == "class default"
            || trimmed.starts_with("type is")
            || trimmed.starts_with("rank ")
            || trimmed == "rank default"
            || trimmed.starts_with("elsewhere")
            || trimmed == "contains"
            || trimmed.starts_with("#:else")
            || trimmed.starts_with("#:elif")
            || trimmed.starts_with("#:endif")
            || trimmed.starts_with("#:endfor")
            || trimmed.starts_with("#:enddef")
            || trimmed.starts_with("#:endcall")
            || trimmed.starts_with("#:endblock")
            || trimmed.starts_with("#:endmute")
            || trimmed.starts_with("#endif")
            || trimmed.starts_with("#else");

        // Remove blank lines before closers
        if is_closer {
            while result.last().is_some_and(|l| l.trim().is_empty()) {
                result.pop();
            }
        }

        // Remove blank lines after openers
        if prev_was_opener && trimmed.is_empty() {
            prev_was_opener = false;
            continue; // skip this blank line
        }

        let is_opener = trimmed.ends_with("then")
            || (trimmed.starts_with("do ") && !trimmed.starts_with("do concurrent"))
            || trimmed.starts_with("do concurrent")
            || trimmed == "do"
            || trimmed.starts_with("select case")
            || trimmed.starts_with("select type")
            || trimmed.starts_with("select rank")
            || (trimmed.starts_with("subroutine ") || trimmed.contains(" subroutine "))
            || (trimmed.starts_with("function ") || trimmed.contains(" function "))
            || trimmed.starts_with("module ") && !trimmed.starts_with("module procedure")
            || trimmed.starts_with("program ")
            || trimmed.starts_with("block")
            || trimmed.starts_with("associate")
            || trimmed.starts_with("critical")
            || trimmed.starts_with("#:if ")
            || trimmed.starts_with("#:for ")
            || trimmed.starts_with("#:call ")
            || trimmed.starts_with("#:def ");

        prev_was_opener = is_opener;

        result.push(line.clone());
    }

    result
}

/// Re-wrap a long Fypp macro line ($: or @:) at top-level argument commas.
/// Only breaks between macro arguments (paren depth 1), never inside strings.
/// Continuation lines are aligned with the opening paren using & continuation.
fn rewrap_fypp_line(line: &str, max_length: usize) -> Vec<String> {
    let indent = leading_spaces(line);
    let content = line.trim_start();

    // Find the opening paren
    let paren_pos = match content.find('(') {
        Some(p) => p,
        None => return vec![line.to_string()],
    };

    let cont_indent = indent + paren_pos + 1;
    let cont_prefix = " ".repeat(cont_indent);

    // Walk the content and find comma positions where we can break.
    // Break at: top-level commas (paren depth 1, between macro args)
    // AND commas inside '[...]' Fypp list strings (safe to break with &).
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut in_fypp_list = false; // inside '[...]' within a string
    let mut quote_char = b' ';
    let mut comma_positions: Vec<usize> = Vec::new();

    for i in 0..len {
        let b = bytes[i];
        if in_string {
            if b == quote_char {
                if i + 1 < len && bytes[i + 1] == quote_char {
                    continue;
                }
                in_string = false;
                in_fypp_list = false;
            } else if b == b'[' && !in_fypp_list {
                in_fypp_list = true;
            } else if b == b']' && in_fypp_list {
                in_fypp_list = false;
            } else if b == b',' && in_fypp_list {
                // Comma inside '[...]' Fypp list — safe to break
                let mut end = i + 1;
                if end < len && bytes[end] == b' ' {
                    end += 1;
                }
                comma_positions.push(end);
            }
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_string = true;
            quote_char = b;
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth -= 1;
        } else if b == b',' && depth == 1 {
            // Top-level comma between macro arguments
            let mut end = i + 1;
            if end < len && bytes[end] == b' ' {
                end += 1;
            }
            comma_positions.push(end);
        }
    }

    if comma_positions.is_empty() {
        return vec![line.to_string()];
    }

    // Build lines by greedily packing content until exceeding max_length
    let indent_str = " ".repeat(indent);
    let cont_with_amp = format!("{}& ", cont_prefix);

    let mut result: Vec<String> = Vec::new();
    let mut chunk_start = 0usize;

    for &comma_end in &comma_positions {
        
        let current_len = if result.is_empty() {
            indent + comma_end
        } else {
            cont_with_amp.len() + (comma_end - chunk_start)
        };

        if current_len > max_length && chunk_start < comma_end {
            // Find the last comma that fits
            let mut best_break = chunk_start;
            for &cp in &comma_positions {
                if cp <= chunk_start {
                    continue;
                }
                let len_with_break = if result.is_empty() {
                    indent + cp
                } else {
                    cont_with_amp.len() + (cp - chunk_start)
                };
                if len_with_break <= max_length {
                    best_break = cp;
                } else {
                    break;
                }
            }

            if best_break > chunk_start {
                let segment = content[chunk_start..best_break].trim_end();
                if result.is_empty() {
                    result.push(format!("{}{} &", indent_str, segment));
                } else {
                    result.push(format!("{}& {} &", cont_prefix, segment));
                }
                chunk_start = best_break;
                // Skip leading space
                while chunk_start < len && bytes[chunk_start] == b' ' {
                    chunk_start += 1;
                }
            }
        }
    }

    // Emit remaining content
    let remaining = content[chunk_start..].trim_end();
    if result.is_empty() {
        result.push(format!("{}{}", indent_str, remaining));
    } else {
        result.push(format!("{}& {}", cont_prefix, remaining));
    }

    result
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

    // Align continuation lines with opening paren if present,
    // otherwise use flat indent (one indent_width deeper).
    let paren_col = content.find('(').map(|p| indent + p + 1);
    let cont_indent = paren_col.unwrap_or(indent + indent_width);
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

/// Remove space between Fypp macro name and ( on $: and @: lines.
/// E.g., "$:GPU_PARALLEL_LOOP (collapse=3)" -> "$:GPU_PARALLEL_LOOP(collapse=3)"
fn remove_fypp_macro_paren_space(line: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?m)^(\s*[@$]:[\w]+)\s+\(").unwrap()
    });
    re.replace(line, r"${1}(").to_string()
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
