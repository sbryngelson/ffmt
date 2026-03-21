//! Post-processing pass: align `::` in consecutive declaration lines.
//!
//! Groups consecutive lines that:
//! - Have the same indentation level
//! - Are declaration lines (start with a type keyword and contain `::`)
//! - Are not separated by blank lines, comments, or non-declaration lines
//!
//! Within each group, pads the space before `::` so all `::` appear at the same column.

/// Check if a trimmed line (no leading whitespace) is a Fortran declaration line.
/// A declaration line starts with a type keyword and contains `::`.
fn is_declaration_line(trimmed: &str) -> bool {
    if !trimmed.contains("::") {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();

    // Must start with a type keyword
    let type_keywords = [
        "integer",
        "real",
        "complex",
        "character",
        "logical",
        "type(",
        "class(",
        "double precision",
        "double",
    ];

    for kw in &type_keywords {
        if lower.starts_with(kw) {
            return true;
        }
    }

    false
}

/// Get the leading indentation (spaces) of a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Find the position of `::` in a line, accounting for strings.
/// Returns the byte offset of the first `::` outside of string literals.
fn find_double_colon(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut quote_char = b' ';
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        if in_string {
            if bytes[i] == quote_char {
                if i + 1 < len && bytes[i + 1] == quote_char {
                    i += 2; // escaped quote
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            in_string = true;
            quote_char = bytes[i];
            i += 1;
            continue;
        }
        if bytes[i] == b'!' {
            break; // comment
        }
        if bytes[i] == b':' && i + 1 < len && bytes[i + 1] == b':' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Check if a line is inside a Fypp block (starts with `#:` or `$:` after indent,
/// or is a Fypp continuation line starting with `&`).
fn is_fypp_line(trimmed: &str) -> bool {
    trimmed.starts_with("#:") || trimmed.starts_with("$:") || trimmed.starts_with("@:")
}

/// Find the position of `!<` inline Doxygen comment in a line, outside strings.
/// Returns the byte offset of `!<`, or None if not present.
fn find_inline_doxygen(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut quote_char = b' ';
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        if in_string {
            if bytes[i] == quote_char {
                if i + 1 < len && bytes[i + 1] == quote_char {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            in_string = true;
            quote_char = bytes[i];
            i += 1;
            continue;
        }
        if bytes[i] == b'!' && i + 1 < len && bytes[i + 1] == b'<' {
            return Some(i);
        }
        if bytes[i] == b'!' {
            break; // regular comment, not !<
        }
        i += 1;
    }
    None
}

/// Align `!<` inline Doxygen comments in a group of declaration lines.
/// Lines with `!<` get padded so all `!<` start at the same column.
/// Lines without `!<` are left unchanged.
fn align_inline_comments(lines: &mut [String]) {
    // Count how many lines have !< comments
    let comment_count = lines.iter().filter(|l| find_inline_doxygen(l).is_some()).count();
    if comment_count < 2 {
        return;
    }

    // Find the max code-content length (before !<) across lines that have !<
    let mut max_code_len: usize = 0;
    for line in lines.iter() {
        if let Some(pos) = find_inline_doxygen(line) {
            let before = &line[..pos];
            let trimmed = before.trim_end();
            if trimmed.len() > max_code_len {
                max_code_len = trimmed.len();
            }
        }
    }

    // Re-align each line that has !<
    for line in lines.iter_mut() {
        if let Some(pos) = find_inline_doxygen(line) {
            let before = &line[..pos];
            let comment = &line[pos..]; // includes "!< ..."
            let trimmed_before = before.trim_end();
            let padding = max_code_len - trimmed_before.len();
            *line = format!("{}{} {}", trimmed_before, " ".repeat(padding), comment);
        }
    }
}

/// Align `::` in consecutive declaration lines.
/// If `compact` is true, also remove blank lines between declarations in a group.
/// If `align_comments` is true, also align `!<` inline comments within groups.
pub fn align_declarations(lines: &[String], compact: bool, align_comments: bool) -> Vec<String> {
    let mut result = lines.to_vec();
    let mut i = 0;

    while i < result.len() {
        // Try to start a group of consecutive declaration lines
        let trimmed = result[i].trim_start();

        if trimmed.is_empty() || is_fypp_line(trimmed) || !is_declaration_line(trimmed) {
            i += 1;
            continue;
        }

        let group_indent = indent_of(&result[i]);

        // Find the extent of this group.
        // When compact mode is enabled, skip over single blank lines
        // to find larger declaration groups.
        let group_start = i;
        let mut group_end = i;

        while group_end < result.len() {
            let line = &result[group_end];
            let t = line.trim_start();

            if t.is_empty() {
                // Blank line: if compact, look ahead for more declarations
                if compact && group_end + 1 < result.len() {
                    let next_t = result[group_end + 1].trim_start();
                    if is_declaration_line(next_t) && indent_of(&result[group_end + 1]) == group_indent {
                        group_end += 1; // skip blank, continue group
                        continue;
                    }
                }
                break;
            }
            if is_fypp_line(t) {
                break;
            }
            if indent_of(line) != group_indent {
                break;
            }
            if !is_declaration_line(t) {
                break;
            }
            group_end += 1;
        }

        // In compact mode, remove blank lines within the group
        if compact {
            let mut compacted: Vec<String> = Vec::new();
            for line in &result[group_start..group_end] {
                if !line.trim().is_empty() {
                    compacted.push(line.clone());
                }
            }
            // Replace the group range with the compacted version
            let removed = group_end - group_start - compacted.len();
            result.splice(group_start..group_end, compacted);
            group_end -= removed;
            // Update len since we may have removed lines
        }

        let group_size = group_end - group_start;

        if group_size >= 2 {
            // Find the maximum position of `::` relative to line start

            let mut max_pre_colon_len = 0;

            for line in &result[group_start..group_end] {
                if let Some(pos) = find_double_colon(line) {
                    // pos is absolute position in the line
                    // Trim trailing spaces before ::
                    let before_colon = &line[..pos];
                    let trimmed_before = before_colon.trim_end();
                    // Length of content before :: (including indent)
                    let content_len = trimmed_before.len();
                    if content_len > max_pre_colon_len {
                        max_pre_colon_len = content_len;
                    }
                }
            }

            // Now re-align each line's ::
            for line in &mut result[group_start..group_end] {
                if let Some(pos) = find_double_colon(line) {
                    let before_colon = &line[..pos];
                    let after_colon = &line[pos + 2..]; // after `::`
                    let trimmed_before = before_colon.trim_end();
                    let padding = max_pre_colon_len - trimmed_before.len();

                    // Rebuild: trimmed_before + padding spaces + " :: " + after content
                    let after_trimmed = after_colon.trim_start();
                    let new_line = format!(
                        "{}{} :: {}",
                        trimmed_before,
                        " ".repeat(padding),
                        after_trimmed
                    );
                    *line = new_line;
                }
            }

            // Align !< inline comments within the group
            if align_comments {
                align_inline_comments(&mut result[group_start..group_end]);
            }
        }

        i = group_end;
    }

    // Remove consecutive duplicate blank lines that may have been created
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_alignment() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "    real(wp) :: y".into(),
            "    character(len=100) :: z".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    integer            :: x");
        assert_eq!(result[1], "    real(wp)           :: y");
        assert_eq!(result[2], "    character(len=100) :: z");
    }

    #[test]
    fn test_no_alignment_single_line() {
        let input: Vec<String> = vec!["    integer :: x".into()];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    integer :: x");
    }

    #[test]
    fn test_blank_line_breaks_group() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "".into(),
            "    real(wp) :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        // No alignment since blank line separates them
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[1], "");
        assert_eq!(result[2], "    real(wp) :: y");
    }

    #[test]
    fn test_different_indent_breaks_group() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "        real(wp) :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        // Different indentation, no alignment
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[1], "        real(wp) :: y");
    }

    #[test]
    fn test_non_declaration_breaks_group() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "    x = 1".into(),
            "    real(wp) :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[1], "    x = 1");
        assert_eq!(result[2], "    real(wp) :: y");
    }

    #[test]
    fn test_attributes() {
        let input: Vec<String> = vec![
            "    real(wp), allocatable, dimension(:,:,:) :: var".into(),
            "    integer :: n".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(
            result[0],
            "    real(wp), allocatable, dimension(:,:,:) :: var"
        );
        assert_eq!(
            result[1],
            "    integer                                 :: n"
        );
    }

    #[test]
    fn test_already_aligned() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "    real    :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[1], "    real    :: y");
    }

    #[test]
    fn test_type_and_class() {
        let input: Vec<String> = vec![
            "    type(scalar_field) :: sf".into(),
            "    logical :: flag".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    type(scalar_field) :: sf");
        assert_eq!(result[1], "    logical            :: flag");
    }

    #[test]
    fn test_fypp_line_not_grouped() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "    @:ALLOCATE(x)".into(),
            "    real :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        // Fypp line breaks the group
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[2], "    real :: y");
    }

    #[test]
    fn test_preserves_after_double_colon() {
        let input: Vec<String> = vec![
            "    integer :: x = 0".into(),
            "    real(wp) :: y = 1.0_wp".into(),
        ];
        let result = align_declarations(&input, false, true);
        assert_eq!(result[0], "    integer  :: x = 0");
        assert_eq!(result[1], "    real(wp) :: y = 1.0_wp");
    }

    #[test]
    fn test_private_public_not_declaration() {
        // `private; public :: s_foo` should NOT be treated as a declaration
        let input: Vec<String> = vec![
            "    private; public :: s_foo".into(),
            "    integer :: x".into(),
        ];
        let result = align_declarations(&input, false, true);
        // "private" doesn't start with a type keyword, so no grouping
        assert_eq!(result[0], "    private; public :: s_foo");
        assert_eq!(result[1], "    integer :: x");
    }

    #[test]
    fn test_comment_line_breaks_group() {
        let input: Vec<String> = vec![
            "    integer :: x".into(),
            "    ! a comment".into(),
            "    real :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        // Comment is not a declaration, breaks the group
        assert_eq!(result[0], "    integer :: x");
        assert_eq!(result[2], "    real :: y");
    }

    #[test]
    fn test_inline_doxygen_alignment() {
        let input: Vec<String> = vec![
            "    character, parameter :: dflt_char = ' ' !< Default string value".into(),
            "    real(wp), parameter  :: dflt_real = -1.e6_wp                !< Default real value".into(),
            "    real(wp), parameter  :: sgm_eps = 1.e-16_wp               !< Segmentation tolerance".into(),
        ];
        let result = align_declarations(&input, false, true);
        // All !< should be at the same column
        let pos0 = result[0].find("!<").unwrap();
        let pos1 = result[1].find("!<").unwrap();
        let pos2 = result[2].find("!<").unwrap();
        assert_eq!(pos0, pos1);
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn test_inline_doxygen_mixed_with_no_comment() {
        let input: Vec<String> = vec![
            "    integer, parameter :: x = 1 !< First".into(),
            "    integer, parameter :: num_patches_max = 1000".into(),
            "    integer, parameter :: y = 2 !< Third".into(),
        ];
        let result = align_declarations(&input, false, true);
        // Lines with !< should align; line without !< is unchanged (except :: alignment)
        let pos0 = result[0].find("!<").unwrap();
        let pos2 = result[2].find("!<").unwrap();
        assert_eq!(pos0, pos2);
        // Line without !< should not have !< added
        assert!(result[1].find("!<").is_none());
    }

    #[test]
    fn test_inline_doxygen_single_comment_no_alignment() {
        let input: Vec<String> = vec![
            "    integer :: x !< Only one comment".into(),
            "    integer :: y".into(),
        ];
        let result = align_declarations(&input, false, true);
        // Only 1 line has !<, so no alignment needed — just :: alignment
        assert!(result[0].contains("!<"));
    }
}
