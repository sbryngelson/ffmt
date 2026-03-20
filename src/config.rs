use serde::Deserialize;
use std::path::{Path, PathBuf};

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
    pub normalize_keywords: bool,
    /// Indent Fypp preprocessor blocks.
    #[serde(rename = "indent-fypp")]
    pub indent_fypp: bool,
    /// Indent body of module/program.
    #[serde(rename = "indent-module")]
    pub indent_module: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WhitespaceConfig {
    /// Space around relational operators (==, /=, <, <=, >, >=).
    pub relational: bool,
    /// Space around logical operators (.and., .or., .not.).
    pub logical: bool,
    /// Space around binary +/-.
    pub plusminus: bool,
    /// Space around * and /.
    pub multdiv: bool,
    /// Space around **.
    pub power: bool,
    /// Space around = in assignments.
    pub assignment: bool,
    /// Space around =>.
    pub pointer: bool,
    /// Space around // (string concatenation).
    pub concatenation: bool,
    /// Space around :: in declarations.
    pub declaration: bool,
    /// Space after comma.
    pub comma: bool,
    /// Space around : in array slices.
    #[serde(rename = "slice-colon")]
    pub slice_colon: bool,
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
            line_length: 1000,
            keyword_case: KeywordCase::Lower,
            normalize_keywords: true,
            indent_fypp: true,
            indent_module: true,
            whitespace: WhitespaceConfig::default(),
            files: FilesConfig::default(),
        }
    }
}

impl Default for WhitespaceConfig {
    fn default() -> Self {
        WhitespaceConfig {
            relational: true,
            logical: true,
            plusminus: true,
            multdiv: false,
            power: false,
            assignment: true,
            pointer: true,
            concatenation: true,
            declaration: true,
            comma: true,
            slice_colon: false,
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

/// Search upward from `dir` for config files.
fn find_config_file(dir: &Path) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    loop {
        let candidate = current.join("ffmt.toml");
        if candidate.is_file() {
            return Some(candidate);
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

/// Load config from a file path.
fn load_config_file(path: &Path) -> Result<Config, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if filename == "pyproject.toml" {
        let pyproject: PyprojectToml = toml::from_str(&content)
            .map_err(|e| format!("parse error in {}: {e}", path.display()))?;
        Ok(pyproject.tool.and_then(|t| t.ffmt).unwrap_or_default())
    } else {
        toml::from_str::<Config>(&content)
            .map_err(|e| format!("parse error in {}: {e}", path.display()))
    }
}
