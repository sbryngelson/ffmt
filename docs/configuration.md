# Configuration

Create `ffmt.toml` in your project root, or add `[tool.ffmt]` to `pyproject.toml`. Config is searched upward from each formatted file's directory.

!!! tip
    Use `ffmt --dump-config` to see the resolved configuration for the current directory.

## Toggle values

Most options accept three values:

| Value | Meaning |
|-------|---------|
| `true` | Actively apply the rule |
| `false` | Actively suppress the rule |
| `"preserve"` | Don't touch -- leave as-is |

---

## General options

### `indent-width` (integer)

Number of spaces per indentation level.

**Default:** `4`

```fortran
! indent-width = 2              ! indent-width = 4
subroutine s_foo()              subroutine s_foo()
  if (.true.) then                  if (.true.) then
    x = 1                              x = 1
  end if                            end if
end subroutine                  end subroutine
```

### `line-length` (integer)

Maximum line length. Code and comments are wrapped at this limit. Set to `1000` to effectively disable wrapping.

**Default:** `132`

### `keyword-case` (`"lower"` | `"upper"` | `"preserve"`)

Case normalization for Fortran keywords.

**Default:** `"lower"`

```fortran
! "lower"                       ! "upper"
if (x == 1) then                IF (x == 1) THEN
    call s_foo()                    CALL s_foo()
end if                          END IF
```

### `normalize-keywords` (toggle)

Split compound keywords into their canonical two-word form.

**Default:** `true`

```fortran
! true                          ! false
end do                          enddo
end if                          endif
end subroutine s_foo            endsubroutine s_foo
```

### `named-ends` (toggle)

Add procedure/module name to bare `end` statements.

**Default:** `true`

```fortran
! true                          ! false
end subroutine s_foo            end subroutine
end function f_bar              end function
end module m_baz                end module
```

### `enforce-double-colon` (toggle)

Add `::` separator to variable declarations that are missing it.

**Default:** `true`

```fortran
! true                          ! false (preserve)
integer :: x                    integer x
real(wp) :: y                   real(wp) y
type(foo) :: bar                type(foo) bar
```

Does not modify function/subroutine signatures (`logical function f_foo` is left alone).

### `modernize-operators` (toggle)

Convert legacy Fortran relational operators to modern symbolic form. String-aware -- does not modify operators inside string literals or comments.

**Default:** `true`

```fortran
! true                          ! false (preserve)
if (x == 1) then                if (x .eq. 1) then
if (y /= 0) then               if (y .ne. 0) then
if (z < 10) then               if (z .lt. 10) then
if (w >= 5) then               if (w .ge. 5) then
s = 'x .eq. y'                 s = 'x .eq. y'   ! strings preserved
```

### `split-statements` (toggle)

Split semicolon-separated statements onto separate lines. The `private; public ::` idiom is preserved.

**Default:** `false`

```fortran
! true                          ! false (preserve)
x = 1                           x = 1; y = 2; z = 3
y = 2
z = 3
private; public :: s_foo        private; public :: s_foo  ! preserved
```

### `rewrap-comments` (toggle)

Re-wrap long `!` and `!!` comment blocks at `line-length`. Doxygen `!>` / `!!` blocks are joined and re-wrapped as units. Short consecutive comment lines are merged.

**Default:** `true`

```fortran
! true (line-length = 80)
! This is a long comment that was originally on a single very long line but has
! been wrapped to fit within the line length limit.

! false (preserve)
! This is a long comment that was originally on a single very long line but has been wrapped to fit within the line length limit.
```

### `rewrap-code` (toggle)

Re-wrap long code lines at `line-length` using token-aware splitting. Never breaks inside string literals.

**Default:** `true`

```fortran
! true
call s_long_subroutine_name(arg1, arg2, arg3, &
                            & arg4, arg5)

! false (preserve)
call s_long_subroutine_name(arg1, arg2, arg3, arg4, arg5)
```

### `align-declarations` (boolean)

Vertically align `::` in consecutive declaration blocks.

**Default:** `true`

```fortran
! true                          ! false
integer                :: x     integer :: x
real(wp), dimension(3) :: y     real(wp), dimension(3) :: y
logical                :: z     logical :: z
```

### `align-comments` (boolean)

Vertically align `!<` inline Doxygen comments in declaration blocks. Enforces a minimum 2-space gap before the comment.

**Default:** `true`

```fortran
! true
integer :: x  !< First variable
real(wp) :: y !< Second variable with long type

! becomes:
integer  :: x  !< First variable
real(wp) :: y  !< Second variable with long type
```

### `align-assignments` (toggle)

Align `=` signs across consecutive assignment statements.

**Default:** `false`

```fortran
! true                          ! false
x        = 1                    x = 1
long_var = 2                    long_var = 2
y        = 3                    y = 3
```

### `align-ampersand` (toggle)

Align trailing `&` continuation markers at the column limit.

**Default:** `false`

```fortran
! true (line-length = 40)
call s_foo(x,                          &
           & y, z)

! false
call s_foo(x, &
           & y, z)
```

### `compact-declarations` (toggle)

Remove blank lines between consecutive declarations within the same scope.

**Default:** `true`

### `compact-use` (toggle)

Remove blank lines between consecutive `use` statements.

**Default:** `true`

### `use-formatting` (toggle)

Reformat `use, only:` imports to one item per line. Accepts `true`, `"one-per-line"`, `false`, or `"preserve"`.

**Default:** `false`

```fortran
! true / "one-per-line"
use m_foo, only: &
    & s_bar, &
    & s_baz, &
    & f_qux

! false (preserve)
use m_foo, only: s_bar, s_baz, f_qux
```

### `end-of-line` (`"lf"` | `"crlf"` | `"preserve"`)

Normalize line endings.

**Default:** `"lf"`

### `space-after-comment` (toggle)

Ensure a space after `!` in regular comments (`!comment` -> `! comment`). Does not affect `!$acc`, `!<`, `!>`, `!!`, or `!&`.

**Default:** `true`

### `collapse-double-spaces` (boolean)

Collapse runs of multiple spaces to a single space in code (not in strings or comments).

**Default:** `true`

### `keyword-paren-space` (toggle)

Add a space between control-flow keywords and `(`.

**Default:** `true`

```fortran
! true                          ! false
if (x == 1) then                if(x == 1) then
call s_foo()                    call s_foo()
allocate (x(n))                 allocate(x(n))
```

### `unicode-to-ascii` (boolean)

Replace Unicode Greek letters and math symbols with LaTeX equivalents in comments.

**Default:** `true`

### `fypp-list-commas` (toggle)

Normalize comma spacing inside Fypp `'[...]'` list arguments.

**Default:** `true`

### `indent-fypp` (boolean)

Indent Fypp preprocessor blocks (`#:if`, `#:for`, `#:call`).

**Default:** `true`

### `indent-module` (boolean)

Indent the body of `module` and `program` blocks.

**Default:** `true`

---

## Whitespace options

The `[whitespace]` section controls spacing around specific operators. All accept `true`, `false`, or `"preserve"`.

### `relational` (toggle)

Space around `==`, `/=`, `<`, `<=`, `>`, `>=`.

**Default:** `true`

```fortran
! true                          ! false
if (x == 1) then                if (x==1) then
```

### `logical` (toggle)

Space around `.and.`, `.or.`, `.not.`, `.eqv.`, `.neqv.`.

**Default:** `true`

```fortran
! true                          ! false
if (a .and. b) then             if (a.and.b) then
```

### `plusminus` (toggle)

Space around binary `+` and `-`.

**Default:** `true`

```fortran
! true                          ! false
x = a + b - c                  x = a+b-c
```

### `multdiv` (toggle)

Space around `*` and `/`.

**Default:** `false`

```fortran
! true                          ! false
x = a * b / c                  x = a*b/c
```

### `power` (toggle)

Space around `**`.

**Default:** `false`

```fortran
! true                          ! false
x = a ** 2                     x = a**2
```

### `assignment` (toggle)

Space around `=` in assignments.

**Default:** `true`

```fortran
! true                          ! false
x = 1                           x=1
```

### `pointer` (toggle)

Space around `=>`.

**Default:** `true`

### `concatenation` (toggle)

Space around `//` (string concatenation).

**Default:** `true`

```fortran
! true                          ! false
s = 'hello' // ' world'        s = 'hello'//' world'
```

### `declaration` (toggle)

Space around `::` in declarations.

**Default:** `true`

```fortran
! true                          ! false
integer :: x                    integer::x
```

### `comma` (toggle)

Space after `,`.

**Default:** `true`

```fortran
! true                          ! false
call s_foo(x, y, z)            call s_foo(x,y,z)
```

### `slice-colon` (toggle)

Space around `:` in array slices.

**Default:** `false`

```fortran
! true                          ! false
x(1 : n)                       x(1:n)
```

---

## File options

### `extensions` (list of strings)

File extensions to treat as Fortran source.

**Default:** `["fpp", "f90", "F90", "f95", "f03", "F", "F95", "F03"]`

### `exclude` (list of strings)

Glob patterns to exclude from formatting.

**Default:** `[]`

```toml
[files]
exclude = ["generated/**", "third_party/**"]
```

### `respect-gitignore` (boolean)

Skip files listed in `.gitignore`.

**Default:** `true`

---

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
