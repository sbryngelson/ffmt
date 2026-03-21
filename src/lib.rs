pub mod reader;
pub mod classifier;
pub mod scope;
pub mod whitespace;
pub mod case_norm;
pub mod keyword_norm;
pub mod formatter;
pub mod config;
pub mod align;
pub mod unicode;
pub mod cli;
pub mod lsp;

pub use config::Config;

/// Match a dot-delimited operator/literal starting at `pos` in `bytes`.
/// Returns `(end_position, is_operator)` where `is_operator` distinguishes
/// `.true.`/`.false.` (literals, not operators) from `.and.`/`.eq.` etc.
pub fn match_dot_token(bytes: &[u8], pos: usize) -> Option<(usize, bool)> {
    if bytes[pos] != b'.' {
        return None;
    }
    let len = bytes.len();
    let mut end = pos + 1;
    while end < len && bytes[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end >= len || bytes[end] != b'.' {
        return None;
    }
    end += 1;

    let word: String = bytes[pos + 1..end - 1]
        .iter()
        .map(|&b| (b as char).to_ascii_lowercase())
        .collect();

    match word.as_str() {
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "and" | "or" | "not" | "eqv" | "neqv" => {
            Some((end, true)) // operator
        }
        "true" | "false" => {
            Some((end, false)) // literal
        }
        _ => None,
    }
}

/// Format a Fortran source string using default config.
pub fn format_string(source: &str) -> String {
    formatter::format(source)
}

/// Format with a specific config.
pub fn format_string_with_config(source: &str, config: &Config) -> String {
    formatter::format_with_config(source, config, None)
}

/// Format only lines within a range (1-based inclusive).
pub fn format_range(source: &str, start: usize, end: usize) -> String {
    formatter::format_with_range(source, Some((start, end)))
}

/// Format a file in-place using default config. Returns true if changed.
pub fn format_file(path: &std::path::Path) -> std::io::Result<bool> {
    let source = std::fs::read_to_string(path)?;
    let formatted = format_string(&source);
    if formatted == source {
        return Ok(false);
    }
    std::fs::write(path, &formatted)?;
    Ok(true)
}
