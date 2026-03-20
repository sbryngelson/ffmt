use clap::Parser;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

/// Default cache directory (relative to cwd).
const CACHE_DIR: &str = ".ffmt_cache";

#[derive(Parser)]
#[command(
    name = "ffmt",
    about = "An opinionated Fortran formatter",
    version
)]
struct Args {
    /// Files or directories to format. Use `-` for stdin.
    #[arg(required = true)]
    paths: Vec<String>,

    /// Check if files are formatted (exit 1 if not)
    #[arg(long)]
    check: bool,

    /// Print unified diff of changes
    #[arg(long)]
    diff: bool,

    /// Number of parallel jobs
    #[arg(short = 'j', long = "jobs", default_value = "1")]
    jobs: usize,

    /// Filepath to use in diagnostics when reading from stdin
    #[arg(long = "stdin-filepath")]
    stdin_filepath: Option<String>,

    /// Colorize output (auto, always, never)
    #[arg(long, default_value = "auto")]
    color: ColorChoice,

    /// Glob patterns to exclude (can be repeated)
    #[arg(long = "exclude", short = 'e')]
    excludes: Vec<String>,

    /// Don't respect .gitignore files
    #[arg(long)]
    no_ignore: bool,

    /// Disable file hash cache (skip-unchanged optimization)
    #[arg(long)]
    no_cache: bool,

    /// Directory to store file hash cache
    #[arg(long = "cache-dir")]
    cache_dir: Option<String>,

    /// Only format lines within this range (START:END, 1-based inclusive)
    #[arg(long)]
    range: Option<String>,

    /// Suppress all output except errors
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Show verbose output (files being processed)
    #[arg(long, short = 'v')]
    verbose: bool,
}

#[derive(Clone, Debug)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for ColorChoice {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(ColorChoice::Auto),
            "always" => Ok(ColorChoice::Always),
            "never" => Ok(ColorChoice::Never),
            _ => Err(format!(
                "invalid color choice: {s} (expected auto, always, never)"
            )),
        }
    }
}

fn use_color(choice: &ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => atty_stdout(),
    }
}

fn atty_stdout() -> bool {
    #[cfg(unix)]
    {
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        unsafe { isatty(1) != 0 }
    }
    #[cfg(windows)]
    {
        extern "C" {
            fn _isatty(fd: i32) -> i32;
        }
        unsafe { _isatty(1) != 0 }
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

pub fn run() {
    let args = Args::parse();
    let color = use_color(&args.color);

    // Load config from ffmt.toml or pyproject.toml
    let config = {
        let search_dir = if args.paths.len() == 1 && args.paths[0] == "-" {
            std::env::current_dir().unwrap_or_default()
        } else {
            let p = PathBuf::from(&args.paths[0]);
            if p.is_file() {
                p.parent().map(|p| p.to_path_buf()).unwrap_or_default()
            } else {
                p.clone()
            }
        };
        crate::config::Config::find_and_load(&search_dir)
    };

    // quiet wins over verbose if both given
    let quiet = args.quiet;
    let verbose = args.verbose && !args.quiet;

    // Handle stdin mode
    if args.paths.len() == 1 && args.paths[0] == "-" {
        run_stdin(&args, color, quiet, &config);
        return;
    }

    let paths: Vec<PathBuf> = args.paths.iter().map(PathBuf::from).collect();
    let no_ignore = args.no_ignore || !config.files.respect_gitignore;
    let mut all_excludes = args.excludes.clone();
    all_excludes.extend(config.files.exclude.clone());
    let files = discover_files_with_config(&paths, &all_excludes, no_ignore, &config.files.extensions);

    if files.is_empty() {
        eprintln!("ffmt: no Fortran files found");
        process::exit(2);
    }

    // Configure rayon thread pool
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs)
        .build_global()
        .ok();

    // Load cache
    let cache_dir = args
        .cache_dir
        .as_deref()
        .unwrap_or(CACHE_DIR);
    let mut cache = if args.no_cache || args.check {
        None
    } else {
        Some(FileCache::load(cache_dir))
    };

    // Filter files that haven't changed since last format
    let files_to_process: Vec<&PathBuf> = if let Some(ref cache) = cache {
        files
            .iter()
            .filter(|f| !cache.is_cached(f))
            .collect()
    } else {
        files.iter().collect()
    };

    let range = parse_range(&args.range);
    let any_changed = AtomicBool::new(false);

    let opts = ProcessOptions {
        check: args.check,
        diff: args.diff,
        color,
        range,
        quiet,
        verbose,
    };

    files_to_process.par_iter().for_each(|path| {
        let changed = process_file(path, &opts, &config);
        if changed {
            any_changed.store(true, Ordering::Relaxed);
        }
    });

    // Update cache for all files (including ones we skipped — they're still valid)
    if let Some(ref mut cache) = cache {
        // Only update cache when formatting (not in check/diff mode)
        if !args.check && !args.diff {
            for f in &files {
                cache.update(f);
            }
            cache.save(cache_dir);
        }
    }

    if args.check && any_changed.load(Ordering::Relaxed) {
        process::exit(1);
    }
}

fn parse_range(range_str: &Option<String>) -> Option<(usize, usize)> {
    let s = range_str.as_ref()?;
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        eprintln!("ffmt: invalid range '{s}' (expected START:END, e.g., 10:50)");
        process::exit(2);
    }
    let start: usize = parts[0].parse().unwrap_or_else(|_| {
        eprintln!("ffmt: invalid range start '{}'", parts[0]);
        process::exit(2);
    });
    let end: usize = parts[1].parse().unwrap_or_else(|_| {
        eprintln!("ffmt: invalid range end '{}'", parts[1]);
        process::exit(2);
    });
    if start == 0 || end == 0 || start > end {
        eprintln!("ffmt: invalid range {start}:{end} (must be 1-based, start <= end)");
        process::exit(2);
    }
    Some((start, end))
}

fn run_stdin(args: &Args, color: bool, quiet: bool, config: &crate::config::Config) {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .unwrap_or_else(|e| {
            eprintln!("ffmt: cannot read stdin: {e}");
            process::exit(2);
        });

    let range = parse_range(&args.range);
    let formatted = crate::formatter::format_with_config(&source, config, range);
    let filepath = args.stdin_filepath.as_deref().unwrap_or("<stdin>");

    if args.check || args.diff {
        if formatted != source {
            if args.check && !quiet {
                println!("{filepath}");
            }
            if args.diff {
                print_diff(Path::new(filepath), &source, &formatted, color);
            }
            process::exit(1);
        }
    } else {
        io::stdout()
            .write_all(formatted.as_bytes())
            .unwrap_or_else(|e| {
                eprintln!("ffmt: cannot write stdout: {e}");
                process::exit(2);
            });
    }
}

fn discover_files_with_config(
    paths: &[PathBuf],
    excludes: &[String],
    no_ignore: bool,
    extensions: &[String],
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_fortran_file_ext(path, extensions) {
                files.push(path.clone());
            }
            continue;
        }

        let mut builder = WalkBuilder::new(path);
        builder.hidden(false);

        if no_ignore {
            builder.git_ignore(false);
            builder.git_global(false);
            builder.git_exclude(false);
        } else {
            builder.git_ignore(true);
            builder.git_global(true);
            builder.git_exclude(true);
        }

        if !excludes.is_empty() {
            let mut overrides = ignore::overrides::OverrideBuilder::new(path);
            for pattern in excludes {
                overrides
                    .add(&format!("!{pattern}"))
                    .unwrap_or_else(|e| {
                        eprintln!("ffmt: invalid exclude pattern '{pattern}': {e}");
                        process::exit(2);
                    });
            }
            if let Ok(ov) = overrides.build() {
                builder.overrides(ov);
            }
        }

        for entry in builder.build().flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() && is_fortran_file(entry_path) {
                files.push(entry_path.to_path_buf());
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

fn is_fortran_file(path: &Path) -> bool {
    is_fortran_file_ext(path, &crate::config::FilesConfig::default().extensions)
}

fn is_fortran_file_ext(path: &Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| extensions.iter().any(|e| e == ext))
}

struct ProcessOptions {
    check: bool,
    diff: bool,
    color: bool,
    range: Option<(usize, usize)>,
    quiet: bool,
    verbose: bool,
}

fn process_file(
    path: &Path,
    opts: &ProcessOptions,
    config: &crate::config::Config,
) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
            // Binary or invalid UTF-8: not a text Fortran file, skip silently
            return false;
        }
        Err(e) => {
            eprintln!("ffmt: cannot read {}: {e}", path.display());
            return false;
        }
    };

    if opts.verbose {
        eprintln!("ffmt: formatting {}", path.display());
    }

    let formatted = crate::formatter::format_with_config(&source, config, opts.range);

    if formatted == source {
        return false;
    }

    if opts.check || opts.diff {
        if opts.check && !opts.quiet {
            println!("{}", path.display());
        }
        if opts.diff {
            print_diff(path, &source, &formatted, opts.color);
        }
        return true;
    }

    if let Err(e) = fs::write(path, &formatted) {
        eprintln!("ffmt: cannot write {}: {e}", path.display());
    }

    true
}

fn print_diff(path: &Path, original: &str, formatted: &str, color: bool) {
    let (red, green, cyan, reset) = if color {
        ("\x1b[31m", "\x1b[32m", "\x1b[36m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };

    println!("{red}--- {}{reset}", path.display());
    println!("{green}+++ {}{reset}", path.display());

    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();
    let max = orig_lines.len().max(fmt_lines.len());

    let mut i = 0;
    while i < max {
        let orig = orig_lines.get(i).copied().unwrap_or("");
        let fmt = fmt_lines.get(i).copied().unwrap_or("");
        if orig != fmt {
            let hunk_start = i;
            let mut hunk_end = i;
            while hunk_end < max {
                let o = orig_lines.get(hunk_end).copied().unwrap_or("");
                let f = fmt_lines.get(hunk_end).copied().unwrap_or("");
                if o == f && hunk_end > i {
                    break;
                }
                hunk_end += 1;
            }
            println!(
                "{cyan}@@ -{},{} +{},{} @@{reset}",
                hunk_start + 1,
                hunk_end - hunk_start,
                hunk_start + 1,
                hunk_end - hunk_start
            );
            for j in hunk_start..hunk_end {
                if j < orig_lines.len() {
                    println!("{red}-{}{reset}", orig_lines[j]);
                }
            }
            for j in hunk_start..hunk_end {
                if j < fmt_lines.len() {
                    println!("{green}+{}{reset}", fmt_lines[j]);
                }
            }
            i = hunk_end;
        } else {
            i += 1;
        }
    }
}

// --- File hash cache ---
// Stores md5-like hashes of file contents after formatting.
// On subsequent runs, files whose content hash matches the cache are skipped.
// Cache is stored as a simple text file: one "hash path" pair per line.

struct FileCache {
    entries: HashMap<PathBuf, u64>,
}

impl FileCache {
    fn load(cache_dir: &str) -> Self {
        let cache_file = Path::new(cache_dir).join("hashes");
        let mut entries = HashMap::new();

        if let Ok(content) = fs::read_to_string(&cache_file) {
            for line in content.lines() {
                let mut parts = line.splitn(2, ' ');
                if let (Some(hash_str), Some(path_str)) = (parts.next(), parts.next()) {
                    if let Ok(hash) = hash_str.parse::<u64>() {
                        entries.insert(PathBuf::from(path_str), hash);
                    }
                }
            }
        }

        FileCache { entries }
    }

    fn is_cached(&self, path: &Path) -> bool {
        if let Some(cached_hash) = self.entries.get(path) {
            if let Ok(content) = fs::read(path) {
                return hash_bytes(&content) == *cached_hash;
            }
        }
        false
    }

    fn update(&mut self, path: &Path) {
        if let Ok(content) = fs::read(path) {
            self.entries.insert(path.to_path_buf(), hash_bytes(&content));
        }
    }

    fn save(&self, cache_dir: &str) {
        let cache_path = Path::new(cache_dir);
        if fs::create_dir_all(cache_path).is_err() {
            return;
        }

        let cache_file = cache_path.join("hashes");
        let mut lines: Vec<String> = self
            .entries
            .iter()
            .map(|(path, hash)| format!("{hash} {}", path.display()))
            .collect();
        lines.sort();

        let content = lines.join("\n") + "\n";
        let _ = fs::write(cache_file, content);
    }
}

/// Simple FNV-1a hash for file contents. Fast, no crypto needed.
fn hash_bytes(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
