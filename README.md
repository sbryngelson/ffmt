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

- **Indentation** -- scope-based indentation for all Fortran constructs
- **Whitespace** -- consistent spacing around operators, commas, colons, parentheses
- **Keywords** -- case normalization (`IF` -> `if`, `ENDDO` -> `end do`)
- **Continuation lines** -- normalized with proportional re-indentation
- **Preprocessor** -- Fypp (`#:if`, `#:for`), OpenACC (`!$acc`), OpenMP (`!$omp`) handled correctly

### Configuration

Create `ffmt.toml` or add `[tool.ffmt]` to `pyproject.toml`. All options have sensible defaults -- most projects need no config file at all.

#### General

| Option | Default | Description |
|--------|---------|-------------|
| `indent-width` | 4 | Number of spaces per indentation level. |
| `line-length` | 132 | Maximum line length. Code and comments are wrapped at this limit. Set to 1000 to disable wrapping. |
| `keyword-case` | `"lower"` | Case for Fortran keywords: `"lower"`, `"upper"`, or `"preserve"`. |
| `normalize-keywords` | true | Split compound keywords like `enddo` into `end do`. |
| `named-ends` | true | Add procedure/module name to bare `end` statements (`end subroutine` -> `end subroutine s_foo`). |
| `align-declarations` | true | Vertically align `::` in consecutive declaration blocks. |
| `unicode-to-ascii` | true | Replace Unicode Greek letters and math symbols with LaTeX equivalents in comments (`σ` -> `\sigma`). |
| `rewrap-comments` | true | Re-wrap long comments at `line-length`, splitting at word boundaries. Doxygen `!>` / `!!` blocks are joined and re-wrapped as units. |
| `rewrap-code` | true | Re-wrap long code lines at `line-length` using token-aware splitting (never breaks inside numbers or strings). |
| `space-after-comment` | true | Ensure a space after `!` in regular comments (`!comment` -> `! comment`). Does not affect `!$acc`, `!<`, `!>`, `!!`, or `!&`. |
| `collapse-double-spaces` | true | Collapse runs of multiple spaces to a single space in code (not in strings or comments). |
| `keyword-paren-space` | true | Add a space between control-flow keywords and `(` (`if(` -> `if (`). Applies to `if`, `call`, `allocate`, `select case`, `where`, etc. |
| `fypp-list-commas` | true | Normalize comma spacing inside Fypp `'[...]'` list arguments. |
| `indent-fypp` | true | Indent Fypp preprocessor blocks (`#:if`, `#:for`, `#:call`). |
| `indent-module` | true | Indent the body of `module` and `program` blocks. |

#### Whitespace

The `[whitespace]` section controls spacing around specific operators:

| Option | Default | Description |
|--------|---------|-------------|
| `relational` | true | Space around `==`, `/=`, `<`, `<=`, `>`, `>=`. |
| `logical` | true | Space around `.and.`, `.or.`, `.not.`, `.eqv.`, `.neqv.`. |
| `plusminus` | true | Space around binary `+` and `-`. |
| `multdiv` | false | Space around `*` and `/`. |
| `power` | false | Space around `**`. |
| `assignment` | true | Space around `=` in assignments. |
| `pointer` | true | Space around `=>`. |
| `concatenation` | true | Space around `//` (string concatenation). |
| `declaration` | true | Space around `::` in declarations. |
| `comma` | true | Space after `,`. |
| `slice-colon` | false | Space around `:` in array slices. |

#### Files

| Option | Default | Description |
|--------|---------|-------------|
| `extensions` | `["fpp", "f90", "F90", "f95", "f03", "F", "F95", "F03"]` | File extensions to format. |
| `exclude` | `[]` | Glob patterns to exclude. |
| `respect-gitignore` | true | Skip files listed in `.gitignore`. |

#### Example

```toml
indent-width = 4
line-length = 132
keyword-case = "lower"
normalize-keywords = true

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

### Preserved as-is

- String literals and inline expressions (`${...}$`, `@{...}@`)
- Comment contents and Doxygen alignment (`!<`, `!>`, `!!`)
- Continuation line structure across `#ifdef`/`#:if` preprocessor blocks
- `#ifdef`/`#endif` blocks (no indentation change)
- Fypp/Python code inside `$:` and `@:` macro invocations

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
    rev: v0.2.1
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
