# comprende

Compresses repetitive logs for LLM consumption. Feed it a 350KB stack trace, get back 50KB of clean, deduplicated output that any LLM can parse. No more "context window exceeded" errors when debugging production issues.

## Usage

```bash
cat logs.txt | comprende
```

## Example

**Input (9 lines):**
```
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x114df74  [0x106111f74]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x115c9c0  [0x1061209c0]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x1e99770  [0x106e5d770]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x1d09b64  [0x106ccdb64]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x121a7e0  [0x1061de7e0]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x2c21e8  [0x1052861e8]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x29b458  [0x10525f458]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x123c3e8  [0x1062003e8]
+ 1744 ???  (in Live)  load address 0x104fc4000 + 0x29975c  [0x10525d75c]
```

**Output (1 line):**
```
[9x] 1744 ???  (in Live)  load address <hex> + <hex>  <addr>
```

## What Gets Normalized

- **Hex addresses**: `0x104fc4000` → `<hex>`
- **Bracketed addresses**: `[0x106111f74]` → `<addr>`
- **UUIDs**: `<4B0BCBB4-2271-376E-B5C3-CC18D418FC11>` → `<uuid>`
- **Thread IDs**: `Thread_4243153` → `Thread_<id>`
- **Timestamps**: `07:28:03` → `<time>`
- **Large numbers** (5+ digits): `54087` → `<num>`
- **Indentation/tree markers**: stripped for better grouping

## Binary Images (macOS crash/sample reports)

For macOS stack traces, comprende keeps app/plugin images but summarizes system libraries:

```
=== Binary Images ===
<hex> - <hex> com.xfer.serum2.VST3 (2.0.22) <uuid> /Library/Audio/Plug-Ins/VST3/Serum2.vst3/...
<hex> - <hex> com.xlnaudio.xo (1.7.5) <uuid> /Library/Audio/Plug-Ins/VST3/XO.vst3/...
[1076 system libraries omitted]
```

## Installation

### Prerequisites

Install Rust using the instructions at [rustup.rs](https://rustup.rs/), then restart your terminal or run `source "$HOME/.cargo/env"`.

### Building

```bash
cargo build --release
```

### Adding to PATH

```bash
cp target/release/comprende ~/.cargo/bin/
```

Or add the build directory to your PATH:

```bash
echo 'export PATH="$PATH:/path/to/compact_repetition/target/release"' >> ~/.zshrc
source ~/.zshrc
```

## Tips

### Shell alias for clipboard compression

Add to your `.zshrc` or `.bashrc`:

```bash
alias clog='pbpaste | comprende | pbcopy && echo "Compressed and copied"'
```

Then run `clog` after copying logs to compress them in your clipboard before pasting into an LLM.

## License

MIT
