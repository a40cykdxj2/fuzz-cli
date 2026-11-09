# fz

A small terminal fuzzy file finder that shows a preview of the highlighted file
next to the list, so you can decide before you quit.

```text
 fz > scan                                      3 / 3 match
> src/scan.rs                    | use std::fs;
  src/score.rs                   | use std::path::PathBuf;
  docs/scanning.md               | # Scanning
```

## Why

Fuzzy finders are fast at narrowing a list and useless at answering the actual
question: is this the file I meant? Opening each candidate to look costs a
quit-and-rerun cycle. `fz` renders the first lines of the selected file in a
side pane instead, on the same keystroke that moves the selection, so the
choice is made in one pass. Previewing is cheap: at most 16 KiB is read per
file and binary files are reported as such rather than sprayed over the
terminal.

The tool is deliberately one screen and no config. Directory walk order is
stable, `Esc` aborts, and the chosen path goes to stdout so `fz` composes with
`$EDITOR`, `xargs` and shell loops.

## Install

From a checkout:

```sh
cargo install --path .
```

That puts `fz` on `$PATH`. Building without installing also works:

```sh
cargo build --release && ./target/release/fz
```

Requires a POSIX terminal (`stty`) and Rust 1.74 or newer.

## Usage

```sh
fz                      # browse the current directory
fz src                  # start from a subdirectory
fz -d 4 src             # cap the walk at four levels
fz --no-preview ~/code  # narrow layout for small terminals

file=$(fz) && ${EDITOR:-vi} "$file"
```

Keys: type to filter, `C-n` / `C-p` or the arrow keys move, `Enter` prints the
path, `C-u` clears the query, `Esc` or `C-c` aborts.

When stdin is not a terminal, one query line is read and up to 20 matches are
printed, which makes the tool usable in pipelines and editors:

```sh
printf 'handler\n' | fz src
```

### Exit codes

| code | meaning                     |
|------|-----------------------------|
| 0    | a path was printed          |
| 1    | no match, or aborted        |
| 2    | usage error                 |
| 3    | I/O error                   |

Scripts should test for 0 rather than for output, because an abort and an empty
result are both normal.

### Completions

```sh
fz --completions bash > /etc/bash_completion.d/fz
fz --completions zsh  > ~/.zsh/completions/_fz
fz --completions fish > ~/.config/fish/completions/fz.fish
```

## Layout

```text
src/main.rs         argument parsing, key loop, drawing
src/render/         pane geometry, scrolling window, match highlighting
src/scan.rs         breadth-first directory walk
src/score.rs        fuzzy subsequence scoring and ranking
src/preview.rs      file sample loading and line clipping
src/term.rs         raw mode, size query, key decoding
src/completions.rs  static completion scripts
```

The ranking is a plain subsequence score: contiguous runs, word boundaries and
matches in the file name are rewarded, path length and skipped characters are
penalised. No index is built, so startup is proportional to the tree.

## Tests

```sh
cargo test
```

Unit tests cover ranking order, the empty query and preview clipping, plus the
non-panicking path for unreadable files.

## Demo

![fz demo](docs/demo.gif)

<!-- TODO: re-record the demo against the column-split layout once it lands. -->

## License

MIT, see [LICENSE](LICENSE).
