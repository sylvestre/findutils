# findutils

[![Crates.io](https://img.shields.io/crates/v/findutils.svg)](https://crates.io/crates/findutils)
[![Discord](https://img.shields.io/badge/discord-join-7289DA.svg?logo=discord&longCache=true&style=flat)](https://discord.gg/wQVJbvJ)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/findutils/blob/main/LICENSE)
[![dependency status](https://deps.rs/repo/github/uutils/findutils/status.svg)](https://deps.rs/repo/github/uutils/findutils)
[![codecov](https://codecov.io/gh/uutils/findutils/branch/master/graph/badge.svg)](https://codecov.io/gh/uutils/findutils)

Rust implementation of [GNU findutils](https://www.gnu.org/software/findutils/): `xargs`, `find`, `locate` and `updatedb`.
The goal is to be a full drop-in replacement of the original commands.

## Expression diagnostics

`find` expressions get long, and a plain `find: unknown predicate '-nmae'` does not
say *where* in the expression the problem is. Setting `UU_DIAG` to a non-empty value
(other than `0`) makes `find` underline the offending argument:

```
$ UU_DIAG=1 find /srv -type f -a \( -name '*.rs' -o -nmae '*.toml' \) -print
find: unknown predicate `-nmae'
   ╭─[ command line:1:47 ]
   │
 1 │ find /srv -type f -a '(' -name '*.rs' -o -nmae '*.toml' ')' -print
   │                                          ──┬──
   │                                            ╰──── not a known predicate
   │
   │ Help: did you mean `-name'?
───╯
```

This is off by default, so the usual output stays byte-for-byte compatible with GNU
`find`. Colour follows the [`NO_COLOR`](https://no-color.org) convention and is only
used when stderr is a terminal.

## Run the GNU testsuite on rust/findutils:

```
bash util/build-gnu.sh

# To run a specific test:
bash util/build-gnu.sh tests/misc/help-version.sh
```

## Comparing with GNU

![Evolution over time - GNU testsuite](https://github.com/uutils/findutils-tracking/blob/main/gnu-results.svg?raw=true)
![Evolution over time - BFS testsuite](https://github.com/uutils/findutils-tracking/blob/main/bfs-results.svg?raw=true)

## Build/run with BFS

[bfs](https://github.com/tavianator/bfs) is a variant of the UNIX find command that operates breadth-first rather than depth-first.

```
bash util/build-bfs.sh

# To run a specific test:
bash util/build-bfs.sh posix/basic
```

For more details, see https://github.com/uutils/findutils-tracking/
