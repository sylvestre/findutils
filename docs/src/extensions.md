# Extensions

The goal of this project is to be a drop-in replacement for GNU findutils, so by
default the tools behave exactly like the originals. A few opt-in extras go
beyond what GNU offers; they are all off unless you ask for them, and none of
them change the default output.

The two below are the only ones. Nothing else here is an addition to GNU: every
other predicate, option, `-printf` directive and environment variable either
matches GNU or is still missing. In particular `LOCATE_PATH`, `FINDOPTIONS`,
`PRUNEPATHS`, `PRUNEFS`, `NETPATHS`, `LOCALUSER` and `NETUSER` are standard
findutils variables, not extensions. Where we differ from GNU it is a gap rather
than an extra — see [GNU test coverage](test_coverage.md).

## `-sorted`: deterministic traversal order

GNU `find` returns entries in whatever order the filesystem hands them back, so
two runs over the same tree can print the same lines in a different order. The
`-sorted` predicate makes `find` sort each directory's entries by name before
descending:

```console
$ find /srv/data -type f -printf '%f\n'
yankee
bravo
mike
alpha
zeta

$ find /srv/data -sorted -type f -printf '%f\n'
alpha
bravo
mike
yankee
zeta
```

It is a global flag rather than a test: it always matches, and it affects the
whole traversal no matter where it appears in the expression. Sorting requires
reading each directory in full before descending into it, so it costs memory and
latency on very large directories — that is why it is opt-in rather than the
default.

This is most useful when you want reproducible output: comparing two trees,
generating a manifest, or writing a test whose expected output is a fixed list
of lines.

## `UU_DIAG`: rich expression diagnostics

`find` expressions get long, and a lone error line does not say *where* in the
expression the problem is:

```console
$ find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
```

Set `UU_DIAG` to a non-empty value other than `0` and `find` underlines the
argument at fault, in place, on the command line you actually typed:

```console
$ UU_DIAG=1 find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
   ╭─[ command line:1:66 ]
   │
 1 │ find /srv/www -type f -a '(' -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' ')' -print
   │                                                                  ──┬──
   │                                                                    ╰──── not a known predicate
   │
   │ Help: did you mean `-name'?
───╯
```

The first line is the message `find` has always printed, so anything that matched
it before still matches. Everything below it is added context.

### What gets a diagnostic

Errors in the *structure* of the expression, and in the predicates themselves.
The unknown-predicate case is shown above; the rest follow, all with `UU_DIAG`
set in the environment.

A predicate left without its argument:

```console
$ find /home -xdev -type f -size +1G -a -mtime -7 -printf
find: missing argument to -printf
   ╭─[ command line:1:49 ]
   │
 1 │ find /home -xdev -type f -size +1G -a -mtime -7 -printf
   │                                                 ───┬───
   │                                                    ╰───── this predicate needs an argument
───╯
```

An operator with nothing *after* it, typically a half-finished edit:

```console
$ find /var/spool -type f -a \( -user postfix -o -group mail \) -a -mtime +7 -o
find: expected an expression after -o
   ╭─[ command line:1:78 ]
   │
 1 │ find /var/spool -type f -a '(' -user postfix -o -group mail ')' -a -mtime +7 -o
   │                                                                              ─┬
   │                                                                               ╰── nothing follows this operator
───╯
```

An operator with nothing *before* it. Note this underlines the leading token,
where the previous example underlined the trailing one — two mistakes that read
identically in the plain message now look different:

```console
$ find . -o -type f -name '*.tmp' -print
find: invalid expression; you have used a binary operator '-o' with nothing before it.
   ╭─[ command line:1:8 ]
   │
 1 │ find . -o -type f -name '*.tmp' -print
   │        ─┬
   │         ╰── no expression before this operator
───╯
```

A `(` that is never closed. There are three `(` on this line; the diagnostic
picks the unbalanced one rather than pointing at the end of the command:

```console
$ find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
   ╭─[ command line:1:72 ]
   │
 1 │ find . '(' -type d -a '(' -name .git -o -name target ')' -prune ')' -o '(' -type f -print
   │                                                                        ─┬─
   │                                                                         ╰─── this parenthesis is never closed
───╯
```

An empty group:

```console
$ find . -type f \( \) -o -name '*.bak' -print
find: invalid expression; empty parentheses are not allowed.
   ╭─[ command line:1:16 ]
   │
 1 │ find . -type f '(' ')' -o -name '*.bak' -print
   │                ─┬─
   │                 ╰─── nothing between these parentheses
───╯
```

And a `)` with no opener:

```console
$ find /etc \( -name '*.conf' -a -newer /etc/fstab \) \) -o -name '*.cfg' -print
find: invalid expression: expected expression before closing parentheses ')'.
   ╭─[ command line:1:55 ]
   │
 1 │ find /etc '(' -name '*.conf' -a -newer /etc/fstab ')' ')' -o -name '*.cfg' -print
   │                                                       ─┬─
   │                                                        ╰─── no matching '(' before this
───╯
```

Errors in the *value* of an argument — a bad `-size` suffix, `-perm` mode,
`-type` list, `-printf` format or `-newerXY` date — keep the plain single-line
message for now.

### Comparison with GNU

GNU `find` reports the same errors, but only ever as a single line. On a typo
buried in a group, that is all you get:

```console
$ find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
```

```console
$ UU_DIAG=1 find /srv/www -type f -a \( -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' \) -print
find: unknown predicate `-nmae'
   ╭─[ command line:1:66 ]
   │
 1 │ find /srv/www -type f -a '(' -name '*.php' -o -name '*.phtml' -o -nmae '*.inc' ')' -print
   │                                                                  ──┬──
   │                                                                    ╰──── not a known predicate
   │
   │ Help: did you mean `-name'?
───╯
```

The difference is starkest where the message names no argument at all. GNU tells
you a `)` is missing, but not which `(` is unbalanced — on a line with three of
them, that is the whole question:

```console
$ find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
```

```console
$ UU_DIAG=1 find . \( -type d -a \( -name .git -o -name target \) -prune \) -o \( -type f -print
find: invalid expression; I was expecting to find a ')' somewhere but did not see one.
   ╭─[ command line:1:72 ]
   │
 1 │ find . '(' -type d -a '(' -name .git -o -name target ')' -prune ')' -o '(' -type f -print
   │                                                                        ─┬─
   │                                                                         ╰─── this parenthesis is never closed
───╯
```

In both cases the first line is identical to what GNU 4.10.0 prints, and the exit
code is `1` either way. `UU_DIAG` only ever *adds* the block below.

That holds for four of the seven messages, which are byte-for-byte identical to
GNU's under `LC_ALL=C`:

```text
unknown predicate `-nmae'
invalid expression; you have used a binary operator '-o' with nothing before it.
invalid expression; empty parentheses are not allowed.
invalid expression; I was expecting to find a ')' somewhere but did not see one.
```

The remaining three differ. These are compatibility gaps in the message text
itself — they predate `UU_DIAG` and are unaffected by it, since the variable
adds a block below whichever message is printed rather than changing it:

| Expression | uutils `find` | GNU `find` 4.10.0 |
| --- | --- | --- |
| `find . -printf` | `missing argument to -printf` | ``missing argument to `-printf'`` |
| `find . -true -o` | `expected an expression after -o` | `expected an expression after '-o'` |
| `find . -true \)` | `invalid expression: expected expression before closing parentheses ')'.` | `you have too many ')'` |

The first two differ only in the quoting around the predicate name; the third is
different wording for the same condition.

### Notes

- **The rendered line is a reconstruction, not an echo.** Arguments are re-quoted
  so the line is unambiguous and can be pasted back into a shell: `\(` shows up
  as `'('`, and globs the shell never expanded stay quoted. That is what makes the
  underline trustworthy even when the shell rewrote what you typed.
- **Suggestions are edit-distance based**, with a threshold that scales with the
  length of the name (one edit up to 3 characters, two up to 7, three beyond).
  Transpositions and dropped letters are caught — `-nam`, `-pritn`, `-exce`,
  `-mindpeth` — while genuinely unrelated input gets no suggestion rather than a
  misleading one.
- **Colour follows [`NO_COLOR`](https://no-color.org)** and is used only when
  standard error is a terminal. The diagnostic itself is *not* gated on being a
  terminal, so it still works when piping to a pager or a file.
- **Nothing changes when `UU_DIAG` is unset**, which is how the GNU and bfs
  compatibility testsuites run. Standard error stays byte-for-byte what it was.
