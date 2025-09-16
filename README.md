# cdrle

Calldata RLE — a simple, bounded run-length encoding specialized for Ethereum calldata.

- Literals: any byte != `0x00`
- Runs: `(0x00, CONTROL)` where `bit7=0→zeros`, `bit7=1→0xFF`, `low7=(len-1)`
- Caps: zeros `1..=128`, `0xFF` `1..=32`
- Encoder XOR-negates the first 4 bytes of the *compressed* stream to aid fallback routing; decoder un-negates on read.

## Usage

```rust
let input = b"\x00\x00\x00\x00*\xff\xff\xff\x01\x02\x03\x00\xff\x00\xaa\xbb\x00\xff";
let compressed = cdrle::compress(input);
let decompressed = cdrle::decompress(&compressed).unwrap();
assert_eq!(input, &decompressed[..]);
```

