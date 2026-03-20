# ffmt

[![PyPI](https://img.shields.io/pypi/v/ffmt.svg)](https://pypi.org/project/ffmt/)
[![CI](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml/badge.svg)](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub Marketplace](https://img.shields.io/badge/Marketplace-ffmt-blue?logo=github)](https://github.com/marketplace/actions/ffmt-fortran-formatter)

A fast, configurable Fortran formatter. Written in Rust. Installable via `pip`.

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

Create `ffmt.toml` or add `[tool.ffmt]` to `pyproject.toml`:

```toml
indent-width = 4
keyword-case = "lower"     # lower, upper, preserve
normalize-keywords = true  # enddo -> end do

[whitespace]
relational = true          # ==, /=, <, <=, >, >=
logical = true             # .and., .or., .not.
plusminus = true            # binary +, -
multdiv = false             # *, /
power = false               # **
assignment = true           # =
declaration = true          # ::
comma = true                # space after comma
```

All options have sensible defaults -- most projects need no config file at all.

### Preserved as-is

- String literals and inline expressions (`${...}$`, `@{...}@`)
- Comment contents and Doxygen alignment (`!<`, `!>`, `!!`)
- Continuation line structure
- `#ifdef`/`#endif` blocks (no indentation change)

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
- uses: sbryngelson/ffmt@v0.1.0
  with:
    args: "--check src/"
```

### pre-commit

```yaml
repos:
  - repo: https://github.com/sbryngelson/ffmt
    rev: v0.1.0
    hooks:
      - id: ffmt
```

## License

MIT
