# ffmt

A fast, configurable Fortran formatter with support for Fypp, Doxygen, and OpenACC/OpenMP directives. Written in Rust.

## Features

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

## Comparison with other formatters

| Formatter | Language | Status | Notes |
|-----------|----------|--------|-------|
| [fprettify](https://github.com/fortran-lang/fprettify) | Python | Unmaintained (last release 2020) | Free-form only. Fypp support. Requires multiple passes for convergence. |
| [findent](https://github.com/wvermin/findent) | C | Active | Indentation and fixed/free conversion. No whitespace normalization. |
| [Codee Formatter](https://www.codee.com) | Proprietary | Active | Commercial. Tree-sitter based. Fixed and free-form. |
| [LFortran fmt](https://lfortran.org) | Rust | In development | AST-based. Part of the LFortran compiler project. |
| [Fortitude](https://github.com/PlasmaFAIR/fortitude) | Rust | Active | Linter with auto-fix, not a full formatter. Tree-sitter based. |
| [f90-mode](https://www.gnu.org/software/emacs/manual/html_node/emacs/Fortran.html) | Emacs Lisp | Active | Emacs built-in. Indentation only. |
