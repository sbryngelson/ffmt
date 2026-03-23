use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Tri-state toggle: Enable, Disable, or Preserve (don't touch).
/// Accepts `true`, `false`, or `"preserve"` in TOML config.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Toggle {
    Enable,
    Disable,
    Preserve,
}

impl Toggle {
    pub fn is_enabled(self) -> bool {
        self == Toggle::Enable
    }
}

impl<'de> Deserialize<'de> for Toggle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct ToggleVisitor;

        impl<'de> de::Visitor<'de> for ToggleVisitor {
            type Value = Toggle;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("true, false, or \"preserve\"")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Toggle, E> {
                Ok(if v { Toggle::Enable } else { Toggle::Disable })
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Toggle, E> {
                match v.to_ascii_lowercase().as_str() {
                    "true" | "enable" | "on" | "one-per-line" => Ok(Toggle::Enable),
                    "false" | "disable" | "off" => Ok(Toggle::Disable),
                    "preserve" | "keep" => Ok(Toggle::Preserve),
                    _ => Err(de::Error::unknown_variant(
                        v,
                        &["true", "false", "preserve"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(ToggleVisitor)
    }
}

/// Configuration for ffmt formatting.
///
/// Defaults match MFC's current style conventions.
/// Config is loaded from `ffmt.toml` or `[tool.ffmt]` in `pyproject.toml`,
/// searched upward from the formatted file's directory.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Indentation width in spaces.
    #[serde(rename = "indent-width")]
    pub indent_width: usize,
    /// Maximum line length before splitting.
    #[serde(rename = "line-length")]
    pub line_length: usize,
    /// Keyword case normalization mode.
    #[serde(rename = "keyword-case")]
    pub keyword_case: KeywordCase,
    /// Normalize compound keywords (enddo -> end do).
    #[serde(rename = "normalize-keywords")]
    pub normalize_keywords: Toggle,
    /// Indent Fypp preprocessor blocks.
    #[serde(rename = "indent-fypp")]
    pub indent_fypp: bool,
    /// Indent body of module/program.
    #[serde(rename = "indent-module")]
    pub indent_module: bool,
    /// Add procedure/module name to bare end statements.
    #[serde(rename = "named-ends")]
    pub named_ends: Toggle,
    /// Align :: in consecutive declaration blocks.
    #[serde(rename = "align-declarations")]
    pub align_declarations: bool,
    /// Align !< inline Doxygen comments in declaration blocks.
    #[serde(rename = "align-comments")]
    pub align_comments: bool,
    /// Remove blank lines between consecutive declarations.
    #[serde(rename = "compact-declarations")]
    pub compact_declarations: Toggle,
    /// Remove blank lines between consecutive use statements.
    #[serde(rename = "compact-use")]
    pub compact_use: Toggle,
    /// Replace Unicode Greek/math symbols with LaTeX in comments.
    #[serde(rename = "unicode-to-ascii")]
    pub unicode_to_ascii: bool,
    /// Re-wrap long comments at line-length.
    #[serde(rename = "rewrap-comments")]
    pub rewrap_comments: Toggle,
    /// Re-wrap long code lines at line-length.
    #[serde(rename = "rewrap-code")]
    pub rewrap_code: Toggle,
    /// Ensure space after ! in regular comments.
    #[serde(rename = "space-after-comment")]
    pub space_after_comment: Toggle,
    /// Collapse double spaces to single in code regions.
    #[serde(rename = "collapse-double-spaces")]
    pub collapse_double_spaces: bool,
    /// Add space between keywords and ( (if, call, allocate, etc.).
    #[serde(rename = "keyword-paren-space")]
    pub keyword_paren_space: Toggle,
    /// Normalize comma spacing in Fypp '[...]' list arguments.
    #[serde(rename = "fypp-list-commas")]
    pub fypp_list_commas: Toggle,
    /// Line ending normalization: "lf", "crlf", or "preserve".
    #[serde(rename = "end-of-line")]
    pub end_of_line: EndOfLine,
    /// Modernize legacy relational operators (.eq. -> ==). Accepts true/false/"preserve".
    #[serde(rename = "modernize-operators")]
    pub modernize_operators: Toggle,
    /// Enforce :: separator in variable declarations. Accepts true/false/"preserve".
    #[serde(rename = "enforce-double-colon")]
    pub enforce_double_colon: Toggle,
    /// Split semicolon-separated statements onto separate lines. Accepts true/false/"preserve".
    #[serde(rename = "split-statements")]
    pub split_statements: Toggle,
    /// Align trailing & continuation markers at column limit. Accepts true/false/"preserve".
    #[serde(rename = "align-ampersand")]
    pub align_ampersand: Toggle,
    /// Align = in consecutive assignment statements. Accepts true/false/"preserve".
    #[serde(rename = "align-assignments")]
    pub align_assignments: Toggle,
    /// Reformat use-statement imports. "one-per-line" puts each imported name on its own line.
    /// Accepts "one-per-line", false, or "preserve".
    #[serde(rename = "use-formatting")]
    pub use_formatting: Toggle,
    /// Whitespace rules.
    pub whitespace: WhitespaceConfig,
    /// File handling.
    pub files: FilesConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeywordCase {
    Lower,
    Upper,
    Preserve,
}

/// Line ending normalization mode.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndOfLine {
    Lf,
    Crlf,
    Preserve,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WhitespaceConfig {
    /// Space around relational operators (==, /=, <, <=, >, >=).
    pub relational: Toggle,
    /// Space around logical operators (.and., .or., .not.).
    pub logical: Toggle,
    /// Space around binary +/-.
    pub plusminus: Toggle,
    /// Space around * and /.
    pub multdiv: Toggle,
    /// Space around **.
    pub power: Toggle,
    /// Space around = in assignments.
    pub assignment: Toggle,
    /// Space around =>.
    pub pointer: Toggle,
    /// Space around // (string concatenation).
    pub concatenation: Toggle,
    /// Space around :: in declarations.
    pub declaration: Toggle,
    /// Space after comma.
    pub comma: Toggle,
    /// Space around : in array slices.
    #[serde(rename = "slice-colon")]
    pub slice_colon: Toggle,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilesConfig {
    /// File extensions to treat as Fortran.
    pub extensions: Vec<String>,
    /// Patterns to exclude.
    pub exclude: Vec<String>,
    /// Respect .gitignore files.
    #[serde(rename = "respect-gitignore")]
    pub respect_gitignore: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            indent_width: 4,
            line_length: 132,
            keyword_case: KeywordCase::Lower,
            normalize_keywords: Toggle::Enable,
            indent_fypp: true,
            indent_module: true,
            named_ends: Toggle::Enable,
            align_declarations: true,
            align_comments: true,
            compact_declarations: Toggle::Enable,
            compact_use: Toggle::Enable,
            unicode_to_ascii: true,
            rewrap_comments: Toggle::Enable,
            rewrap_code: Toggle::Enable,
            space_after_comment: Toggle::Enable,
            collapse_double_spaces: true,
            keyword_paren_space: Toggle::Enable,
            fypp_list_commas: Toggle::Enable,
            end_of_line: EndOfLine::Lf,
            modernize_operators: Toggle::Enable,
            enforce_double_colon: Toggle::Enable,
            split_statements: Toggle::Disable,
            align_ampersand: Toggle::Disable,
            align_assignments: Toggle::Disable,
            use_formatting: Toggle::Disable,
            whitespace: WhitespaceConfig::default(),
            files: FilesConfig::default(),
        }
    }
}

impl Default for WhitespaceConfig {
    fn default() -> Self {
        WhitespaceConfig {
            relational: Toggle::Enable,
            logical: Toggle::Enable,
            plusminus: Toggle::Enable,
            multdiv: Toggle::Disable,
            power: Toggle::Disable,
            assignment: Toggle::Enable,
            pointer: Toggle::Enable,
            concatenation: Toggle::Enable,
            declaration: Toggle::Enable,
            comma: Toggle::Enable,
            slice_colon: Toggle::Disable,
        }
    }
}

impl Default for FilesConfig {
    fn default() -> Self {
        FilesConfig {
            extensions: vec![
                "fpp".into(),
                "f90".into(),
                "F90".into(),
                "f95".into(),
                "f03".into(),
                "F".into(),
                "F95".into(),
                "F03".into(),
            ],
            exclude: Vec::new(),
            respect_gitignore: true,
        }
    }
}

/// For reading [tool.ffmt] from pyproject.toml
#[derive(Deserialize)]
struct PyprojectToml {
    tool: Option<PyprojectTool>,
}

#[derive(Deserialize)]
struct PyprojectTool {
    ffmt: Option<Config>,
}

impl Config {
    /// Search upward from `start_dir` for `ffmt.toml` or `pyproject.toml`
    /// with `[tool.ffmt]`. Returns default config if nothing found.
    pub fn find_and_load(start_dir: &Path) -> Self {
        if let Some(path) = find_config_file(start_dir) {
            match load_config_file(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("ffmt: warning: error reading {}: {e}", path.display());
                    Config::default()
                }
            }
        } else {
            Config::default()
        }
    }

    /// Load from a specific config file path.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        load_config_file(path)
    }
}

fn find_config_file(dir: &Path) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    loop {
        // Check ffmt.toml, then .ffmt.toml (hidden), then pyproject.toml
        for name in ["ffmt.toml", ".ffmt.toml"] {
            let candidate = current.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let candidate = current.join("pyproject.toml");
        if candidate.is_file() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[tool.ffmt]") {
                    return Some(candidate);
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn load_config_file(path: &Path) -> Result<Config, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if filename == "pyproject.toml" {
        let pyproject: PyprojectToml = toml::from_str(&content)
            .map_err(|e| format!("parse error in {}: {e}", path.display()))?;
        Ok(pyproject.tool.and_then(|t| t.ffmt).unwrap_or_default())
    } else {
        toml::from_str::<Config>(&content)
            .map_err(|e| format!("parse error in {}: {e}", path.display()))
    }
}
