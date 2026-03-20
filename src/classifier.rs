use regex::Regex;
use std::sync::OnceLock;

/// Classification of a logical line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    FortranBlockOpen,
    FortranBlockClose,
    FortranContinuation,
    FortranContains,
    FortranStatement,
    FyppBlockOpen,
    FyppBlockClose,
    FyppContinuation,
    FyppStatement,
    PreprocessorDirective,
    PreprocessorContinuation,
    PreprocessorClose,
    Directive,
    InlineFypp,
    Comment,
    Blank,
}

/// Helper to get or compile a case-insensitive regex, cached in a OnceLock.
macro_rules! re {
    ($lock:ident, $pat:expr) => {{
        static $lock: OnceLock<Regex> = OnceLock::new();
        $lock.get_or_init(|| Regex::new($pat).unwrap())
    }};
}

/// Classify a Fypp line (starts with `#:` or `#!`).
fn classify_fypp(trimmed: &str) -> LineKind {
    let re_block_open =
        re!(FYPP_OPEN, r"(?i)^#:\s*(if|for|def|call|block|mute)\b");
    let re_block_close =
        re!(FYPP_CLOSE, r"(?i)^#:\s*(endif|endfor|enddef|endcall|endblock|endmute)\b");
    let re_continuation =
        re!(FYPP_CONT, r"(?i)^#:\s*(elif|else)\b");

    if re_block_close.is_match(trimmed) {
        LineKind::FyppBlockClose
    } else if re_block_open.is_match(trimmed) {
        LineKind::FyppBlockOpen
    } else if re_continuation.is_match(trimmed) {
        LineKind::FyppContinuation
    } else {
        // #:include, #:set, #! comment, etc.
        LineKind::FyppStatement
    }
}

/// Classify a preprocessor line (starts with `#` but not `#:` or `#!`).
fn classify_preprocessor(trimmed: &str) -> LineKind {
    let re_close = re!(CPP_CLOSE, r"(?i)^#\s*endif\b");
    let re_cont = re!(CPP_CONT, r"(?i)^#\s*else\b");
    // Note: #else comes before this, so we won't match #elif as #else
    let re_elif = re!(CPP_ELIF, r"(?i)^#\s*elif\b");

    if re_close.is_match(trimmed) {
        LineKind::PreprocessorClose
    } else if re_elif.is_match(trimmed) {
        // #elif is a continuation, not a directive
        LineKind::PreprocessorContinuation
    } else if re_cont.is_match(trimmed) {
        LineKind::PreprocessorContinuation
    } else {
        // #ifdef, #ifndef, #if, #define, #undef, etc.
        LineKind::PreprocessorDirective
    }
}

/// Strip a trailing Fortran comment from a string.
/// E.g., "then! comment" -> "then", "x = 1 ! note" -> "x = 1"
/// Does not strip `!$` (directives) or `!` inside strings.
fn strip_trailing_comment(s: &str) -> String {
    // Simple: find first `!` not inside a string
    let bytes = s.as_bytes();
    let mut in_string = false;
    let mut string_delim = b' ';
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == string_delim {
                in_string = false;
            }
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_string = true;
            string_delim = b;
            continue;
        }
        if b == b'!' {
            return s[..i].trim_end().to_string();
        }
    }
    s.trim_end().to_string()
}

/// Find the matching closing paren for the paren at position `start`.
/// Returns the index of the closing paren, or None if not found.
fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    for (i, &byte) in bytes.iter().enumerate().skip(start) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Classify a Fortran line.
fn classify_fortran(trimmed: &str) -> LineKind {
    // Strip optional label prefix like "outer: "
    let line = {
        let re_label = re!(LABEL, r"(?i)^\w+\s*:\s*");
        // Only strip if it looks like a construct label (not a keyword: like "type:")
        // A label must start with a letter and be followed by ":"
        if let Some(m) = re_label.find(trimmed) {
            let candidate = &trimmed[..m.end()];
            // Make sure it's not a keyword like "type" followed by content
            let label_word = candidate.split(':').next().unwrap().trim();
            // If the label word is a Fortran keyword, don't strip it
            let keywords = [
                "if", "do", "else", "end", "type", "class", "select", "case",
                "where", "block", "associate", "critical", "enum", "forall",
                "module", "program", "subroutine", "function", "interface",
                "submodule", "use", "implicit", "contains", "rank",
                "elsewhere", "pure", "elemental", "impure", "recursive",
                "integer", "real", "character", "logical", "complex",
                "double", "change",
            ];
            if keywords.iter().any(|k| k.eq_ignore_ascii_case(label_word)) {
                trimmed
            } else {
                trimmed[m.end()..].trim()
            }
        } else {
            trimmed
        }
    };

    let lower = line.to_ascii_lowercase();

    // --- Block closers (check first since "end" prefix is distinctive) ---
    let re_end_block = re!(
        END_BLOCK,
        r"(?i)^end\s*(if|do|select|subroutine|function|module|submodule|program|interface|type|block|associate|where|forall|enum|critical|team)\b"
    );
    if re_end_block.is_match(line) {
        return LineKind::FortranBlockClose;
    }
    // Bare "end" (possibly followed by comment)
    let re_bare_end = re!(BARE_END, r"(?i)^end\s*(!.*)?$");
    if re_bare_end.is_match(line) {
        return LineKind::FortranBlockClose;
    }

    // --- Continuations ---
    // else if ... then
    let re_else_if = re!(ELSE_IF, r"(?i)^else\s*if\b");
    if re_else_if.is_match(line) {
        return LineKind::FortranContinuation;
    }
    // bare else (possibly followed by comment)
    let re_bare_else = re!(BARE_ELSE, r"(?i)^else\s*(!.*)?$");
    if re_bare_else.is_match(line) {
        return LineKind::FortranContinuation;
    }
    // case (...) or case default
    let re_case = re!(CASE, r"(?i)^case\s*(\(|default\b)");
    if re_case.is_match(line) {
        return LineKind::FortranContinuation;
    }
    // type is (...) / class is (...) / class default
    let re_type_is = re!(TYPE_IS, r"(?i)^(type|class)\s+is\s*\(");
    if re_type_is.is_match(line) {
        return LineKind::FortranContinuation;
    }
    if lower == "class default" || lower.starts_with("class default") && lower[13..].trim().is_empty() {
        return LineKind::FortranContinuation;
    }
    // rank (...) or rank default
    let re_rank = re!(RANK_CONT, r"(?i)^rank\s*(\(|default\b)");
    if re_rank.is_match(line) {
        return LineKind::FortranContinuation;
    }
    // elsewhere
    let re_elsewhere = re!(ELSEWHERE, r"(?i)^elsewhere\b");
    if re_elsewhere.is_match(line) {
        return LineKind::FortranContinuation;
    }

    // --- Contains ---
    if lower == "contains" {
        return LineKind::FortranContains;
    }

    // --- Block openers ---

    // if (...) then
    if lower.starts_with("if") {
        let re_if = re!(IF_OPEN, r"(?i)^if\s*\(");
        if re_if.is_match(line) {
            // Find the matching closing paren, then check if "then" follows
            if let Some(paren_start) = line.find('(') {
                if let Some(close) = find_matching_paren(line, paren_start) {
                    let after = line[close + 1..].trim();
                    // Strip trailing comment (! to end) before checking for "then"
                    // e.g., "then! comment" should still match
                    let after_no_comment = strip_trailing_comment(after);
                    let after_lower = after_no_comment.to_ascii_lowercase();
                    // Could end with "then" or "label then"
                    if after_lower == "then" || after_lower.ends_with("then") {
                        // Check that it truly ends with "then"
                        let re_then = re!(THEN_END, r"(?i)\bthen\s*$");
                        if re_then.is_match(&after_no_comment) {
                            return LineKind::FortranBlockOpen;
                        }
                    }
                }
            }
            return LineKind::FortranStatement;
        }
    }

    // do / do concurrent
    let re_do = re!(DO_OPEN, r"(?i)^do\b");
    if re_do.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // select case/type/rank
    let re_select = re!(SELECT, r"(?i)^select\s*(case|type|rank)\b");
    if re_select.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // subroutine
    let re_sub = re!(SUB, r"(?i)^(((pure|elemental|impure|recursive|module)\s+)*)subroutine\b");
    if re_sub.is_match(line) {
        // But "module procedure" is a statement, not an opener
        // subroutine is always an opener
        return LineKind::FortranBlockOpen;
    }

    // function (with optional prefixes)
    let re_func = re!(
        FUNC,
        r"(?i)^(((pure|elemental|impure|recursive|integer|real|double\s+precision|complex|character|logical|type\s*\([^)]*\))\s+)*)function\b"
    );
    if re_func.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // module (but not "module procedure")
    if lower.starts_with("module") {
        let re_mod_proc = re!(MOD_PROC, r"(?i)^module\s+procedure\b");
        if re_mod_proc.is_match(line) {
            return LineKind::FortranStatement;
        }
        let re_module = re!(MODULE, r"(?i)^module\b");
        if re_module.is_match(line) {
            return LineKind::FortranBlockOpen;
        }
    }

    // submodule
    let re_submod = re!(SUBMOD, r"(?i)^submodule\b");
    if re_submod.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // program
    let re_prog = re!(PROG, r"(?i)^program\b");
    if re_prog.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // interface
    let re_iface = re!(IFACE, r"(?i)^(abstract\s+)?interface\b");
    if re_iface.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // type definition vs type usage
    if lower.starts_with("type") {
        // type(name) :: x  -> statement (type usage/declaration)
        let re_type_usage = re!(TYPE_USAGE, r"(?i)^type\s*\(");
        if re_type_usage.is_match(line) {
            return LineKind::FortranStatement;
        }
        // type is (...) was already handled in continuations above
        // type :: name  or  type, attrs :: name  -> definition (block open)
        let re_type_def = re!(TYPE_DEF, r"(?i)^type\b[\s,]");
        if re_type_def.is_match(line) {
            return LineKind::FortranBlockOpen;
        }
        // bare "type" without parens -> could be block open
        if lower == "type" {
            return LineKind::FortranBlockOpen;
        }
    }

    // where block vs statement
    if lower.starts_with("where") {
        let re_where = re!(WHERE, r"(?i)^where\s*\(");
        if re_where.is_match(line) {
            if let Some(paren_start) = line.find('(') {
                if let Some(close) = find_matching_paren(line, paren_start) {
                    let after = line[close + 1..].trim();
                    if after.is_empty() {
                        return LineKind::FortranBlockOpen;
                    } else {
                        return LineKind::FortranStatement;
                    }
                }
            }
        }
    }

    // forall block vs statement (same logic as where)
    if lower.starts_with("forall") {
        let re_forall = re!(FORALL, r"(?i)^forall\s*\(");
        if re_forall.is_match(line) {
            if let Some(paren_start) = line.find('(') {
                if let Some(close) = find_matching_paren(line, paren_start) {
                    let after = line[close + 1..].trim();
                    if after.is_empty() {
                        return LineKind::FortranBlockOpen;
                    } else {
                        return LineKind::FortranStatement;
                    }
                }
            }
        }
    }

    // block (bare keyword)
    if lower == "block" {
        return LineKind::FortranBlockOpen;
    }

    // associate
    let re_assoc = re!(ASSOC, r"(?i)^associate\s*\(");
    if re_assoc.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // critical
    if lower == "critical" || lower.starts_with("critical") {
        let re_crit = re!(CRIT, r"(?i)^critical\b");
        if re_crit.is_match(line) {
            return LineKind::FortranBlockOpen;
        }
    }

    // enum
    let re_enum = re!(ENUM, r"(?i)^enum\b");
    if re_enum.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // change team
    let re_change_team = re!(CHANGE_TEAM, r"(?i)^change\s+team\b");
    if re_change_team.is_match(line) {
        return LineKind::FortranBlockOpen;
    }

    // Default: statement
    LineKind::FortranStatement
}

/// Classify a logical line.
pub fn classify(line: &str) -> LineKind {
    let trimmed = line.trim();

    // Blank
    if trimmed.is_empty() {
        return LineKind::Blank;
    }

    // 1. Fypp: #: or #!
    if trimmed.starts_with("#:") {
        return classify_fypp(trimmed);
    }
    if trimmed.starts_with("#!") {
        return LineKind::FyppStatement;
    }

    // 2. Preprocessor: # (but not #: or #!)
    if trimmed.starts_with('#') {
        return classify_preprocessor(trimmed);
    }

    // 3. Directive: !$ or !DIR
    if trimmed.starts_with("!$") || trimmed.starts_with("!DIR") || trimmed.starts_with("!dir") {
        return LineKind::Directive;
    }

    // 4. Inline Fypp: $: or @:
    if trimmed.starts_with("$:") || trimmed.starts_with("@:") {
        return LineKind::InlineFypp;
    }

    // 5. Comment: !
    if trimmed.starts_with('!') {
        return LineKind::Comment;
    }

    // 6. Fortran
    classify_fortran(trimmed)
}

/// Extract the scope name from a Fortran block-opening line.
///
/// Examples:
/// - `subroutine s_foo(...)` -> Some("s_foo")
/// - `pure subroutine s_bar` -> Some("s_bar")
/// - `function f_baz(x) result(y)` -> Some("f_baz")
/// - `module m_rhs` -> Some("m_rhs")
/// - `program main` -> Some("main")
/// - `type :: my_type` -> Some("my_type")
/// - `type, extends(base) :: child` -> Some("child")
pub fn extract_scope_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();

    // subroutine name(...) or subroutine name
    let re_sub = re!(
        SCOPE_SUB,
        r"(?i)(?:(?:pure|elemental|impure|recursive|module)\s+)*subroutine\s+(\w+)"
    );
    if let Some(caps) = re_sub.captures(trimmed) {
        return Some(caps[1].to_string());
    }

    // function name(...) or function name
    let re_func = re!(
        SCOPE_FUNC,
        r"(?i)(?:(?:pure|elemental|impure|recursive|integer|real|double\s+precision|complex|character|logical|type\s*\([^)]*\))\s+)*function\s+(\w+)"
    );
    if let Some(caps) = re_func.captures(trimmed) {
        return Some(caps[1].to_string());
    }

    // module name (but not "module procedure")
    if lower.starts_with("module") {
        let re_mod = re!(SCOPE_MOD, r"(?i)^module\s+(\w+)");
        if let Some(caps) = re_mod.captures(trimmed) {
            let name = &caps[1];
            if !name.eq_ignore_ascii_case("procedure") {
                return Some(name.to_string());
            }
        }
    }

    // program name
    let re_prog = re!(SCOPE_PROG, r"(?i)^program\s+(\w+)");
    if let Some(caps) = re_prog.captures(trimmed) {
        return Some(caps[1].to_string());
    }

    // type [, attrs] :: name  or  type name
    if lower.starts_with("type") {
        // type :: name  or  type, extends(...) :: name
        let re_type_def = re!(SCOPE_TYPE, r"(?i)^type\b[^(]*::\s*(\w+)");
        if let Some(caps) = re_type_def.captures(trimmed) {
            return Some(caps[1].to_string());
        }
        // type name (no :: but also not type(...) which is usage)
        let re_type_bare = re!(SCOPE_TYPE_BARE, r"(?i)^type\s+(\w+)\s*$");
        if let Some(caps) = re_type_bare.captures(trimmed) {
            let name = &caps[1];
            // Exclude "type is" which is a continuation
            if !name.eq_ignore_ascii_case("is") {
                return Some(name.to_string());
            }
        }
    }

    // submodule(parent) name
    let re_submod = re!(SCOPE_SUBMOD, r"(?i)^submodule\s*\([^)]*\)\s*(\w+)");
    if let Some(caps) = re_submod.captures(trimmed) {
        return Some(caps[1].to_string());
    }

    None
}

/// Check if an `end ...` line already has a name after the keyword.
/// E.g., `end subroutine s_foo` -> true, `end subroutine` -> false.
pub fn end_statement_has_name(line: &str) -> bool {
    let trimmed = line.trim();
    let re_end_with_name = re!(
        END_WITH_NAME,
        r"(?i)^end\s+(subroutine|function|module|submodule|program|type)\s+\w+"
    );
    re_end_with_name.is_match(trimmed)
}

/// Extract the block keyword from an `end` statement.
/// E.g., `end subroutine` -> Some("subroutine"), `end` -> None.
pub fn end_block_keyword(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let lower_bytes = trimmed.as_bytes();

    // Must start with "end" (case-insensitive)
    if trimmed.len() < 3 {
        return None;
    }
    let prefix: String = lower_bytes[..3].iter().map(|&b| b.to_ascii_lowercase() as char).collect();
    if prefix != "end" {
        return None;
    }

    let rest = trimmed[3..].trim_start();
    if rest.is_empty() {
        return None;
    }

    // Get the keyword (first word after "end")
    let keyword_end = rest.find(|c: char| c.is_whitespace() || c == '!').unwrap_or(rest.len());
    let keyword = &rest[..keyword_end];
    let keyword_lower: String = keyword.chars().map(|c| c.to_ascii_lowercase()).collect();

    match keyword_lower.as_str() {
        "subroutine" | "function" | "module" | "submodule" | "program" | "type" => {
            Some(&rest[..keyword_end])
        }
        _ => None,
    }
}
