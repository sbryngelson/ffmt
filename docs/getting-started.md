# Getting Started

## Installation

=== "pip"
    ```bash
    pip install ffmt
    ```

=== "Cargo"
    ```bash
    cargo install ffmt
    ```

=== "From source"
    ```bash
    git clone https://github.com/sbryngelson/ffmt
    cd ffmt
    cargo install --path .
    ```

## Usage

```bash
ffmt src/                        # format in-place
ffmt --check src/                # CI mode (exit 1 if changes needed)
ffmt --diff src/                 # show colored diff
ffmt -j 8 src/                   # parallel
cat file.fpp | ffmt -            # stdin/stdout
ffmt --range 10:50 file.fpp      # format only lines 10-50
ffmt --no-cache src/             # ignore file modification cache
```

## Quick start

ffmt works out of the box with no configuration. Just run it on your source directory:

```bash
ffmt src/
```

To check formatting without modifying files (useful for CI):

```bash
ffmt --check src/
```

## Configuration

Create `ffmt.toml` in your project root, or add `[tool.ffmt]` to `pyproject.toml`. See [Configuration](configuration.md) for all options.

Most projects need no config file -- the defaults are designed for modern Fortran with Fypp preprocessing.

## Format suppression

Disable formatting for specific code regions:

```fortran
! ffmt off
x    =    1       ! preserved exactly as written
y=2;z=3
! ffmt on
w = 4              ! formatted normally
```

## What gets preserved

ffmt never modifies:

- Code between `! ffmt off` and `! ffmt on` markers
- String literals and inline expressions (`${...}$`, `@{...}@`)
- Doxygen comment structure (`!<`, `!>`, `!!` associations)
- Continuation line structure across `#ifdef`/`#:if` preprocessor blocks
- `#ifdef`/`#endif` blocks (no indentation change)
- Fypp/Python code inside `$:` and `@:` macro invocations
- `!$acc` and `!$omp` directive lines
