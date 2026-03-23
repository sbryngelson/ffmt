/// Replace Unicode Greek letters and math symbols with ASCII/LaTeX equivalents.
/// Only applied to comment content — never modifies strings or code.
pub fn replace_unicode(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            // Lowercase Greek
            '\u{03B1}' => result.push_str("\\alpha"),
            '\u{03B2}' => result.push_str("\\beta"),
            '\u{03B3}' => result.push_str("\\gamma"),
            '\u{03B4}' => result.push_str("\\delta"),
            '\u{03B5}' => result.push_str("\\epsilon"),
            '\u{03B6}' => result.push_str("\\zeta"),
            '\u{03B7}' => result.push_str("\\eta"),
            '\u{03B8}' => result.push_str("\\theta"),
            '\u{03B9}' => result.push_str("\\iota"),
            '\u{03BA}' => result.push_str("\\kappa"),
            '\u{03BB}' => result.push_str("\\lambda"),
            '\u{03BC}' => result.push_str("\\mu"),
            '\u{03BD}' => result.push_str("\\nu"),
            '\u{03BE}' => result.push_str("\\xi"),
            '\u{03BF}' => result.push_str("\\omicron"),
            '\u{03C0}' => result.push_str("\\pi"),
            '\u{03C1}' => result.push_str("\\rho"),
            '\u{03C2}' | '\u{03C3}' => result.push_str("\\sigma"),
            '\u{03C4}' => result.push_str("\\tau"),
            '\u{03C5}' => result.push_str("\\upsilon"),
            '\u{03C6}' | '\u{03D5}' => result.push_str("\\phi"),
            '\u{03C7}' => result.push_str("\\chi"),
            '\u{03C8}' => result.push_str("\\psi"),
            '\u{03C9}' => result.push_str("\\omega"),

            // Uppercase Greek
            '\u{0391}' => result.push_str("\\Alpha"),
            '\u{0392}' => result.push_str("\\Beta"),
            '\u{0393}' => result.push_str("\\Gamma"),
            '\u{0394}' => result.push_str("\\Delta"),
            '\u{0395}' => result.push_str("\\Epsilon"),
            '\u{0396}' => result.push_str("\\Zeta"),
            '\u{0397}' => result.push_str("\\Eta"),
            '\u{0398}' => result.push_str("\\Theta"),
            '\u{0399}' => result.push_str("\\Iota"),
            '\u{039A}' => result.push_str("\\Kappa"),
            '\u{039B}' => result.push_str("\\Lambda"),
            '\u{039C}' => result.push_str("\\Mu"),
            '\u{039D}' => result.push_str("\\Nu"),
            '\u{039E}' => result.push_str("\\Xi"),
            '\u{039F}' => result.push_str("\\Omicron"),
            '\u{03A0}' => result.push_str("\\Pi"),
            '\u{03A1}' => result.push_str("\\Rho"),
            '\u{03A3}' => result.push_str("\\Sigma"),
            '\u{03A4}' => result.push_str("\\Tau"),
            '\u{03A5}' => result.push_str("\\Upsilon"),
            '\u{03A6}' => result.push_str("\\Phi"),
            '\u{03A7}' => result.push_str("\\Chi"),
            '\u{03A8}' => result.push_str("\\Psi"),
            '\u{03A9}' => result.push_str("\\Omega"),

            // Math operators and symbols
            '\u{2202}' => result.push_str("\\partial"),
            '\u{221A}' => result.push_str("\\sqrt"),
            '\u{221E}' => result.push_str("\\infty"),
            '\u{2207}' => result.push_str("\\nabla"),
            '\u{222B}' => result.push_str("\\int"),
            '\u{2211}' => result.push_str("\\sum"),
            '\u{220F}' => result.push_str("\\prod"),
            '\u{2208}' => result.push_str("\\in"),
            '\u{2209}' => result.push_str("\\notin"),
            '\u{2282}' => result.push_str("\\subset"),
            '\u{2283}' => result.push_str("\\supset"),
            '\u{2229}' => result.push_str("\\cap"),
            '\u{222A}' => result.push_str("\\cup"),
            '\u{2205}' => result.push_str("\\emptyset"),
            '\u{2200}' => result.push_str("\\forall"),
            '\u{2203}' => result.push_str("\\exists"),

            // Relations
            '\u{2248}' => result.push_str("\\approx"),
            '\u{2260}' => result.push_str("\\neq"),
            '\u{2261}' => result.push_str("\\equiv"),
            '\u{2264}' => result.push_str("<="),
            '\u{2265}' => result.push_str(">="),
            '\u{226A}' => result.push_str("\\ll"),
            '\u{226B}' => result.push_str("\\gg"),
            '\u{221D}' => result.push_str("\\propto"),
            '\u{2225}' => result.push_str("\\parallel"),
            '\u{22A5}' => result.push_str("\\perp"),

            // Arrows
            '\u{2190}' => result.push_str("<-"),
            '\u{2192}' => result.push_str("->"),
            '\u{2194}' => result.push_str("<->"),
            '\u{21D0}' => result.push_str("<="),
            '\u{21D2}' => result.push_str("=>"),
            '\u{21D4}' => result.push_str("<=>"),

            // Miscellaneous math
            '\u{00B1}' => result.push_str("+/-"),
            '\u{00D7}' => result.push('*'),
            '\u{00F7}' => result.push('/'),
            '\u{2212}' => result.push('-'), // minus sign
            '\u{22C5}' => result.push('*'), // dot operator
            '\u{2218}' => result.push_str("\\circ"),
            '\u{00B0}' => result.push_str(" deg"), // degree

            // Dashes (typographic → ASCII hyphen)
            '\u{2013}' => result.push('-'), // en dash
            '\u{2014}' => result.push('-'), // em dash
            '\u{2010}' => result.push('-'), // hyphen

            // Typographic quotes → ASCII
            '\u{201C}' | '\u{201D}' => result.push('"'),
            '\u{2018}' | '\u{2019}' => result.push('\''),
            '\u{2026}' => result.push_str("..."), // ellipsis

            // Accented Latin (common in author names)
            '\u{00E9}' => result.push('e'), // e-acute (Alfven)
            '\u{00F8}' => result.push('o'), // o-slash (Norsett)
            '\u{00FC}' => result.push('u'), // u-umlaut
            '\u{00F6}' => result.push('o'), // o-umlaut
            '\u{00E4}' => result.push('a'), // a-umlaut
            '\u{00E8}' => result.push('e'), // e-grave
            '\u{00EA}' => result.push('e'), // e-circumflex
            '\u{00E0}' => result.push('a'), // a-grave
            '\u{00E1}' => result.push('a'), // a-acute
            '\u{00ED}' => result.push('i'), // i-acute
            '\u{00F3}' => result.push('o'), // o-acute
            '\u{00FA}' => result.push('u'), // u-acute
            '\u{00F1}' => result.push('n'), // n-tilde
            '\u{00E7}' => result.push('c'), // c-cedilla

            // Everything else: pass through
            _ => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greek_lowercase() {
        assert_eq!(replace_unicode("σ = 0"), "\\sigma = 0");
        assert_eq!(replace_unicode("π/2"), "\\pi/2");
        assert_eq!(replace_unicode("∂f/∂x"), "\\partialf/\\partialx");
    }

    #[test]
    fn test_greek_uppercase() {
        assert_eq!(replace_unicode("ΔE"), "\\DeltaE");
        assert_eq!(replace_unicode("Ω"), "\\Omega");
    }

    #[test]
    fn test_math_symbols() {
        assert_eq!(replace_unicode("√2"), "\\sqrt2");
        assert_eq!(replace_unicode("∞"), "\\infty");
        assert_eq!(replace_unicode("≈"), "\\approx");
        assert_eq!(replace_unicode("≠"), "\\neq");
    }

    #[test]
    fn test_arrows() {
        assert_eq!(replace_unicode("⇒"), "=>");
        assert_eq!(replace_unicode("→"), "->");
    }

    #[test]
    fn test_dashes() {
        assert_eq!(replace_unicode("Courant–Friedrichs"), "Courant-Friedrichs");
        assert_eq!(replace_unicode("a—b"), "a-b");
    }

    #[test]
    fn test_accented() {
        assert_eq!(replace_unicode("Alfvén"), "Alfven");
        assert_eq!(replace_unicode("Nørsett"), "Norsett");
    }

    #[test]
    fn test_smart_quotes() {
        assert_eq!(replace_unicode("\u{201C}hello\u{201D}"), "\"hello\"");
    }

    #[test]
    fn test_ascii_unchanged() {
        assert_eq!(replace_unicode("x = y + z"), "x = y + z");
        assert_eq!(replace_unicode("! comment"), "! comment");
    }
}
