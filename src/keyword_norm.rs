use regex::Regex;
use std::sync::OnceLock;

/// Normalize compound Fortran keywords by inserting a space.
///
/// Examples:
///   `enddo` → `end do`
///   `endif` → `end if`
///   `endsubroutine` → `end subroutine`
///   `elseif` → `else if`
///   `selectcase` → `select case`
///   `endmodule` → `end module`
///
/// Only operates outside strings, comments, and Fypp regions.
/// Preserves the original case of the keyword parts.
pub fn normalize_keywords(line: &str) -> String {
    let trimmed = line.trim_start();

    // Skip Fypp directives, preprocessor, and comments entirely
    if trimmed.starts_with("$:")
        || trimmed.starts_with("@:")
        || trimmed.starts_with("#")
        || trimmed.starts_with("!")
    {
        return line.to_string();
    }

    // Find the code portion (before any trailing comment)
    let (code_part, comment_part) = split_at_comment(line);

    let normalized = normalize_code_keywords(code_part);

    if comment_part.is_empty() {
        normalized
    } else {
        format!("{}{}", normalized, comment_part)
    }
}

/// Split a line at the first `!` that starts a trailing comment (outside strings).
fn split_at_comment(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut quote_char = b' ';

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == quote_char {
                if i + 1 < bytes.len() && bytes[i + 1] == quote_char {
                    continue; // doubled quote
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
            return (&line[..i], &line[i..]);
        }
    }
    (line, "")
}

/// Apply keyword normalization to the code portion of a line.
/// Protects string literals from modification.
fn normalize_code_keywords(code: &str) -> String {
    static RE: OnceLock<Vec<(Regex, &str)>> = OnceLock::new();
    let patterns = RE.get_or_init(|| {
        vec![
            // end* compounds — capture both parts to preserve case:
            // "ENDDO" → "END DO", "EndDo" → "End Do", "enddo" → "end do"
            (
                Regex::new(r"(?i)\b(end)(do|if|select|subroutine|function|module|submodule|program|interface|type|block|associate|where|forall|enum|critical|team)\b").unwrap(),
                "$1 $2",
            ),
            // else if — normalize spacing (but NOT elsewhere, which is a single keyword)
            (Regex::new(r"(?i)\b(else)\s*(if)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(select)\s*(case)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(select)\s*(type)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(select)\s*(rank)\b").unwrap(), "$1 $2"),
            // double precision
            (
                Regex::new(r"(?i)\b(double)\s*(precision)\b").unwrap(),
                "$1 $2",
            ),
            // error stop, change team, go to
            (Regex::new(r"(?i)\b(error)\s*(stop)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(change)\s*(team)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(go)\s*(to)\b").unwrap(), "$1 $2"),
            // coarray compounds
            (Regex::new(r"(?i)\b(sync)\s*(all)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(sync)\s*(images)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(sync)\s*(memory)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(event)\s*(post)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(event)\s*(wait)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(fail)\s*(image)\b").unwrap(), "$1 $2"),
            (Regex::new(r"(?i)\b(form)\s*(team)\b").unwrap(), "$1 $2"),
        ]
    });

    // Process only non-string portions of the code.
    let bytes = code.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == quote {
                    i += 1;
                    if i < len && bytes[i] == quote {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            result.push_str(&code[start..i]);
        } else {
            // Find extent of non-string text
            let start = i;
            while i < len && bytes[i] != b'\'' && bytes[i] != b'"' {
                i += 1;
            }
            let mut segment = code[start..i].to_string();
            for (re, replacement) in patterns {
                segment = re.replace_all(&segment, *replacement).to_string();
            }
            result.push_str(&segment);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enddo() {
        assert_eq!(normalize_keywords("enddo"), "end do");
    }

    #[test]
    fn test_endif() {
        assert_eq!(normalize_keywords("endif"), "end if");
    }

    #[test]
    fn test_end_subroutine_already_spaced() {
        assert_eq!(
            normalize_keywords("end subroutine foo"),
            "end subroutine foo"
        );
    }

    #[test]
    fn test_endsubroutine() {
        assert_eq!(
            normalize_keywords("endsubroutine foo"),
            "end subroutine foo"
        );
    }

    #[test]
    fn test_elseif() {
        assert_eq!(
            normalize_keywords("elseif (x > 0) then"),
            "else if (x > 0) then"
        );
    }

    #[test]
    fn test_selectcase() {
        assert_eq!(normalize_keywords("selectcase (x)"), "select case (x)");
    }

    #[test]
    fn test_preserves_strings() {
        assert_eq!(
            normalize_keywords("x = 'endif is a keyword'"),
            "x = 'endif is a keyword'"
        );
    }

    #[test]
    fn test_preserves_comments() {
        assert_eq!(
            normalize_keywords("end do ! endif here"),
            "end do ! endif here"
        );
    }

    #[test]
    fn test_fypp_untouched() {
        assert_eq!(
            normalize_keywords("$:GPU_PARALLEL_LOOP(collapse=3)"),
            "$:GPU_PARALLEL_LOOP(collapse=3)"
        );
    }

    #[test]
    fn test_endmodule() {
        assert_eq!(normalize_keywords("endmodule m_foo"), "end module m_foo");
    }

    #[test]
    fn test_mixed_case() {
        assert_eq!(normalize_keywords("EndDo"), "End Do");
    }

    #[test]
    fn test_endblock() {
        assert_eq!(normalize_keywords("endblock"), "end block");
    }

    #[test]
    fn test_no_false_positive_on_identifier() {
        // "endorphin" should not become "end orphin"
        assert_eq!(normalize_keywords("x = endorphin"), "x = endorphin");
    }

    #[test]
    fn test_double_precision() {
        assert_eq!(
            normalize_keywords("doubleprecision :: x"),
            "double precision :: x"
        );
    }
}
