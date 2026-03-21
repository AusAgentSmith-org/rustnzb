# rustnzb Performance Optimization Status

## Summary

Starting from benchmark run 020819, we identified and fixed several performance issues in rustnzb's post-processing pipeline. The work culminated in a standalone pure Rust PAR2 implementation (`rust_par2`) — the first Rust crate with full PAR2 repair support.

---

## Completed Items

### 1. Par2: single-pass repair instead of verify+repair
**Status:** Done
**Commit:** `32e23fc`

The pipeline was running `par2 verify` then `par2 repair` as separate processes. The repair command already verifies first, so this was a redundant double-scan causing 5-14x slowdown vs SABnzbd. Now runs repair directly when articles failed > 0.

### 2. Par2: multi-threaded operation (`-tN` flag)
**Status:** Done
**Commits:** `32e23fc`, `89c8f61`

par2cmdline-turbo was invoked without thread flags, defaulting to single-threaded. Now passes `-tN` (no space — par2cmdline-turbo syntax requires `-t4` not `-t 4`). The `-t <N>` (with space) format caused silent failures in runs 024621, 041656, 044858, 050534.

### 3. Skip par2 verify when zero article failures
**Status:** Done (superseded by native par2 verify)
**Commit:** `32e23fc`

When `articles_failed == 0`, par2 verification is skipped entirely (files are known-good). Eliminated 22-46s of wasted par2 verify time in unpack scenarios. Later superseded by native Rust par2 verify which always runs (it's cheap — no process spawn).

### 4. Capture rustnzb internal metrics in benchmark harness
**Status:** Done
**Commit:** `32e23fc`

Added `InternalMetrics` struct with `server_stats`, `stage_durations`, `download_throughput_mbps`, article counts. Pulled from `/api/history` after job completion. Added to CSV (3 new columns) and JSON output.

### 5. Decode timing instrumentation
**Status:** Done
**Commit:** `32e23fc`

Added `total_decode_us`, `total_assemble_us`, `total_articles_decoded` atomic counters to `DownloadEngine`. Workers accumulate via `fetch_add`. New log line after download phase shows cumulative decode/assemble time and percentage of wall time. Finding: yEnc decode consumes ~2.5 CPU cores of ~3.7 total (250% of wall time across threads).

### 6. Bundled par2cmdline-turbo binary (`par2-sys` crate)
**Status:** Done
**Commits:** Multiple (par2-sys crate creation through `2c32bcd`)

Created `crates/par2-sys/` that downloads the pre-built par2cmdline-turbo binary at build time and embeds it via `include_bytes!`. Extracts to `/tmp/par2-sys/par2-{version}` on first use. Falls back to source compilation when autotools are available (Dockerfile installs them). Eliminates runtime dependency on system `par2`.

### 7. Native Rust PAR2 parser and verifier (`crates/par2/`)
**Status:** Done
**Commit:** `52955d1`

Pure Rust PAR2 file parser (5 packet types) and MD5-based verification. Uses asm-accelerated MD5 + 2MB double-buffered I/O. Matches or beats par2cmdline-turbo verify performance at 5GB scale (7.4s vs 7.6s). Pipeline now always runs native verify first, only falls back to par2cmdline for actual repair.

### 8. Standalone full Rust PAR2 implementation (`/home/sprooty/rust_par2/`)
**Status:** Done — first Rust crate with PAR2 repair support

~4,000 lines of Rust implementing:
- **GF(2^16) arithmetic** (`gf.rs`) — log/antilog tables, polynomial 0x1100B (not available in any existing Rust crate)
- **SIMD GF multiply** (`gf_simd.rs`) — AVX2 VPSHUFB (32 bytes/instruction), SSSE3 fallback, scalar fallback
- **Matrix operations** (`matrix.rs`) — Vandermonde construction with PAR2 input constants, Gaussian elimination
- **PAR2 parser** (`packets.rs`) — Main, FileDesc, IFSC, RecoverySlice, Creator packets
- **Verification** (`verify.rs`) — parallel file hashing with rayon, per-block damage detection
- **Recovery block reader** (`recovery.rs`) — loads RecoverySlice packets from .vol files
- **Repair engine** (`repair.rs`) — parallel block reconstruction with SIMD multiply-accumulate

41 tests, byte-identical output confirmed against par2cmdline-turbo.

---

## Benchmark Results Across Runs

### Par2 Repair Performance Journey

| Run | Status | 5GB par2 total | 10GB par2 total | Notes |
|-----|--------|----------------|-----------------|-------|
| 020819 | Pre-fix baseline | 147.1s | 420.4s | Double-scan verify+repair |
| 021725 | Pre-fix (faster node) | 241.6s | 810.0s | Same bugs, different node |
| 024621 | `-t` flag crash | 5.2s* | 8.5s* | *Par2 failed silently (vanilla par2) |
| 041656 | Binary not found | 4.3s* | 8.1s* | *par2_bin_path pointed to build dir |
| 044858 | Binary not found | 5.1s* | 8.6s* | *Same issue, different build |
| 050534 | `-t N` syntax wrong | 4.3s* | 8.1s* | *`-t 20` invalid, needs `-t20` |
| 061858 | First working repair | 36.5s | 90.3s | Par2 working but single-threaded (no `-tN`) |
| 083205 | With `-tN` threading | 33.5s | 89.3s | Threading restored, 40% slower than SABnzbd |

\* Par2 repair failed — times reflect download-only

### Standalone rust_par2 Benchmarks (1GB, 3% damage)

| Version | Repair Time | vs par2cmdline-turbo | Speedup from baseline |
|---------|------------|---------------------|----------------------|
| Naive scalar | 92.8s | 21x slower | baseline |
| + SSSE3 PSHUFB | 6.9s | 1.6x slower | 13.4x |
| + AVX2 VPSHUFB | 6.7s | 1.5x slower | 13.9x |
| + Tiled source-major | 6.9s | 1.7x slower | no gain |
| + Flat consolidation | 8.2s | 1.9x slower | worse |
| **Final (AVX2 + rayon)** | **6.7s** | **1.5x slower** | **13.9x** |
| par2cmdline-turbo | 4.4s | baseline | — |

### Verify Performance (1GB)

| Implementation | Intact files | Damaged files |
|---------------|-------------|---------------|
| rust-par2 | 1.5s | 3.0s |
| par2cmdline-turbo | 1.5s | 4.4s |

Native Rust verify matches par2cmdline-turbo on intact files and is **35% faster** on damaged files.

---

## Architecture

### rustnzb Post-Processing Pipeline (current)

```
Download complete
    │
    ▼
┌─────────────────────────────────┐
│ Native PAR2 verify (pure Rust)  │  ← par2::verify()
│ asm MD5 + double-buffered I/O   │     ~1.5s for 1GB
└──────────┬──────────────────────┘
           │
     ┌─────┴──────┐
     │ All OK?    │
     ▼            ▼
   Skip      ┌────────────────────────┐
   repair    │ par2cmdline-turbo      │  ← par2-sys (embedded binary)
             │ repair -tN -B <path>   │     ~4s for 1GB
             └────────────────────────┘
                      │
                      ▼
              ┌───────────────┐
              │ Extract       │  7z / RAR / ZIP
              └───────────────┘
                      │
                      ▼
              ┌───────────────┐
              │ Cleanup       │  Remove par2/archive files
              └───────────────┘
```

### rust_par2 standalone crate

```
rust_par2/
├── src/
│   ├── gf.rs          GF(2^16) arithmetic (polynomial 0x1100B)
│   ├── gf_simd.rs     AVX2/SSSE3 VPSHUFB multiply-accumulate
│   ├── matrix.rs      Vandermonde matrix + Gaussian elimination
│   ├── packets.rs     PAR2 file format parser
│   ├── types.rs       Data structures
│   ├── verify.rs      Parallel file verification (rayon + asm MD5)
│   ├── recovery.rs    Recovery block reader from .vol files
│   └── repair.rs      Full repair engine (SIMD + rayon parallel)
├── examples/
│   ├── benchmark.rs   Benchmark vs par2cmdline-turbo
│   └── debug_repair.rs
└── tests/
    ├── fixtures/      Test sets: intact, damaged, missing, unrepairable
    └── integration.rs Golden tests vs par2cmdline-turbo
```

---

## Known Remaining Issues

### 1. Repair 1.5x slower than par2cmdline-turbo
The remaining gap is memory bandwidth, not compute. par2cmdline-turbo's ParPar backend uses region-based processing that keeps working sets in L1 cache. Our approach reads source data D times (once per damaged block). Closing this requires fundamentally different data flow architecture.

### 2. No PAR2 create support
rust_par2 can verify and repair but not create recovery files. Not needed for NZB clients (they only consume PAR2, never generate).

### 3. No AVX-512 support
Could add AVX-512 VPSHUFB (64 bytes/instruction) for another ~2x on supported CPUs. Low priority since few consumer CPUs benefit.

### 4. CPU usage higher than SABnzbd for downloads
rustnzb uses ~3-4x more CPU than SABnzbd during raw downloads. yEnc decode is ~2.5 CPU cores of the total. SIMD yEnc decoder (existing `yenc-simd` crate) could reduce this but is low priority since download speed is already 20-45% faster.

---

## File Change Summary

| File | Change |
|------|--------|
| `crates/nzb-postproc/src/pipeline.rs` | Native par2 verify + conditional par2cmdline repair |
| `crates/nzb-postproc/src/par2.rs` | Uses par2-sys bundled binary, `-tN` flags, `-B` basepath |
| `crates/par2-sys/` | Downloads/embeds par2cmdline-turbo at build time |
| `crates/par2/` | Native Rust PAR2 parser + verifier |
| `crates/nzb-web/src/download_engine.rs` | Decode timing instrumentation |
| `crates/nzb-web/src/queue_manager.rs` | Passes articles_failed to pipeline |
| `benchnzb/src/runner.rs` | Internal metrics capture |
| `benchnzb/src/clients/rustnzb.rs` | get_internal_metrics() from /api/history |
| `benchnzb/src/report.rs` | Extended CSV with internal metrics columns |
| `Dockerfile` | Autotools for par2-sys source compilation |
| `/home/sprooty/rust_par2/` | Standalone full PAR2 implementation |
