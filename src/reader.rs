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
/// Strip leading `& !` from continuation-comment lines.
/// Lines like `& ! some comment text` are continuation lines that contain
/// only a comment after the `&`. Cray ftn (ftn-71) rejects these as invalid.
/// Convert them to plain comment lines by removing the leading `&`.
fn strip_amp_comment_continuation(line: &str) -> String {
    let trimmed = line.trim_start();
    // Match: leading `&` followed by optional spaces then `!` (but not `!$` for OpenACC/OMP)
    if let Some(stripped) = trimmed.strip_prefix('&') {
        let after_amp = stripped.trim_start();
        if after_amp.starts_with('!') && !after_amp.starts_with("!$") {
            let indent = line.len() - line.trim_start().len();
            return format!("{}{}", " ".repeat(indent), after_amp);
        }
    }
    line.to_string()
}

/// Strip trailing `!&` (with optional surrounding whitespace) from a line.
/// `!&` is a no-op comment used in some codebases to suppress compiler warnings
/// about line continuations. It carries no semantic meaning and can confuse
/// compilers like Cray ftn that strictly enforce continuation rules.
fn strip_trailing_bang_amp(line: &str) -> String {
    let mut trimmed = line.trim_end();
    // Strip all trailing `!&` (may be repeated, e.g., `!&!&`)
    while trimmed.ends_with("!&") {
        trimmed = trimmed[..trimmed.len() - 2].trim_end();
    }
    if trimmed.len() == line.trim_end().len() {
        return line.to_string();
    }
    // Verify that stripping is safe (not inside a string)
    let before = trimmed;
    // Quick check: if `!&` is preceded by something that looks like code or
    // whitespace (not inside a string), strip it.
    let mut in_string = false;
    let mut delim = ' ';
    for ch in before.chars() {
        if in_string {
            if ch == delim {
                in_string = false;
            }
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

    // Pre-process: strip trailing `!&` and convert `& ! comment` continuations
    let cleaned: Vec<String> = raw[..raw_count]
        .iter()
        .map(|l| strip_amp_comment_continuation(&strip_trailing_bang_amp(l)))
        .collect();

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

            // If this line is a comment (possibly after `& !` was cleaned to `!`),
            // check whether more continuation code follows. Cray ftn (ftn-71)
            // rejects `& ! comment` lines, so these are converted to plain
            // comments. If they appear mid-continuation (more `& code` follows),
            // we preserve them in the raw lines but skip them in the joined
            // content so the logical statement stays intact. If the comment is
            // the last line of the continuation, strip the trailing `&` from the
            // preceding code line and stop.
            let cont_trimmed = cont_line.trim_start();
            if cont_trimmed.starts_with('!') && !cont_trimmed.starts_with("!$") {
                // Peek ahead past any additional blank/comment lines to see
                // whether a continuation line (starting with `&`) follows.
                let mut peek = i + 1;
                while peek < raw_count {
                    let p = cleaned[peek].as_str().trim_start();
                    if p.is_empty() {
                        peek += 1;
                        continue;
                    }
                    if p.starts_with('!') && !p.starts_with("!$") {
                        peek += 1;
                        continue;
                    }
                    break;
                }
                let more_continuation =
                    peek < raw_count && cleaned[peek].as_str().trim_start().starts_with('&');

                if more_continuation {
                    // Mid-continuation comment: keep in raw lines so the
                    // original structure is preserved in the
                    // continuation_interrupted path, but don't add any content
                    // to joined_parts (comments carry no code).
                    raw_lines_acc.push(cont_line.to_string());
                    i += 1;
                    continue;
                } else {
                    // Terminal comment: strip trailing `&` from the last code
                    // line and stop — the comment becomes its own logical line.
                    if let Some(last_raw) = raw_lines_acc.last_mut() {
                        if let Some(pos) = find_continuation_amp(last_raw) {
                            *last_raw = last_raw[..pos].trim_end().to_string();
                        }
                    }
                    break;
                }
            }

            raw_lines_acc.push(cont_line.to_string());

            // Track whether this continuation line had a leading `&`.
            // Per the Fortran free-form standard, when `&` is the last
            // non-blank character on line N and line N+1 begins with `&`,
            // the statement continues immediately after the leading `&`
            // with NO implicit space inserted (the two tokens are glued).
            // When line N+1 does NOT begin with `&`, one implicit space
            // is inserted.  This matters for member access: `x &\n& %y`
            // must join as `x%y`, not `x %y`.
            let had_leading_amp = cont_line.trim_start().starts_with('&');

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

            let content_trimmed = content.trim_start();
            // Suppress the space separator when the continuation starts with
            // `%` (member accessor).  Adding a space there would produce
            // `a %b` which is a blank within a lexical token — invalid in
            // Fortran free-form (§6.3.2.2).
            let sep = if had_leading_amp && content_trimmed.starts_with('%') {
                ""
            } else {
                " "
            };
            joined_parts.push(format!("{}{}", sep, content_trimmed));
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
