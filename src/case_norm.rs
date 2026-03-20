/// Normalize case of Fortran keywords in a source line.
///
/// Rules:
/// - Fypp directive lines (`$:`, `@:`, `#:`, `#!`) are returned unchanged.
/// - String literals (`'...'`, `"..."`), Fypp inline expressions (`${...}$`,
///   `@{...}@`), and trailing comments (`! ...`) are opaque — never modified.
/// - Fortran keywords and dot operators (`.AND.`, `.TRUE.`, etc.) are
///   lowercased in non-opaque regions.
/// - All other text (identifiers, numbers, punctuation) is preserved.
pub fn normalize_case(line: &str) -> String {
    let trimmed = line.trim_start();

    // If the line is a Fypp directive, return unchanged.
    if trimmed.starts_with("$:")
        || trimmed.starts_with("@:")
        || trimmed.starts_with("#:")
        || trimmed.starts_with("#!")
    {
        return line.to_string();
    }

    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        let ch = bytes[i];

        // --- String literals: pass through unchanged ---
        if ch == b'\'' || ch == b'"' {
            let quote = ch;
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == quote {
                    i += 1;
                    // Doubled quote is an escape sequence, continue
                    if i < len && bytes[i] == quote {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            out.push_str(&line[start..i]);
            continue;
        }

        // --- Fypp inline expressions: ${...}$ ---
        if ch == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
            let start = i;
            i += 2;
            let mut depth = 1usize;
            while i < len && depth > 0 {
                if bytes[i] == b'{' {
                    depth += 1;
                } else if bytes[i] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        // consume trailing $
                        if i < len && bytes[i] == b'$' {
                            i += 1;
                        }
                        break;
                    }
                }
                i += 1;
            }
            out.push_str(&line[start..i]);
            continue;
        }

        // --- Fypp inline expressions: @{...}@ ---
        if ch == b'@' && i + 1 < len && bytes[i + 1] == b'{' {
            let start = i;
            i += 2;
            let mut depth = 1usize;
            while i < len && depth > 0 {
                if bytes[i] == b'{' {
                    depth += 1;
                } else if bytes[i] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        // consume trailing @
                        if i < len && bytes[i] == b'@' {
                            i += 1;
                        }
                        break;
                    }
                }
                i += 1;
            }
            out.push_str(&line[start..i]);
            continue;
        }

        // --- Inline comment: everything from ! to end-of-line is opaque ---
        if ch == b'!' {
            out.push_str(&line[i..]);
            break;
        }

        // --- Dot operators: .TRUE., .AND., etc. ---
        if ch == b'.' && i + 2 < len {
            if let Some((dot_end, _)) = crate::match_dot_token(bytes, i) {
                // Lowercase the entire dot keyword/operator token
                let op_str = &line[i..dot_end];
                out.push_str(&op_str.to_ascii_lowercase());
                i = dot_end;
                continue;
            }
        }

        // --- Word boundary: check for keyword ---
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &line[start..i];
            if is_keyword(word) {
                out.push_str(&word.to_ascii_lowercase());
            } else {
                out.push_str(word);
            }
            continue;
        }

        // --- Everything else: pass through unchanged ---
        out.push(ch as char);
        i += 1;
    }

    out
}

/// Return true if `word` is a Fortran keyword (case-insensitive).
fn is_keyword(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    KEYWORDS.binary_search(&lower.as_str()).is_ok()
}

/// All Fortran keywords that should be lowercased.
/// Must remain in lexicographic (ASCII) order for binary search.
const KEYWORDS: &[&str] = &[
    "abstract",
    "allocatable",
    "allocate",
    "associate",
    "asynchronous",
    "backspace",
    "bind",
    "block",
    "call",
    "case",
    "change",
    "character",
    "class",
    "close",
    "codimension",
    "common",
    "complex",
    "contains",
    "contiguous",
    "continue",
    "critical",
    "cycle",
    "data",
    "deallocate",
    "default",
    "deferred",
    "dimension",
    "do",
    "double",
    "elemental",
    "else",
    "elsewhere",
    "end",
    "endfile",
    "entry",
    "enum",
    "enumerator",
    "equivalence",
    "error",
    "event",
    "exit",
    "extends",
    "external",
    "fail",
    "final",
    "flush",
    "forall",
    "form",
    "format",
    "function",
    "generic",
    "go",
    "if",
    "image",
    "implicit",
    "import",
    "impure",
    "in",
    "inout",
    "inquire",
    "integer",
    "intent",
    "interface",
    "intrinsic",
    "lock",
    "logical",
    "module",
    "namelist",
    "non_overridable",
    "none",
    "nopass",
    "nullify",
    "only",
    "open",
    "optional",
    "out",
    "parameter",
    "pass",
    "pointer",
    "precision",
    "print",
    "private",
    "procedure",
    "program",
    "protected",
    "public",
    "pure",
    "rank",
    "read",
    "real",
    "recursive",
    "result",
    "return",
    "rewind",
    "save",
    "select",
    "sequence",
    "stop",
    "submodule",
    "subroutine",
    "sync",
    "target",
    "team",
    "then",
    "to",
    "type",
    "unlock",
    "use",
    "value",
    "volatile",
    "wait",
    "where",
    "while",
    "write",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_list_sorted() {
        // Ensure KEYWORDS is sorted so binary search would work correctly
        for i in 1..KEYWORDS.len() {
            assert!(
                KEYWORDS[i - 1] < KEYWORDS[i],
                "KEYWORDS not sorted at index {}: '{}' >= '{}'",
                i,
                KEYWORDS[i - 1],
                KEYWORDS[i]
            );
        }
    }
}
