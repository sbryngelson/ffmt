/// A logical Fortran line: one or more raw lines joined at `&` continuations.
#[derive(Debug, Clone)]
pub struct LogicalLine {
    pub joined: String,
    pub raw_lines: Vec<String>,
    pub line_number: usize,
}

/// Scan a line and return the index of a trailing `&` continuation character.
/// Returns `None` if the line does not end with a continuation `&`.
///
/// In Fortran free-form source, `&` as the last non-whitespace character
/// before any comment is ALWAYS a continuation marker, even inside strings
/// that span multiple lines. String state is only tracked to correctly
/// identify where comments start (`!` inside a string is not a comment).
fn find_continuation_amp(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();

    // Step 1: Find where the comment starts (! outside strings)
    let mut in_string = false;
    let mut string_delim = ' ';
    let mut comment_start = n; // index into chars
    let mut i = 0;
    while i < n {
        let ch = chars[i];
        if in_string {
            if ch == string_delim {
                if i + 1 < n && chars[i + 1] == string_delim {
                    i += 2; // skip doubled quote escape
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            in_string = true;
            string_delim = ch;
            i += 1;
            continue;
        }
        if ch == '!' {
            comment_start = i;
            break;
        }
        i += 1;
    }

    // Step 2: Find the last non-whitespace char before the comment.
    // If it's `&`, that's the continuation marker.
    let mut last_amp_byte: Option<usize> = None;
    let mut byte_pos = 0usize;
    for ch in chars.iter().take(comment_start) {
        let ch = *ch;
        if ch == '&' {
            last_amp_byte = Some(byte_pos);
        } else if !ch.is_whitespace() {
            last_amp_byte = None;
        }
        byte_pos += ch.len_utf8();
    }

    last_amp_byte
}

/// Strip the leading `&` from a continuation line if present (after optional
/// leading whitespace).
fn strip_leading_amp(line: &str) -> &str {
    let trimmed_start = line.trim_start();
    if trimmed_start.starts_with('&') {
        // Return everything after the `&` — keep remaining whitespace
        let amp_byte = line.len() - trimmed_start.len();
        &line[amp_byte + 1..]
    } else {
        line
    }
}

/// Read source text into logical lines.
/// Strip trailing `!&` (with optional surrounding whitespace) from a line.
/// `!&` is a no-op comment used in some codebases to suppress compiler warnings
/// about line continuations. It carries no semantic meaning and can confuse
/// compilers like Cray ftn that strictly enforce continuation rules.
fn strip_trailing_bang_amp(line: &str) -> String {
    let trimmed = line.trim_end();
    if !trimmed.ends_with("!&") {
        return line.to_string();
    }
    // Verify that `!&` is a comment, not inside a string.
    // Find the `!` that starts the `!&` and check it's not in a string.
    let bang_pos = trimmed.len() - 2;
    let before = &trimmed[..bang_pos];
    // Quick check: if `!&` is preceded by something that looks like code or
    // whitespace (not inside a string), strip it.
    let mut in_string = false;
    let mut delim = ' ';
    for ch in before.chars() {
        if in_string {
            if ch == delim { in_string = false; }
        } else if ch == '\'' || ch == '"' {
            in_string = true;
            delim = ch;
        }
    }
    // If we're inside a string at the `!` position, don't strip
    if in_string {
        return line.to_string();
    }
    // Strip the `!&` — return the line up to and including any content before it
    before.trim_end().to_string()
}

pub fn read_logical_lines(source: &str) -> Vec<LogicalLine> {
    // Split into raw lines. We preserve the newline character for raw_lines but
    // work with trimmed content for logic.
    let raw: Vec<&str> = source.split('\n').collect();

    // If source ends with \n, the last element will be empty — skip it
    let raw_count = if source.ends_with('\n') && !raw.is_empty() {
        raw.len() - 1
    } else {
        raw.len()
    };

    let mut result: Vec<LogicalLine> = Vec::new();
    let mut i = 0usize;

    // Pre-process: strip trailing `!&` from all lines
    let cleaned: Vec<String> = raw[..raw_count].iter().map(|l| strip_trailing_bang_amp(l)).collect();

    while i < raw_count {
        let raw_line = cleaned[i].as_str();
        let line_number = i + 1; // 1-based

        // Blank line — always its own LogicalLine
        if raw_line.trim().is_empty() {
            result.push(LogicalLine {
                joined: raw_line.to_string(),
                raw_lines: vec![raw_line.to_string()],
                line_number,
            });
            i += 1;
            continue;
        }

        // Full-line comment (`! ...`) — its own LogicalLine (but not directives).
        // We detect a comment line as one that, after stripping leading whitespace,
        // starts with `!`. We still emit it as its own line (directives like `!$acc`
        // also satisfy this and are emitted as their own line — both are correct).
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with('!') {
            result.push(LogicalLine {
                joined: raw_line.to_string(),
                raw_lines: vec![raw_line.to_string()],
                line_number,
            });
            i += 1;
            continue;
        }

        // Check for Fortran continuation (`&` at end of line outside strings).
        // Note: Fypp `!&` continuations are NOT joined — they are Fypp-level
        // continuations handled by the Fypp preprocessor, not the formatter.
        // The `!` starts a comment, so `find_continuation_amp` won't see the `&`.
        let fort_cont = find_continuation_amp(raw_line).is_some();

        if !fort_cont {
            // Simple line, no continuation
            result.push(LogicalLine {
                joined: raw_line.to_string(),
                raw_lines: vec![raw_line.to_string()],
                line_number,
            });
            i += 1;
            continue;
        }

        // Fortran continuation: gather lines joined by `&`
        let mut raw_lines_acc: Vec<String> = Vec::new();
        let mut joined_parts: Vec<String> = Vec::new();

        // Process first line: strip trailing `&` and trim trailing space
        let amp_pos = find_continuation_amp(raw_line).unwrap();
        let first_content = raw_line[..amp_pos].trim_end().to_string();

        raw_lines_acc.push(raw_line.to_string());
        joined_parts.push(first_content);
        i += 1;

        // Collect continuation lines
        loop {
            if i >= raw_count {
                break;
            }

            let cont_line = cleaned[i].as_str();

            // Blank lines are NEVER joined — stop continuation
            if cont_line.trim().is_empty() {
                break;
            }

            // Preprocessor directives (#ifdef, #ifndef, #else, #endif, #if, #define)
            // must NOT be joined into continuation lines — they need to stay separate
            if cont_line.trim_start().starts_with('#') {
                break;
            }

            raw_lines_acc.push(cont_line.to_string());

            // Strip leading `&` if present
            let stripped = strip_leading_amp(cont_line);

            // Check if this continuation line itself continues further
            let this_fort = find_continuation_amp(stripped).is_some();

            let content = if this_fort {
                let amp_pos = find_continuation_amp(stripped).unwrap();
                stripped[..amp_pos].trim_end().to_string()
            } else {
                stripped.to_string()
            };

            // Add space separator to prevent tokens from merging
            joined_parts.push(format!(" {}", content.trim_start()));
            i += 1;

            if !this_fort {
                break;
            }
        }

        let joined = joined_parts.join("");
        result.push(LogicalLine {
            joined,
            raw_lines: raw_lines_acc,
            line_number,
        });
    }

    result
}
