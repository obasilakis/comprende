# comprende

Compresses repetitive logs for LLM consumption. A simplified [Drain](https://jiemingzhu.github.io/pub/pjhe_icws2017.pdf) algorithm that identifies patterns, deduplicates them, and shows sample values.

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

**Output (2 lines):**
```
[9x] + 1744 ??? (in Live) load address 0x104fc4000 + <0> <1>
     <0>: 0x114df74, 0x115c9c0, 0x1e99770 | <1>: [0x106111f74], [0x1061209c0], [0x106e5d770]
```

## How It Works

1. **Tokenize** each line by whitespace
2. **Compute entropy** per column: `H = -Σ p(x) log₂ p(x)`
3. **Identify noise** - high entropy columns (addresses, timestamps, counters)
4. **Group lines** by template pattern
5. **Output** each pattern once with count and sample values

## Building

```bash
cargo build --release
```

## License

MIT
