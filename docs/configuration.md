# Configuration

Create `ffmt.toml` in your project root, or add `[tool.ffmt]` to `pyproject.toml`. Config is searched upward from each formatted file's directory.

## Toggle values

Most options accept three values:

| Value | Meaning |
|-------|---------|
| `true` | Actively apply the rule |
| `false` | Actively suppress the rule |
| `"preserve"` | Don't touch -- leave as-is |

## General options

| Option | Default | Description |
|--------|---------|-------------|
| `indent-width` | `4` | Number of spaces per indentation level. |
| `line-length` | `132` | Maximum line length. Code and comments are wrapped at this limit. Set to `1000` to disable wrapping. |
| `keyword-case` | `"lower"` | Case for Fortran keywords: `"lower"`, `"upper"`, or `"preserve"`. |
| `normalize-keywords` | `true` | Split compound keywords like `enddo` into `end do`. |
| `named-ends` | `true` | Add procedure/module name to bare `end` statements. |
| `align-declarations` | `true` | Vertically align `::` in consecutive declaration blocks. |
| `align-comments` | `true` | Vertically align `!<` inline Doxygen comments in declaration blocks. |
| `compact-declarations` | `true` | Remove blank lines between consecutive declarations. |
| `compact-use` | `true` | Remove blank lines between consecutive `use` statements. |
| `unicode-to-ascii` | `true` | Replace Unicode Greek letters and math symbols with LaTeX equivalents in comments. |
| `rewrap-comments` | `true` | Re-wrap long comments at `line-length`. Doxygen `!>`/`!!` blocks are joined and re-wrapped as units. |
| `rewrap-code` | `true` | Re-wrap long code lines at `line-length`. |
| `space-after-comment` | `true` | Ensure a space after `!` in regular comments. |
| `collapse-double-spaces` | `true` | Collapse runs of multiple spaces to a single space in code. |
| `keyword-paren-space` | `true` | Add a space between keywords and `(` (`if(` -> `if (`). |
| `fypp-list-commas` | `true` | Normalize comma spacing inside Fypp `'[...]'` list arguments. |
| `indent-fypp` | `true` | Indent Fypp preprocessor blocks (`#:if`, `#:for`, `#:call`). |
| `indent-module` | `true` | Indent the body of `module` and `program` blocks. |
| `end-of-line` | `"lf"` | Line ending normalization: `"lf"`, `"crlf"`, or `"preserve"`. |
| `modernize-operators` | `true` | Convert legacy relational operators to modern form (`.eq.` -> `==`). |
| `enforce-double-colon` | `true` | Add `::` to declarations missing it (`integer x` -> `integer :: x`). |
| `split-statements` | `false` | Split semicolon-separated statements onto separate lines. Preserves `private; public ::`. |
| `align-ampersand` | `false` | Align trailing `&` continuation markers at column limit. |
| `align-assignments` | `false` | Align `=` in consecutive assignment statements. |
| `use-formatting` | `false` | Reformat `use, only:` imports to one per line. Accepts `true`, `"one-per-line"`, `false`, or `"preserve"`. |

## Whitespace options

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

## File options

| Option | Default | Description |
|--------|---------|-------------|
| `extensions` | `["fpp", "f90", "F90", "f95", "f03", "F", "F95", "F03"]` | File extensions to format. |
| `exclude` | `[]` | Glob patterns to exclude. |
| `respect-gitignore` | `true` | Skip files listed in `.gitignore`. |

## Example configurations

### Minimal (MFC-style)

```toml
indent-width = 2
line-length = 132
keyword-case = "lower"
```

### Strict modernization

```toml
indent-width = 4
line-length = 132
keyword-case = "lower"
modernize-operators = true
enforce-double-colon = true
split-statements = true
use-formatting = "one-per-line"
```

### Preserve-heavy (minimal changes)

```toml
keyword-case = "preserve"
normalize-keywords = "preserve"
rewrap-comments = "preserve"
rewrap-code = "preserve"
modernize-operators = false
enforce-double-colon = false

[whitespace]
relational = "preserve"
logical = "preserve"
plusminus = "preserve"
assignment = "preserve"
```

### pyproject.toml

```toml
[tool.ffmt]
indent-width = 4
line-length = 132
keyword-case = "lower"

[tool.ffmt.whitespace]
relational = true
multdiv = false
```
