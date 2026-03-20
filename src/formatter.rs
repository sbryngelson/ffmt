use crate::case_norm::normalize_case;
use crate::classifier::{classify, LineKind};
use crate::config::{Config, KeywordCase};
use crate::keyword_norm::normalize_keywords;
use crate::reader::read_logical_lines;
use crate::scope::ScopeTracker;
use crate::whitespace::normalize_whitespace;

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
                if consecutive_blanks > 2 {
                    idx += 1;
                    continue;
                }
            }
            let _ = tracker.process(kind);
            output_lines.push(String::new());
            idx += 1;
            continue;
        }

        consecutive_blanks = 0;
        skip_next_blank = false;

        // --- Scope tracking (always runs) ---
        let mut depth = tracker.process(kind);

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
                    if content.starts_with("!!") {
                        output_lines.push(trimmed.to_string());
                    } else {
                        output_lines.push(apply_indent(content, depth, config.indent_width));
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
                    if ll.raw_lines.len() == 1 || raw_idx == 0 {
                        // First line (or single line): full normalization
                        let orig_indent = leading_spaces(trimmed);
                        let mut processed = process_line(trimmed, config);
                        // Ensure space before trailing & on continuation lines,
                        // but NOT for Fypp !& continuations
                        if ll.raw_lines.len() > 1 && processed.trim_end().ends_with('&') {
                            let t = processed.trim_end();
                            if t.len() >= 2
                                && t.as_bytes()[t.len() - 2] != b' '
                                && t.as_bytes()[t.len() - 2] != b'!'
                            {
                                let pos = t.len() - 1;
                                processed = format!("{} &", &t[..pos]);
                            }
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

        idx += 1;
    }

    // Strip trailing blank lines
    while output_lines.last().is_some_and(|l| l.is_empty()) {
        output_lines.pop();
    }

    let mut result = output_lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Process a single Fortran line: keyword norm → whitespace norm → case norm.
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

    // Apply full normalization to the content portion
    let mut normalized = main_content.to_string();

    if config.normalize_keywords {
        normalized = normalize_keywords(&normalized);
    }

    normalized = normalize_whitespace(&normalized, &config.whitespace);

    match config.keyword_case {
        KeywordCase::Lower => normalized = normalize_case(&normalized),
        KeywordCase::Upper => normalized = normalize_case_upper(&normalized),
        KeywordCase::Preserve => {}
    }

    let normalized = normalized.trim();

    // Re-add trailing &
    let with_trailing = if trailing.is_empty() {
        normalized.to_string()
    } else {
        format!("{} &", normalized)
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
            // Comment starts — stop looking
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
