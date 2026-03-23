# ffmt

[![PyPI](https://img.shields.io/pypi/v/ffmt.svg)](https://pypi.org/project/ffmt/)
[![CI](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml/badge.svg)](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub Marketplace](https://img.shields.io/badge/Marketplace-ffmt-blue?logo=github)](https://github.com/marketplace/actions/ffmt-fortran-formatter)

A fast, configurable Fortran formatter with support for Fypp, Doxygen, and OpenACC/OpenMP directives. Written in Rust. Installable via `pip`.

## Installation

```bash
pip install ffmt
```

Or via Cargo:

```bash
cargo install ffmt
```

## Usage

```bash
ffmt src/                        # format in-place
ffmt --check src/                # CI mode (exit 1 if changes needed)
ffmt --diff src/                 # show colored diff
ffmt -j 8 src/                   # parallel
cat file.fpp | ffmt -            # stdin/stdout
ffmt --range 10:50 file.fpp      # format only lines 10-50
```

## What it does

### Code formatting
- **Indentation** -- scope-based indentation for all Fortran constructs
- **Whitespace** -- consistent spacing around operators, commas, colons, parentheses
- **Keywords** -- case normalization (`IF` -> `if`, `ENDDO` -> `end do`)
- **Named ends** -- adds procedure/module name to bare `end` statements
- **Continuation lines** -- normalized with proportional re-indentation
- **Line wrapping** -- string-aware rewrapping of long lines (never breaks inside string literals)
- **Trailing semicolons** -- removed (Fortitude S081)
- **Double-colon enforcement** -- adds `::` to declarations missing it (`integer x` -> `integer :: x`)
- **Relational operator modernization** -- converts legacy operators (`.eq.` -> `==`, `.ne.` -> `/=`, etc.)
- **Multi-statement splitting** -- splits `x = 1; y = 2` onto separate lines (opt-in)
- **Assignment alignment** -- aligns `=` across consecutive assignments (opt-in)
- **Use-statement reformatting** -- one import per line for `use, only:` (opt-in)
- **EOL normalization** -- normalize line endings to LF or CRLF

### Comment formatting
- **Comment rewrapping** -- joins and re-wraps `!`/`!!` comment blocks to fill line width
- **Inline comment spacing** -- enforces 2+ spaces before inline `!` comments (Fortitude S102)
- **`!<` alignment** -- aligns inline Doxygen comments within declaration groups (2-space minimum)
- **Comment space** -- ensures space after `!` in regular comments
- **`!&` cleanup** -- strips trailing `!&` no-op comments that cause Cray ftn errors
- **`& !` cleanup** -- converts comment-only continuation lines to standalone comments
- **Format suppression** -- `! ffmt off` / `! ffmt on` to skip formatting for code regions

### Structural formatting
- **Blank lines after openers** -- removes blank lines after `module`, `subroutine`, `if...then`, `do`, `#ifdef`, `#:if`, etc.
- **Blank lines before closers** -- removes blank lines before `end`, `#endif`, `#else`, `else`, `case`, etc.
- **Blank lines after block ends** -- ensures blank line after `end subroutine`/`end function` before next block
- **Declaration separation** -- ensures blank line between declarations and executable code
- **`::` alignment** -- vertically aligns `::` in consecutive declaration blocks
- **Declaration compaction** -- removes blank lines between consecutive declarations
- **Use compaction** -- removes blank lines between consecutive `use` statements
- **Continuation `&` alignment** -- align trailing `&` at column limit (opt-in)

### Preprocessor support
- **Fypp** -- `#:if`, `#:for`, `#:call`, `#:def`, `$:`, `@:` macros handled correctly
- **C preprocessor** -- `#ifdef`, `#ifndef`, `#if`, `#else`, `#endif` preserved
- **OpenACC** -- `!$acc` directives preserved
- **OpenMP** -- `!$omp` directives preserved

## Configuration

Create `ffmt.toml` or add `[tool.ffmt]` to `pyproject.toml`. All options have sensible defaults -- most projects need no config file at all.

Most options accept `true`, `false`, or `"preserve"`. When set to `"preserve"`, ffmt leaves that aspect of the code untouched.

### General

| Option | Default | Description |
|--------|---------|-------------|
| `indent-width` | `4` | Number of spaces per indentation level. |
| `line-length` | `132` | Maximum line length. Code and comments are wrapped at this limit. Set to `1000` to disable wrapping. |
| `keyword-case` | `"lower"` | Case for Fortran keywords: `"lower"`, `"upper"`, or `"preserve"`. |
| `normalize-keywords` | `true` | Split compound keywords like `enddo` into `end do`. Accepts `true`/`false`/`"preserve"`. |
| `named-ends` | `true` | Add procedure/module name to bare `end` statements. Accepts `true`/`false`/`"preserve"`. |
| `align-declarations` | `true` | Vertically align `::` in consecutive declaration blocks. |
| `align-comments` | `true` | Vertically align `!<` inline Doxygen comments in declaration blocks. |
| `compact-declarations` | `true` | Remove blank lines between consecutive declarations. Accepts `true`/`false`/`"preserve"`. |
| `compact-use` | `true` | Remove blank lines between consecutive `use` statements. Accepts `true`/`false`/`"preserve"`. |
| `unicode-to-ascii` | `true` | Replace Unicode Greek letters and math symbols with LaTeX equivalents in comments. |
| `rewrap-comments` | `true` | Re-wrap long comments at `line-length`. Doxygen `!>`/`!!` blocks are joined and re-wrapped as units. Accepts `true`/`false`/`"preserve"`. |
| `rewrap-code` | `true` | Re-wrap long code lines at `line-length`. Accepts `true`/`false`/`"preserve"`. |
| `space-after-comment` | `true` | Ensure a space after `!` in regular comments. Accepts `true`/`false`/`"preserve"`. |
| `collapse-double-spaces` | `true` | Collapse runs of multiple spaces to a single space in code. |
| `keyword-paren-space` | `true` | Add a space between keywords and `(` (`if(` -> `if (`). Accepts `true`/`false`/`"preserve"`. |
| `fypp-list-commas` | `true` | Normalize comma spacing inside Fypp `'[...]'` list arguments. Accepts `true`/`false`/`"preserve"`. |
| `indent-fypp` | `true` | Indent Fypp preprocessor blocks (`#:if`, `#:for`, `#:call`). |
| `indent-module` | `true` | Indent the body of `module` and `program` blocks. |
| `end-of-line` | `"lf"` | Line ending normalization: `"lf"`, `"crlf"`, or `"preserve"`. |
| `modernize-operators` | `true` | Convert legacy relational operators to modern form (`.eq.` -> `==`). Accepts `true`/`false`/`"preserve"`. |
| `enforce-double-colon` | `true` | Add `::` to declarations missing it (`integer x` -> `integer :: x`). Accepts `true`/`false`/`"preserve"`. |
| `split-statements` | `false` | Split semicolon-separated statements onto separate lines. Preserves `private; public ::`. Accepts `true`/`false`/`"preserve"`. |
| `align-ampersand` | `false` | Align trailing `&` continuation markers at column limit. Accepts `true`/`false`/`"preserve"`. |
| `align-assignments` | `false` | Align `=` in consecutive assignment statements. Accepts `true`/`false`/`"preserve"`. |
| `use-formatting` | `false` | Reformat `use, only:` imports to one per line. Accepts `true`/`"one-per-line"`/`false`/`"preserve"`. |

### Whitespace

The `[whitespace]` section controls spacing around specific operators. All accept `true`, `false`, or `"preserve"`:

| Option | Default | Description |
|--------|---------|-------------|
| `relational` | `true` | Space around `==`, `/=`, `<`, `<=`, `>`, `>=`. |
| `logical` | `true` | Space around `.and.`, `.or.`, `.not.`, `.eqv.`, `.neqv.`. |
| `plusminus` | `true` | Space around binary `+` and `-`. |
| `multdiv` | `false` | Space around `*` and `/`. |
| `power` | `false` | Space around `**`. |
| `assignment` | `true` | Space around `=` in assignments. |
| `pointer` | `true` | Space around `=>`. |
| `concatenation` | `true` | Space around `//` (string concatenation). |
| `declaration` | `true` | Space around `::` in declarations. |
| `comma` | `true` | Space after `,`. |
| `slice-colon` | `false` | Space around `:` in array slices. |

### Files

| Option | Default | Description |
|--------|---------|-------------|
| `extensions` | `["fpp", "f90", "F90", "f95", "f03", "F", "F95", "F03"]` | File extensions to format. |
| `exclude` | `[]` | Glob patterns to exclude. |
| `respect-gitignore` | `true` | Skip files listed in `.gitignore`. |

### Example

```toml
indent-width = 4
line-length = 132
keyword-case = "lower"
normalize-keywords = true
rewrap-comments = true
modernize-operators = true
enforce-double-colon = true
end-of-line = "lf"

# Opt-in features (default: false)
# split-statements = true
# align-ampersand = true
# align-assignments = true
# use-formatting = "one-per-line"

[whitespace]
relational = true
logical = true
plusminus = true
multdiv = false
power = false
assignment = true
declaration = true
comma = true
```

### Format suppression

Disable formatting for specific code regions with `! ffmt off` and `! ffmt on`:

```fortran
! ffmt off
x    =    1       ! preserved exactly as written
y=2;z=3
! ffmt on
w = 4              ! formatted normally
```

### Preserved as-is

- Code between `! ffmt off` and `! ffmt on` markers
- String literals and inline expressions (`${...}$`, `@{...}@`)
- Doxygen comment structure (`!<`, `!>`, `!!` associations)
- Continuation line structure across `#ifdef`/`#:if` preprocessor blocks
- `#ifdef`/`#endif` blocks (no indentation change)
- Fypp/Python code inside `$:` and `@:` macro invocations
- `!$acc` and `!$omp` directive lines

## Editor integration

ffmt includes a built-in LSP server for real-time format-on-save:

```bash
ffmt --lsp
```

### VS Code

```json
{
    "fortran.formatting.formatter": "ffmt",
    "fortran.formatting.args": ["--stdin-filepath", "${file}", "-"]
}
```

### Vim/Neovim

```vim
autocmd BufWritePost *.fpp,*.f90 silent !ffmt %
" Or: set formatprg=ffmt\ -
```

### Neovim (LSP)

```lua
vim.lsp.start({
    name = "ffmt",
    cmd = { "ffmt", "--lsp" },
    root_dir = vim.fs.dirname(vim.fs.find({ "ffmt.toml", "pyproject.toml" }, { upward = true })[1]),
})
```

## CI integration

### GitHub Actions

```yaml
- uses: sbryngelson/ffmt@latest
  with:
    args: "--check src/"
```

### pre-commit

```yaml
repos:
  - repo: https://github.com/sbryngelson/ffmt
    rev: v0.2.8
    hooks:
      - id: ffmt
```

To auto-update to the latest version, run:

```bash
pre-commit autoupdate
```

Or use [pre-commit.ci](https://pre-commit.ci/) to update hooks automatically via pull requests.

## Other Fortran Formatters

| Formatter | Language | Status | Notes |
|-----------|----------|--------|-------|
| [fprettify](https://github.com/fortran-lang/fprettify) | Python | Unmaintained (last release 2020) | Free-form only. Fypp support. Requires multiple passes for convergence. |
| [findent](https://github.com/wvermin/findent) | C | Active | Indentation and fixed/free conversion. No whitespace normalization. |
| [Codee Formatter](https://www.codee.com) | Proprietary | Active | Commercial. Tree-sitter based. Fixed and free-form. |
| [LFortran fmt](https://lfortran.org) | Rust | In development | AST-based. Part of the LFortran compiler project. |
| [Fortitude](https://github.com/PlasmaFAIR/fortitude) | Rust | Active | Linter with auto-fix, not a full formatter. Tree-sitter based. |
| [f90-mode](https://www.gnu.org/software/emacs/manual/html_node/emacs/Fortran.html) | Emacs Lisp | Active | Emacs built-in. Indentation only. |

## License

MIT
