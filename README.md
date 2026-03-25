# ffmt

[![PyPI](https://img.shields.io/pypi/v/ffmt.svg)](https://pypi.org/project/ffmt/)
[![CI](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml/badge.svg)](https://github.com/sbryngelson/ffmt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub Marketplace](https://img.shields.io/badge/Marketplace-ffmt-blue?logo=github)](https://github.com/marketplace/actions/ffmt-fortran-formatter)
[![Docs](https://readthedocs.org/projects/ffmt-fortran/badge/?version=latest)](https://ffmt-fortran.readthedocs.io)

A fast, configurable Fortran formatter with support for Fypp, Doxygen, and OpenACC/OpenMP directives. Written in Rust. Installable via `pip`.

**[Documentation](https://ffmt-fortran.readthedocs.io)** | **[Configuration](https://ffmt-fortran.readthedocs.io/en/latest/configuration/)** | **[Changelog](https://github.com/sbryngelson/ffmt/releases)**

## Installation

```bash
pip install ffmt
```

Or via Cargo:

```bash
cargo install ffmt
```

## Quick start

```bash
ffmt src/                        # format in-place
ffmt --check src/                # CI mode (exit 1 if changes needed)
ffmt --diff src/                 # show colored diff
ffmt -j 8 src/                   # parallel
cat file.fpp | ffmt -            # stdin/stdout
```

## Features

- **Code** -- indentation, whitespace normalization, keyword casing, named ends, line wrapping, operator modernization (`.eq.` -> `==`), double-colon enforcement, trailing semicolon removal
- **Comments** -- rewrapping, inline spacing (S102), `!<` alignment, `!&`/`& !` cleanup, format suppression (`! ffmt off/on`)
- **Structure** -- blank line management around openers/closers/`#ifdef`, declaration `::` alignment, declaration/use compaction
- **Preprocessor** -- Fypp (`#:if`, `$:`, `@:`), C preprocessor (`#ifdef`), OpenACC (`!$acc`), OpenMP (`!$omp`)
- **Opt-in** -- multi-statement splitting, assignment alignment, `&` column alignment, use-statement reformatting

Most options accept `true`, `false`, or `"preserve"`. See the [configuration reference](https://ffmt-fortran.readthedocs.io/en/latest/configuration/) for all options.

## Configuration

Create `ffmt.toml` or add `[tool.ffmt]` to `pyproject.toml`:

```toml
indent-width = 4
line-length = 132
keyword-case = "lower"
modernize-operators = true
enforce-double-colon = true

[whitespace]
relational = true
multdiv = false
```

## CI

```yaml
# GitHub Actions
- uses: sbryngelson/ffmt@latest
  with:
    args: "--check src/"
```

```yaml
# pre-commit
repos:
  - repo: https://github.com/sbryngelson/ffmt
    rev: v0.3.4
    hooks:
      - id: ffmt
```

## Other Fortran formatters

| Formatter | Language | Status | Notes |
|-----------|----------|--------|-------|
| [fprettify](https://github.com/fortran-lang/fprettify) | Python | Unmaintained | Free-form only. Fypp support. |
| [findent](https://github.com/wvermin/findent) | C | Active | Indentation and fixed/free conversion. |
| [Codee Formatter](https://www.codee.com) | Proprietary | Active | Commercial. Tree-sitter based. |
| [LFortran fmt](https://lfortran.org) | Rust | In development | AST-based. |
| [Fortitude](https://github.com/PlasmaFAIR/fortitude) | Rust | Active | Linter with auto-fix. |

## License

MIT
