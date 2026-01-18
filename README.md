# comprende

Compresses macOS `sample` output for LLM consumption. Feed it a 350KB stack trace, get back 15-50KB of clean, deduplicated output. No more "context window exceeded" when asking an LLM to debug your stuck process.

## Usage

```bash
sample MyApp 5 | comprende
# or
cat sample_output.txt | comprende
```

## Example

**Input** - 9 lines from `sample Ableton`:
```
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x114df74  [0x106111f74]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x115c9c0  [0x1061209c0]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x1e99770  [0x106e5d770]
...
```

**Output** - 1 line:
```
[9x] 1744 ???  (in Live)  load address <hex> + <hex>  <addr>
```

Hex addresses, UUIDs, thread IDs, and timestamps are normalized. System libraries in the Binary Images section are summarized into a single line count.

## Installation

Requires [Rust](https://rustup.rs/).

```bash
cargo build --release
cp target/release/comprende ~/.cargo/bin/
```

## Tip: Clipboard compression

Add to `.zshrc`:

```bash
alias clog='pbpaste | comprende | pbcopy && echo "Compressed"'
```

Copy sample output, run `clog`, paste into Claude/ChatGPT.

## License

MIT
