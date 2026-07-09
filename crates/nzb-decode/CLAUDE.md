# nzb-decode

yEnc decoding, file assembly, and article cache for NZB download clients.

**Version:** 0.1.0 | **Edition:** Rust 2024 | **License:** MIT

## This Is a Shared Library

### Consumed By

| App | Via | Tag |
|-----|-----|-----|
| rustnzbd | git | v0.1.0 |
| Arz | git | v0.1.0 |
| NGMS | vendored (path) | — |

### Depends On

- **rust-yenc-simd** (crates.io, v0.1) — SIMD-accelerated yEnc decoder

## Public API

```rust
pub mod assembler;   // FileAssembler, AssemblerError
pub mod cache;       // ArticleCache, CacheKey, CacheStats, CacheError
pub mod yenc;        // YencDecodeResult, decode_yenc

pub use assembler::FileAssembler;
pub use cache::ArticleCache;
pub use yenc::{YencDecodeResult, decode_yenc};
```

### Key Types

- **`FileAssembler`** — writes decoded articles into output files with lock-free concurrent writes (pwrite). Tracks per-file progress with atomic bitmap.
- **`ArticleCache`** — bounded in-memory cache with disk spill to temp files. LRU eviction with stats (hits, misses, spills). Key: `(job_id, file_id, segment_number)`.
- **`decode_yenc(&[u8]) -> Result<YencDecodeResult>`** — decodes raw NNTP article data, returns decoded bytes + metadata (filename, part info, CRC32).

## Key Dependencies

- yenc-simd (SIMD yEnc decoding)
- crc32fast (checksums)
- tokio, parking_lot
