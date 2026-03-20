/// A logical Fortran line: one or more raw lines joined at `&` continuations.
#[derive(Debug, Clone)]
pub struct LogicalLine {
    pub joined: String,
    pub raw_lines: Vec<String>,
    pub line_number: usize,
}

/// Scan a line and return the index of a trailing `&` continuation character,
/// ignoring `&` inside string literals. Returns `None` if the line does not
/// end with a continuation `&`.
///
/// A trailing `&` means: after stripping any trailing comment (`! ...` outside
/// strings), the last non-whitespace character is `&`.
fn find_continuation_amp(line: &str) -> Option<usize> {
    // Walk the line char-by-char tracking string state
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();

    let mut in_string = false;
    let mut string_delim = ' ';
    let mut i = 0;
    // Track the byte position of the last `&` outside a string and outside a comment
    let mut last_amp_byte: Option<usize> = None;
    let mut byte_pos = 0usize;

    while i < n {
        let ch = chars[i];

        if in_string {
            if ch == string_delim {
                // Check for doubled quote (escape)
                if i + 1 < n && chars[i + 1] == string_delim {
                    byte_pos += ch.len_utf8();
                    i += 1;
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                    continue;
                }
                in_string = false;
            }
            byte_pos += ch.len_utf8();
            i += 1;
            continue;
        }

        // Outside string
        if ch == '\'' || ch == '"' {
            in_string = true;
            string_delim = ch;
            last_amp_byte = None; // reset — we're now entering a string
            byte_pos += ch.len_utf8();
            i += 1;
            continue;
        }

        if ch == '!' {
            // Start of comment — stop scanning
            break;
        }

        if ch == '&' {
            last_amp_byte = Some(byte_pos);
        } else if !ch.is_whitespace() {
            // Non-whitespace, non-& outside string → reset
            last_amp_byte = None;
        }

        byte_pos += ch.len_utf8();
        i += 1;
    }

    last_amp_byte
}

/// Check if a line is a Fypp continuation: ends with `!&` (possibly after
/// whitespace). The `!&` must be outside string literals.
fn is_fypp_continuation(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.ends_with("!&")
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

    while i < raw_count {
        let raw_line = raw[i];
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

        // Check for Fypp continuation (`!&` at end of line)
        let fypp_cont = is_fypp_continuation(raw_line);

        // Check for Fortran continuation (`&` at end of line outside strings)
        let fort_cont = if !fypp_cont {
            find_continuation_amp(raw_line).is_some()
        } else {
            false
        };

        if !fypp_cont && !fort_cont {
            // Simple line, no continuation
            result.push(LogicalLine {
                joined: raw_line.to_string(),
                raw_lines: vec![raw_line.to_string()],
                line_number,
            });
            i += 1;
            continue;
        }

        // Continuation: gather lines
        let mut raw_lines_acc: Vec<String> = Vec::new();
        let mut joined_parts: Vec<String> = Vec::new();

        // Process first line: strip trailing `&` (Fortran) or `!&` (Fypp)
        let first_content = if fort_cont {
            // Remove the trailing `&` (and any whitespace after it, up to comment)
            // find_continuation_amp returns byte offset of the `&`
            let amp_pos = find_continuation_amp(raw_line).unwrap();
            raw_line[..amp_pos].to_string()
        } else {
            // Fypp: remove the trailing `!&`
            let end = raw_line.trim_end().len() - 2; // strip `!&`
            raw_line[..end].to_string()
        };

        raw_lines_acc.push(raw_line.to_string());
        joined_parts.push(first_content);
        i += 1;

        // Collect continuation lines
        loop {
            if i >= raw_count {
                break;
            }

            let cont_line = raw[i];

            // Blank lines are NEVER joined — stop continuation
            if cont_line.trim().is_empty() {
                break;
            }

            raw_lines_acc.push(cont_line.to_string());

            // Strip leading `&` if present
            let stripped = strip_leading_amp(cont_line);

            // Check if this continuation line itself continues further
            let this_fypp = is_fypp_continuation(stripped);
            let this_fort = if !this_fypp {
                find_continuation_amp(stripped).is_some()
            } else {
                false
            };

            let content = if this_fort {
                let amp_pos = find_continuation_amp(stripped).unwrap();
                stripped[..amp_pos].to_string()
            } else if this_fypp {
                let end = stripped.trim_end().len() - 2;
                stripped[..end].to_string()
            } else {
                stripped.to_string()
            };

            joined_parts.push(content);
            i += 1;

            if !this_fort && !this_fypp {
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
