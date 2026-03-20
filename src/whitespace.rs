/// Token types emitted by the tokenizer.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// Opaque text that should not be modified (identifiers, numbers, keywords,
    /// string literals, Fypp expressions, comment tails).
    Text(String),
    /// An operator or punctuation token with associated spacing rule.
    Op(OpKind, String),
    /// Whitespace in the original (collapsed/removed as needed).
    Space,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OpKind {
    /// `==`, `/=`, `<`, `<=`, `>`, `>=` — single space each side
    Relational,
    /// `.eq.`, `.and.`, etc. — single space each side
    DotOp,
    /// `=` at paren depth 0 — single space each side
    Assignment,
    /// `=` at paren depth > 0 — no spaces (keyword arg)
    KeywordArg,
    /// `=>` — single space each side
    Pointer,
    /// `//` (string concatenation) — single space each side
    Concat,
    /// Binary `+` or `-` — single space each side
    BinaryAddSub,
    /// Unary `+` or `-` — no space after, attaches to operand
    UnarySign,
    /// `*` (multiply) — no spaces
    Multiply,
    /// `/` (divide) — no spaces
    Divide,
    /// `**` (exponentiation) — no spaces
    Power,
    /// `,` — no space before, one space after
    Comma,
    /// `:` inside parens (array slice) — no spaces
    SliceColon,
    /// `::` (declaration) — single space each side
    DoubleColon,
    /// `(` or `[` — no internal padding (no space after)
    OpenParen,
    /// `)` or `]` — no internal padding (no space before)
    CloseParen,
    /// `;` — preserve as-is with surrounding spaces
    Semicolon,
    /// `!` starting inline comment — at least one space before
    Comment,
}

use crate::config::WhitespaceConfig;

/// Normalize whitespace in a Fortran line.
pub fn normalize_whitespace(line: &str, ws_config: &WhitespaceConfig) -> String {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }

    let tokens = tokenize(trimmed);
    render(&tokens, ws_config)
}

/// Find the position in `line` of the end of non-space content before position `pos`.
/// Used to capture original spacing before Doxygen comments.
fn out_position_before_spaces(line: &str, pos: usize) -> usize {
    let bytes = line.as_bytes();
    let mut j = pos;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        j -= 1;
    }
    j
}

/// Check if byte at `pos` is a `+` or `-` that is part of a Fortran exponent
/// notation. Matches patterns like `1.0e+3`, `1.e-5`, `2d+3`, `1.0E-16`.
/// The char at `pos` should be `+` or `-`.
fn is_exponent_sign(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return false;
    }
    let prev = bytes[pos - 1];
    if prev != b'e' && prev != b'E' && prev != b'd' && prev != b'D' {
        return false;
    }
    // Check that before the e/d there is a digit or a decimal point
    // (covers 1.0e-3, 1.e-3, 2e+5, etc.)
    if pos < 2 {
        return false;
    }
    let before_ed = bytes[pos - 2];
    before_ed.is_ascii_digit() || before_ed == b'.'
}

/// Tokenize a trimmed Fortran line into a sequence of Token values.
fn tokenize(line: &str) -> Vec<Token> {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;
    let mut paren_depth: i32 = 0;

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
                    // Doubled quote is escape, continue
                    if i < len && bytes[i] == quote {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            tokens.push(Token::Text(String::from(&line[start..i])));
            continue;
        }

        // --- Fypp inline expressions: ${...}$ and @{...}@ ---
        if ch == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
            let start = i;
            i += 2;
            let mut depth = 1;
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
            tokens.push(Token::Text(String::from(&line[start..i])));
            continue;
        }
        if ch == b'@' && i + 1 < len && bytes[i + 1] == b'{' {
            let start = i;
            i += 2;
            let mut depth = 1;
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
            tokens.push(Token::Text(String::from(&line[start..i])));
            continue;
        }

        // --- Inline comment: ! (not inside string) ---
        if ch == b'!' {
            // Everything from here to end of line is comment.
            // If the ! is followed by a Doxygen marker (<, >, !, *, @),
            // preserve the original spacing before the ! by capturing
            // any preceding whitespace as part of the comment token.
            let comment_text = &line[i..];
            let is_doxygen = comment_text.len() > 1
                && matches!(comment_text.as_bytes()[1], b'<' | b'>' | b'!' | b'*' | b'@');
            if is_doxygen {
                // Find how much whitespace precedes the ! in the original line
                // and include it in the comment token so it's preserved verbatim.
                // Remove any trailing Space tokens we already emitted.
                while matches!(tokens.last(), Some(Token::Space)) {
                    tokens.pop();
                }
                // Capture the original spacing from after the last non-space
                // content to the !
                let prev_content_end = out_position_before_spaces(line, i);
                let spacing = &line[prev_content_end..i];
                let full_comment = format!("{}{}", spacing, comment_text);
                tokens.push(Token::Op(OpKind::Comment, full_comment));
            } else {
                tokens.push(Token::Op(OpKind::Comment, String::from(comment_text)));
            }
            break;
        }

        // --- Whitespace ---
        if ch == b' ' || ch == b'\t' {
            // Collapse whitespace
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            tokens.push(Token::Space);
            continue;
        }

        // --- Dot operators: .eq., .and., etc. ---
        if ch == b'.' && i + 2 < len {
            if let Some((dot_end, true)) = crate::match_dot_token(bytes, i) {
                let op_str = &line[i..dot_end];
                tokens.push(Token::Op(OpKind::DotOp, String::from(op_str)));
                i = dot_end;
                continue;
            }
        }

        // --- Multi-char operators (longest match first) ---

        // `**`
        if ch == b'*' && i + 1 < len && bytes[i + 1] == b'*' {
            tokens.push(Token::Op(OpKind::Power, String::from("**")));
            i += 2;
            continue;
        }

        // `=>` (must check before `=` and `>`)
        if ch == b'=' && i + 1 < len && bytes[i + 1] == b'>' {
            tokens.push(Token::Op(OpKind::Pointer, String::from("=>")));
            i += 2;
            continue;
        }

        // `==`
        if ch == b'=' && i + 1 < len && bytes[i + 1] == b'=' {
            tokens.push(Token::Op(OpKind::Relational, String::from("==")));
            i += 2;
            continue;
        }

        // `/=`
        if ch == b'/' && i + 1 < len && bytes[i + 1] == b'=' {
            tokens.push(Token::Op(OpKind::Relational, String::from("/=")));
            i += 2;
            continue;
        }

        // `//` (string concatenation)
        if ch == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            tokens.push(Token::Op(OpKind::Concat, String::from("//")));
            i += 2;
            continue;
        }

        // `<=`
        if ch == b'<' && i + 1 < len && bytes[i + 1] == b'=' {
            tokens.push(Token::Op(OpKind::Relational, String::from("<=")));
            i += 2;
            continue;
        }

        // `>=`
        if ch == b'>' && i + 1 < len && bytes[i + 1] == b'=' {
            tokens.push(Token::Op(OpKind::Relational, String::from(">=")));
            i += 2;
            continue;
        }

        // `::` (declaration separator)
        if ch == b':' && i + 1 < len && bytes[i + 1] == b':' {
            tokens.push(Token::Op(OpKind::DoubleColon, String::from("::")));
            i += 2;
            continue;
        }

        // --- Single-char operators ---

        // `=` (assignment at depth 0, keyword arg at depth > 0)
        if ch == b'=' {
            let kind = if paren_depth > 0 {
                OpKind::KeywordArg
            } else {
                OpKind::Assignment
            };
            tokens.push(Token::Op(kind, String::from("=")));
            i += 1;
            continue;
        }

        // `<` and `>`
        if ch == b'<' {
            tokens.push(Token::Op(OpKind::Relational, String::from("<")));
            i += 1;
            continue;
        }
        if ch == b'>' {
            tokens.push(Token::Op(OpKind::Relational, String::from(">")));
            i += 1;
            continue;
        }

        // `+` and `-`: determine if unary or binary
        if ch == b'+' || ch == b'-' {
            // Check for exponent notation first: 1.0e+3, 1.0d-5
            if is_exponent_sign(bytes, i) {
                // Part of a numeric literal — absorb into surrounding text
                let start = i;
                i += 1;
                // Absorb following digits
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                // Merge with previous Text token if possible
                if let Some(Token::Text(ref mut s)) = tokens.last_mut() {
                    s.push_str(&line[start..i]);
                } else {
                    tokens.push(Token::Text(String::from(&line[start..i])));
                }
                continue;
            }

            let is_unary = is_unary_context(&tokens);
            let kind = if is_unary {
                OpKind::UnarySign
            } else {
                OpKind::BinaryAddSub
            };
            tokens.push(Token::Op(kind, String::from(ch as char)));
            i += 1;
            continue;
        }

        // `*` — multiply, or format specifier after print/read/write
        if ch == b'*' {
            // Check if this is a format specifier: `print *`, `read *`, `write(*,*)`
            if is_io_format_star(&tokens) {
                tokens.push(Token::Text(String::from("*")));
            } else {
                tokens.push(Token::Op(OpKind::Multiply, String::from("*")));
            }
            i += 1;
            continue;
        }

        // `/` (divide)
        if ch == b'/' {
            tokens.push(Token::Op(OpKind::Divide, String::from("/")));
            i += 1;
            continue;
        }

        // `,`
        if ch == b',' {
            tokens.push(Token::Op(OpKind::Comma, String::from(",")));
            i += 1;
            continue;
        }

        // `:` — context-dependent:
        //   Inside parens: array slice colon (no spaces by default)
        //   Outside parens: label colon or `only:` — preserve as text
        if ch == b':' {
            if paren_depth > 0 {
                tokens.push(Token::Op(OpKind::SliceColon, String::from(":")));
            } else {
                // Outside parens: label colon (outer: do) or use-only colon.
                // Treat as plain text — don't strip/add spaces.
                tokens.push(Token::Text(String::from(":")));
            }
            i += 1;
            continue;
        }

        // `(` and `[`
        if ch == b'(' || ch == b'[' {
            paren_depth += 1;
            tokens.push(Token::Op(OpKind::OpenParen, String::from(ch as char)));
            i += 1;
            continue;
        }

        // `)` and `]`
        if ch == b')' || ch == b']' {
            paren_depth -= 1;
            if paren_depth < 0 {
                paren_depth = 0;
            }
            tokens.push(Token::Op(OpKind::CloseParen, String::from(ch as char)));
            i += 1;
            continue;
        }

        // `;`
        if ch == b';' {
            tokens.push(Token::Op(OpKind::Semicolon, String::from(";")));
            i += 1;
            continue;
        }

        // --- Default: identifier, number, or other text ---
        let start = i;
        while i < len {
            let c = bytes[i];
            if c == b' '
                || c == b'\t'
                || c == b'='
                || c == b'+'
                || c == b'-'
                || c == b'*'
                || c == b'/'
                || c == b'<'
                || c == b'>'
                || c == b'('
                || c == b')'
                || c == b'['
                || c == b']'
                || c == b','
                || c == b':'
                || c == b'!'
                || c == b'\''
                || c == b'"'
                || c == b';'
            {
                // Check for dot operator
                if c == b'.' && i + 2 < len
                    && matches!(crate::match_dot_token(bytes, i), Some((_, true))) {
                        break;
                    }
                // Check for Fypp inline: ${ or @{
                if (c == b'$' || c == b'@') && i + 1 < len && bytes[i + 1] == b'{' {
                    break;
                }
                if c != b'$' && c != b'@' {
                    break;
                }
                // Bare $ or @ that isn't start of Fypp — include in text
                i += 1;
                continue;
            }
            // Fypp check for $ or @
            if (c == b'$' || c == b'@') && i + 1 < len && bytes[i + 1] == b'{' {
                break;
            }
            if c == b'.' {
                // Could be start of dot operator
                if i + 2 < len && matches!(crate::match_dot_token(bytes, i), Some((_, true))) {
                    break;
                }
            }
            i += 1;
        }
        if i > start {
            tokens.push(Token::Text(String::from(&line[start..i])));
        }
    }

    tokens
}

/// Determine if the current +/- should be treated as unary based on preceding tokens.
/// Check if `*` is an I/O format specifier (after print/read/write or after comma in I/O).
fn is_io_format_star(tokens: &[Token]) -> bool {
    for token in tokens.iter().rev() {
        match token {
            Token::Space => continue,
            Token::Text(s) => {
                let lower = s.trim().to_ascii_lowercase();
                return lower == "print" || lower == "read" || lower == "write";
            }
            Token::Op(OpKind::Comma, _) => {
                // Could be `write(*, *)` — the * after comma in I/O context.
                // Check if there's an I/O keyword earlier.
                for t in tokens.iter().rev() {
                    if let Token::Text(s) = t {
                        let lower = s.trim().to_ascii_lowercase();
                        if lower == "print" || lower == "read" || lower == "write" {
                            return true;
                        }
                    }
                }
                return false;
            }
            _ => return false,
        }
    }
    false
}

fn is_unary_context(tokens: &[Token]) -> bool {
    // Walk backwards, skipping Space tokens
    for token in tokens.iter().rev() {
        match token {
            Token::Space => continue,
            Token::Op(kind, _) => {
                match kind {
                    // After these operators, +/- is unary
                    OpKind::Assignment
                    | OpKind::KeywordArg
                    | OpKind::Relational
                    | OpKind::DotOp
                    | OpKind::Pointer
                    | OpKind::Concat
                    | OpKind::BinaryAddSub
                    | OpKind::UnarySign
                    | OpKind::Multiply
                    | OpKind::Divide
                    | OpKind::Power
                    | OpKind::Comma
                    | OpKind::OpenParen
                    | OpKind::DoubleColon
                    | OpKind::SliceColon => return true,
                    // After close paren or semicolon, it's binary
                    OpKind::CloseParen | OpKind::Semicolon | OpKind::Comment => return false,
                }
            }
            Token::Text(_) => return false,
        }
    }
    // At start of line, it's unary
    true
}

/// Render tokens back into a string with correct spacing.
fn render(tokens: &[Token], ws: &WhitespaceConfig) -> String {
    let mut out = String::new();

    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::Text(s) => {
                out.push_str(s);
            }
            Token::Space => {
                let prev_needs_space = idx > 0 && needs_trailing_space(&tokens[idx - 1]);
                let next_needs_space =
                    idx + 1 < tokens.len() && needs_leading_space(&tokens[idx + 1]);
                if prev_needs_space && next_needs_space && !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            Token::Op(kind, op_str) => {
                // Determine if this operator should have spaces based on config
                let spaced = match kind {
                    OpKind::Relational => ws.relational,
                    OpKind::DotOp => ws.logical,
                    OpKind::Assignment => ws.assignment,
                    OpKind::Pointer => ws.pointer,
                    OpKind::Concat => ws.concatenation,
                    OpKind::BinaryAddSub => ws.plusminus,
                    OpKind::DoubleColon => ws.declaration,
                    OpKind::Multiply => ws.multdiv,
                    OpKind::Divide => ws.multdiv,
                    OpKind::Power => ws.power,
                    OpKind::SliceColon => ws.slice_colon,
                    OpKind::Comma => ws.comma,
                    // These are never configurable:
                    OpKind::KeywordArg
                    | OpKind::UnarySign
                    | OpKind::OpenParen
                    | OpKind::CloseParen
                    | OpKind::Semicolon
                    | OpKind::Comment => false, // handled specially below
                };

                match kind {
                    // Configurable spaced operators (space before and after)
                    OpKind::Relational
                    | OpKind::DotOp
                    | OpKind::Assignment
                    | OpKind::Pointer
                    | OpKind::Concat
                    | OpKind::BinaryAddSub
                    | OpKind::DoubleColon
                    | OpKind::Multiply
                    | OpKind::Divide
                    | OpKind::Power
                    | OpKind::SliceColon => {
                        if spaced {
                            if !out.ends_with('(') && !out.ends_with('[') {
                                ensure_space(&mut out);
                            }
                            out.push_str(op_str);
                            out.push(' ');
                        } else {
                            trim_trailing_space(&mut out);
                            out.push_str(op_str);
                        }
                    }
                    // Comma: no space before, configurable space after
                    OpKind::Comma => {
                        trim_trailing_space(&mut out);
                        out.push_str(op_str);
                        if spaced {
                            out.push(' ');
                        }
                    }
                    // Keyword arg = : always no spaces
                    OpKind::KeywordArg => {
                        trim_trailing_space(&mut out);
                        out.push_str(op_str);
                    }
                    // Unary sign: no trailing space, preserve leading
                    OpKind::UnarySign => {
                        out.push_str(op_str);
                    }
                    // Parens: no internal padding
                    OpKind::OpenParen => {
                        out.push_str(op_str);
                    }
                    OpKind::CloseParen | OpKind::Semicolon => {
                        trim_trailing_space(&mut out);
                        out.push_str(op_str);
                        if *kind == OpKind::Semicolon {
                            out.push(' ');
                        }
                    }
                    // Comment: Doxygen preserves spacing, regular gets one space
                    OpKind::Comment => {
                        if op_str.starts_with(' ') || op_str.starts_with('\t') {
                            let trimmed_out = out.trim_end().len();
                            out.truncate(trimmed_out);
                            out.push_str(op_str);
                        } else {
                            ensure_space(&mut out);
                            out.push_str(op_str);
                        }
                    }
                }
            }
        }
    }

    out.trim_end().to_string()
}

/// Ensure `out` ends with exactly one space. If it already ends with a space, do nothing.
/// If it ends with non-space, add one space.
fn ensure_space(out: &mut String) {
    if out.is_empty() {
        return;
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
}

/// Remove trailing spaces from `out`.
fn trim_trailing_space(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

/// Does this token naturally want a trailing space (i.e., is it "content" that
/// a following Space token should preserve)?
fn needs_trailing_space(token: &Token) -> bool {
    matches!(
        token,
        Token::Text(_)
            | Token::Op(OpKind::CloseParen, _)
            | Token::Op(OpKind::Semicolon, _)
    )
}

/// Does this token naturally want a leading space?
fn needs_leading_space(token: &Token) -> bool {
    matches!(
        token,
        Token::Text(_)
            | Token::Op(OpKind::OpenParen, _)
            | Token::Op(OpKind::DotOp, _)
            | Token::Op(OpKind::Semicolon, _)
    )
}
